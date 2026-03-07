use anyhow::{Result, anyhow};
use chrono::{Duration as ChronoDuration, Utc};
use deadpool_redis::Pool as RedisPool;
use serde_json::{Value, json};
use sqlx::MySqlPool;
use std::env;
use tokio::time::{Duration, interval};
use tracing::{error, info, warn};
use waoowaoo_core::runtime::publisher::{
    TaskLifecycleMessageInput, build_task_lifecycle_message, publish_task_message,
};

const DEFAULT_WATCHDOG_INTERVAL_MS: u64 = 30_000;
const DEFAULT_HEARTBEAT_TIMEOUT_MS: i64 = 90_000;
const DEFAULT_BATCH_LIMIT: i64 = 100;

const WATCHDOG_TIMEOUT_CODE: &str = "WATCHDOG_TIMEOUT";
const WATCHDOG_TIMEOUT_MESSAGE: &str = "Task heartbeat timeout";
const TASK_LOCALE_REQUIRED_CODE: &str = "TASK_LOCALE_REQUIRED";
const TASK_LOCALE_REQUIRED_MESSAGE: &str = "task locale is missing";

#[derive(Debug, Clone, sqlx::FromRow)]
struct LifecycleTask {
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

#[derive(Debug, Clone, sqlx::FromRow)]
struct QueuedTask {
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
    payload: Option<sqlx::types::Json<Value>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ProcessingTask {
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
    attempt: i32,
    #[sqlx(rename = "maxAttempts")]
    max_attempts: i32,
}

fn read_u64_env(key: &str, default: u64, min: u64) -> u64 {
    env::var(key)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|value| *value >= min)
        .unwrap_or(default)
}

fn read_i64_env(key: &str, default: i64, min: i64) -> i64 {
    env::var(key)
        .ok()
        .and_then(|raw| raw.trim().parse::<i64>().ok())
        .filter(|value| *value >= min)
        .unwrap_or(default)
}

fn normalize_locale_candidate(raw: &str) -> Option<&'static str> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    if normalized == "zh" || normalized.starts_with("zh-") {
        return Some("zh");
    }
    if normalized == "en" || normalized.starts_with("en-") {
        return Some("en");
    }
    None
}

fn read_task_locale(payload: &Value) -> Option<&'static str> {
    let from_meta = payload
        .get("meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get("locale"))
        .and_then(Value::as_str)
        .and_then(normalize_locale_candidate);
    if from_meta.is_some() {
        return from_meta;
    }
    payload
        .get("locale")
        .and_then(Value::as_str)
        .and_then(normalize_locale_candidate)
}

fn timeout_failed_payload(task: &LifecycleTask) -> Value {
    json!({
      "status": "failed",
      "queue": "watchdog",
      "stage": "watchdog_timeout",
      "reason": "watchdog_timeout",
      "errorCode": WATCHDOG_TIMEOUT_CODE,
      "errorMessage": WATCHDOG_TIMEOUT_MESSAGE,
      "taskType": task.task_type,
      "targetType": task.target_type,
      "targetId": task.target_id,
      "episodeId": task.episode_id,
    })
}

fn locale_missing_payload(task: &LifecycleTask) -> Value {
    json!({
      "status": "failed",
      "queue": "watchdog",
      "stage": "watchdog_reenqueue",
      "reason": "task_locale_required",
      "errorCode": TASK_LOCALE_REQUIRED_CODE,
      "errorMessage": TASK_LOCALE_REQUIRED_MESSAGE,
      "taskType": task.task_type,
      "targetType": task.target_type,
      "targetId": task.target_id,
      "episodeId": task.episode_id,
    })
}

fn watchdog_requeue_payload(task: &ProcessingTask) -> Value {
    json!({
      "status": "queued",
      "queue": "watchdog",
      "stage": "watchdog_requeue",
      "reason": "watchdog_requeue",
      "taskType": task.task_type,
      "targetType": task.target_type,
      "targetId": task.target_id,
      "episodeId": task.episode_id,
      "attempt": task.attempt,
      "maxAttempts": task.max_attempts,
    })
}

