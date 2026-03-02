use anyhow::{Result, anyhow};
use deadpool_redis::Pool as RedisPool;
use serde_json::{Value, json};
use sqlx::MySqlPool;
use tokio::time::{Duration, interval};
use tracing::{info, warn};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use waoowaoo_core::{
    config::AppConfig,
    db,
    runtime::publisher::{
        TaskLifecycleMessageInput, build_task_lifecycle_message, publish_task_message,
    },
};

const WATCHDOG_INTERVAL_SECS: u64 = 30;
const WATCHDOG_TIMEOUT_MINUTES: i64 = 10;
const WATCHDOG_BATCH_LIMIT: i64 = 100;
const WATCHDOG_ERROR_CODE: &str = "WATCHDOG_TIMEOUT";
const WATCHDOG_ERROR_MESSAGE: &str = "Task heartbeat timeout";

#[derive(Debug, Clone, sqlx::FromRow)]
struct TimedOutTask {
    id: String,
    #[sqlx(rename = "projectId")]
    project_id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
    #[sqlx(rename = "type")]
    task_type: String,
    #[sqlx(rename = "targetType")]
    target_type: String,
    #[sqlx(rename = "targetId")]
    target_id: String,
    #[sqlx(rename = "episodeId")]
    episode_id: Option<String>,
}

fn timeout_event_payload(task: &TimedOutTask) -> Value {
    json!({
        "status": "failed",
        "queue": "watchdog",
        "stage": "watchdog_timeout",
        "errorCode": WATCHDOG_ERROR_CODE,
        "errorMessage": WATCHDOG_ERROR_MESSAGE,
        "taskType": task.task_type,
        "targetType": task.target_type,
        "targetId": task.target_id,
        "episodeId": task.episode_id,
    })
}

async fn find_timed_out_tasks(mysql: &MySqlPool, limit: i64) -> Result<Vec<TimedOutTask>> {
    let rows = sqlx::query_as::<_, TimedOutTask>(
        "SELECT id, projectId, userId, type, targetType, targetId, episodeId
         FROM tasks
         WHERE status = 'processing'
           AND (
             heartbeatAt < DATE_SUB(NOW(3), INTERVAL ? MINUTE)
             OR (heartbeatAt IS NULL AND startedAt < DATE_SUB(NOW(3), INTERVAL ? MINUTE))
             OR (
               heartbeatAt IS NULL
               AND startedAt IS NULL
               AND updatedAt < DATE_SUB(NOW(3), INTERVAL ? MINUTE)
             )
           )
         ORDER BY updatedAt ASC
         LIMIT ?",
    )
    .bind(WATCHDOG_TIMEOUT_MINUTES)
    .bind(WATCHDOG_TIMEOUT_MINUTES)
    .bind(WATCHDOG_TIMEOUT_MINUTES)
    .bind(limit)
    .fetch_all(mysql)
    .await?;

    Ok(rows)
}

async fn fail_timed_out_task(
    mysql: &MySqlPool,
    redis: &RedisPool,
    task: &TimedOutTask,
) -> Result<bool> {
    let updated = sqlx::query(
        "UPDATE tasks
         SET status = 'failed',
             errorCode = ?,
             errorMessage = ?,
             dedupeKey = NULL,
             finishedAt = NOW(3),
             heartbeatAt = NULL,
             updatedAt = NOW(3)
         WHERE id = ?
           AND status = 'processing'",
    )
    .bind(WATCHDOG_ERROR_CODE)
    .bind(WATCHDOG_ERROR_MESSAGE)
    .bind(&task.id)
    .execute(mysql)
    .await?;

    if updated.rows_affected() == 0 {
        return Ok(false);
    }

    let payload = timeout_event_payload(task);
    let insert_result = sqlx::query(
        "INSERT INTO task_events (taskId, projectId, userId, eventType, payload, createdAt)
         VALUES (?, ?, ?, 'task.failed', ?, NOW(3))",
    )
    .bind(&task.id)
    .bind(&task.project_id)
    .bind(&task.user_id)
    .bind(sqlx::types::Json(payload.clone()))
    .execute(mysql)
    .await?;

    let event_id = i64::try_from(insert_result.last_insert_id())
        .map_err(|error| anyhow!("task event id overflow: {error}"))?;

    let message = build_task_lifecycle_message(TaskLifecycleMessageInput {
        id: event_id.to_string(),
        event_type: "task.failed",
        task_id: &task.id,
        project_id: &task.project_id,
        user_id: &task.user_id,
        task_type: &task.task_type,
        target_type: &task.target_type,
        target_id: &task.target_id,
        episode_id: task.episode_id.as_deref(),
        payload,
    });

    publish_task_message(redis, &task.project_id, &message).await?;

    Ok(true)
}

async fn sweep_timed_out_tasks(
    mysql: &MySqlPool,
    redis: &RedisPool,
    limit: i64,
) -> Result<Vec<String>> {
    let stale_tasks = find_timed_out_tasks(mysql, limit).await?;
    let mut timed_out_ids = Vec::with_capacity(stale_tasks.len());

    for task in stale_tasks {
        if fail_timed_out_task(mysql, redis, &task).await? {
            timed_out_ids.push(task.id);
        }
    }

    Ok(timed_out_ids)
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn")),
        )
        .with(fmt::layer().json())
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let config = AppConfig::load()?;
    let mysql = db::connect_mysql(&config.database_url).await?;
    let redis = db::connect_redis(&config.redis_url)?;

    let mut ticker = interval(Duration::from_secs(WATCHDOG_INTERVAL_SECS));
    info!("watchdog loop started");

    loop {
        ticker.tick().await;
        let timed_out_ids = sweep_timed_out_tasks(&mysql, &redis, WATCHDOG_BATCH_LIMIT).await?;

        if !timed_out_ids.is_empty() {
            warn!(
                count = timed_out_ids.len(),
                task_ids = ?timed_out_ids,
                "watchdog marked timed out tasks as failed"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TimedOutTask, timeout_event_payload};

    #[test]
    fn timeout_event_payload_contains_expected_fields() {
        let task = TimedOutTask {
            id: "task-1".to_string(),
            project_id: "project-1".to_string(),
            user_id: "user-1".to_string(),
            task_type: "story_to_script".to_string(),
            target_type: "episode".to_string(),
            target_id: "episode-1".to_string(),
            episode_id: Some("episode-1".to_string()),
        };

        let payload = timeout_event_payload(&task);
        assert_eq!(
            payload.get("errorCode").and_then(serde_json::Value::as_str),
            Some("WATCHDOG_TIMEOUT")
        );
        assert_eq!(
            payload
                .get("errorMessage")
                .and_then(serde_json::Value::as_str),
            Some("Task heartbeat timeout")
        );
        assert_eq!(
            payload.get("stage").and_then(serde_json::Value::as_str),
            Some("watchdog_timeout")
        );
        assert_eq!(
            payload
                .get("targetType")
                .and_then(serde_json::Value::as_str),
            Some("episode")
        );
        assert_eq!(
            payload.get("episodeId").and_then(serde_json::Value::as_str),
            Some("episode-1")
        );
    }
}