async fn insert_and_publish_lifecycle_event(
    mysql: &MySqlPool,
    redis: &RedisPool,
    task: &LifecycleTask,
    event_type: &str,
    payload: Value,
) -> Result<()> {
    let insert_result = sqlx::query(
        "INSERT INTO task_events (taskId, projectId, userId, eventType, payload, createdAt)
         VALUES (?, ?, ?, ?, ?, NOW(3))",
    )
    .bind(&task.id)
    .bind(&task.project_id)
    .bind(&task.user_id)
    .bind(event_type)
    .bind(sqlx::types::Json(payload.clone()))
    .execute(mysql)
    .await?;

    let event_id = i64::try_from(insert_result.last_insert_id())
        .map_err(|error| anyhow!("task event id overflow: {error}"))?;

    let message = build_task_lifecycle_message(TaskLifecycleMessageInput {
        id: event_id.to_string(),
        event_type,
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
    Ok(())
}

async fn find_queued_without_enqueue(mysql: &MySqlPool, limit: i64) -> Result<Vec<QueuedTask>> {
    let rows = sqlx::query_as::<_, QueuedTask>(
        "SELECT id, projectId, userId, type, targetType, targetId, episodeId, payload
         FROM tasks
         WHERE status = 'queued' AND enqueuedAt IS NULL
         ORDER BY createdAt ASC
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(mysql)
    .await?;
    Ok(rows)
}

async fn mark_task_failed(
    mysql: &MySqlPool,
    redis: &RedisPool,
    task: &LifecycleTask,
    code: &str,
    message: &str,
    payload: Value,
    expected_status: &str,
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
           AND status = ?",
    )
    .bind(code)
    .bind(message)
    .bind(&task.id)
    .bind(expected_status)
    .execute(mysql)
    .await?;
    if updated.rows_affected() == 0 {
        return Ok(false);
    }

    insert_and_publish_lifecycle_event(mysql, redis, task, "task.failed", payload).await?;
    Ok(true)
}

async fn recover_queued_tasks(mysql: &MySqlPool, redis: &RedisPool, limit: i64) -> Result<usize> {
    let tasks = find_queued_without_enqueue(mysql, limit).await?;
    let mut recovered = 0usize;

    for task in tasks {
        let lifecycle_task = LifecycleTask {
            id: task.id.clone(),
            project_id: task.project_id.clone(),
            user_id: task.user_id.clone(),
            task_type: task.task_type.clone(),
            target_type: task.target_type.clone(),
            target_id: task.target_id.clone(),
            episode_id: task.episode_id.clone(),
        };

        let payload = task
            .payload
            .as_ref()
            .map(|item| item.0.clone())
            .unwrap_or_else(|| json!({}));
        if read_task_locale(&payload).is_none() {
            if mark_task_failed(
                mysql,
                redis,
                &lifecycle_task,
                TASK_LOCALE_REQUIRED_CODE,
                TASK_LOCALE_REQUIRED_MESSAGE,
                locale_missing_payload(&lifecycle_task),
                "queued",
            )
            .await?
            {
                error!(
                    task_id = %task.id,
                    project_id = %task.project_id,
                    "watchdog failed queued task because locale is missing"
                );
            }
            continue;
        }

        let updated = sqlx::query(
            "UPDATE tasks
             SET enqueuedAt = NOW(3),
                 enqueueAttempts = enqueueAttempts + 1,
                 lastEnqueueError = NULL,
                 updatedAt = NOW(3)
             WHERE id = ?
               AND status = 'queued'
               AND enqueuedAt IS NULL",
        )
        .bind(&task.id)
        .execute(mysql)
        .await?;

        if updated.rows_affected() > 0 {
            recovered += 1;
        }
    }

    Ok(recovered)
}

async fn find_stalled_processing_tasks(
    mysql: &MySqlPool,
    timeout_ms: i64,
    limit: i64,
) -> Result<Vec<ProcessingTask>> {
    let cutoff = (Utc::now() - ChronoDuration::milliseconds(timeout_ms)).naive_utc();
    let rows = sqlx::query_as::<_, ProcessingTask>(
        "SELECT id, projectId, userId, type, targetType, targetId, episodeId, attempt, maxAttempts
         FROM tasks
         WHERE status = 'processing'
           AND (
             heartbeatAt < ?
             OR (heartbeatAt IS NULL AND startedAt < ?)
             OR (heartbeatAt IS NULL AND startedAt IS NULL AND updatedAt < ?)
           )
         ORDER BY updatedAt ASC
         LIMIT ?",
    )
    .bind(cutoff)
    .bind(cutoff)
    .bind(cutoff)
    .bind(limit)
    .fetch_all(mysql)
    .await?;
    Ok(rows)
}

async fn cleanup_stalled_processing_tasks(
    mysql: &MySqlPool,
    redis: &RedisPool,
    timeout_ms: i64,
    limit: i64,
) -> Result<(usize, usize)> {
    let tasks = find_stalled_processing_tasks(mysql, timeout_ms, limit).await?;
    let mut failed = 0usize;
    let mut requeued = 0usize;

    for task in tasks {
        let lifecycle_task = LifecycleTask {
            id: task.id.clone(),
            project_id: task.project_id.clone(),
            user_id: task.user_id.clone(),
            task_type: task.task_type.clone(),
            target_type: task.target_type.clone(),
            target_id: task.target_id.clone(),
            episode_id: task.episode_id.clone(),
        };

        let max_attempts = task.max_attempts.max(1);
        if task.attempt >= max_attempts {
            if mark_task_failed(
                mysql,
                redis,
                &lifecycle_task,
                WATCHDOG_TIMEOUT_CODE,
                WATCHDOG_TIMEOUT_MESSAGE,
                timeout_failed_payload(&lifecycle_task),
                "processing",
            )
            .await?
            {
                failed += 1;
            }
            continue;
        }

        let updated = sqlx::query(
            "UPDATE tasks
             SET status = 'queued',
                 enqueuedAt = NULL,
                 heartbeatAt = NULL,
                 startedAt = NULL,
                 errorCode = NULL,
                 errorMessage = NULL,
                 updatedAt = NOW(3)
             WHERE id = ?
               AND status = 'processing'",
        )
        .bind(&task.id)
        .execute(mysql)
        .await?;
        if updated.rows_affected() == 0 {
            continue;
        }

        insert_and_publish_lifecycle_event(
            mysql,
            redis,
            &lifecycle_task,
            "task.created",
            watchdog_requeue_payload(&task),
        )
        .await?;
        requeued += 1;
    }

    Ok((failed, requeued))
}

pub async fn run_watchdog(mysql: MySqlPool, redis: RedisPool) -> Result<()> {
    let interval_ms = read_u64_env("WATCHDOG_INTERVAL_MS", DEFAULT_WATCHDOG_INTERVAL_MS, 1_000);
    let timeout_ms = read_i64_env(
        "TASK_HEARTBEAT_TIMEOUT_MS",
        DEFAULT_HEARTBEAT_TIMEOUT_MS,
        1_000,
    );
    let batch_limit = read_i64_env("WATCHDOG_BATCH_LIMIT", DEFAULT_BATCH_LIMIT, 1);

    let mut ticker = interval(Duration::from_millis(interval_ms));
    info!(
        interval_ms,
        timeout_ms, batch_limit, "watchdog loop started"
    );

    loop {
        ticker.tick().await;

        let recovered = recover_queued_tasks(&mysql, &redis, batch_limit).await?;
        let (failed, requeued) =
            cleanup_stalled_processing_tasks(&mysql, &redis, timeout_ms, batch_limit).await?;

        if recovered > 0 || failed > 0 || requeued > 0 {
            warn!(
                recovered,
                failed, requeued, "watchdog sweep applied task state changes"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        LifecycleTask, TASK_LOCALE_REQUIRED_CODE, WATCHDOG_TIMEOUT_CODE, locale_missing_payload,
        normalize_locale_candidate, read_task_locale, timeout_failed_payload,
    };

    #[test]
    fn normalize_locale_candidate_accepts_zh_and_en() {
        assert_eq!(normalize_locale_candidate("zh-CN"), Some("zh"));
        assert_eq!(normalize_locale_candidate("en-US"), Some("en"));
        assert_eq!(normalize_locale_candidate("fr-FR"), None);
    }

    #[test]
    fn read_task_locale_prefers_meta_locale() {
        let payload = json!({
          "locale": "en-US",
          "meta": {
            "locale": "zh-CN"
          }
        });
        assert_eq!(read_task_locale(&payload), Some("zh"));
    }

    #[test]
    fn timeout_payload_contains_expected_fields() {
        let task = LifecycleTask {
            id: "task-1".to_string(),
            project_id: "project-1".to_string(),
            user_id: "user-1".to_string(),
            task_type: "story_to_script".to_string(),
            target_type: "episode".to_string(),
            target_id: "episode-1".to_string(),
            episode_id: Some("episode-1".to_string()),
        };

        let payload = timeout_failed_payload(&task);
        assert_eq!(
            payload.get("errorCode").and_then(serde_json::Value::as_str),
            Some(WATCHDOG_TIMEOUT_CODE)
        );
        assert_eq!(
            payload.get("reason").and_then(serde_json::Value::as_str),
            Some("watchdog_timeout")
        );
    }

    #[test]
    fn locale_missing_payload_contains_expected_fields() {
        let task = LifecycleTask {
            id: "task-2".to_string(),
            project_id: "project-2".to_string(),
            user_id: "user-2".to_string(),
            task_type: "image_character".to_string(),
            target_type: "character".to_string(),
            target_id: "character-1".to_string(),
            episode_id: None,
        };

        let payload = locale_missing_payload(&task);
        assert_eq!(
            payload.get("errorCode").and_then(serde_json::Value::as_str),
            Some(TASK_LOCALE_REQUIRED_CODE)
        );
        assert_eq!(
            payload.get("reason").and_then(serde_json::Value::as_str),
            Some("task_locale_required")
        );
    }
}
