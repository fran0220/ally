use deadpool_redis::Pool as RedisPool;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{MySql, QueryBuilder};
use tokio::time::{Duration, sleep};
use waoowaoo_core::errors::AppError;

const IDLE_POLL_INTERVAL_MS: u64 = 500;
const RECEIVED_PROGRESS: i32 = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerTask {
    pub task_id: String,
    pub user_id: String,
    pub project_id: String,
    pub episode_id: Option<String>,
    pub task_type: String,
    pub target_type: String,
    pub target_id: String,
    pub attempt: i32,
    pub max_attempts: i32,
    pub payload: Value,
    pub billing_info: Option<Value>,
}

#[derive(Debug, sqlx::FromRow)]
struct TaskClaimRow {
    id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
    #[sqlx(rename = "projectId")]
    project_id: String,
    #[sqlx(rename = "episodeId")]
    episode_id: Option<String>,
    #[sqlx(rename = "type")]
    task_type: String,
    #[sqlx(rename = "targetType")]
    target_type: String,
    #[sqlx(rename = "targetId")]
    target_id: String,
    attempt: i32,
    #[sqlx(rename = "maxAttempts")]
    max_attempts: i32,
    payload: Option<sqlx::types::Json<Value>>,
    #[sqlx(rename = "billingInfo")]
    billing_info: Option<sqlx::types::Json<Value>>,
}

fn queue_task_types(queue: &str) -> &'static [&'static str] {
    match queue {
        "image" => &[
            "image_panel",
            "image_character",
            "image_location",
            "panel_variant",
            "modify_asset_image",
            "regenerate_group",
            "asset_hub_image",
            "asset_hub_modify",
        ],
        "video" => &["video_panel", "lip_sync"],
        "voice" => &["voice_line", "voice_design", "asset_hub_voice_design"],
        "text" => &[
            "analyze_novel",
            "analyze_global",
            "story_to_script_run",
            "script_to_storyboard_run",
            "clips_build",
            "screenplay_convert",
            "episode_split_llm",
            "voice_analyze",
            "ai_create_character",
            "ai_create_location",
            "ai_modify_appearance",
            "ai_modify_location",
            "ai_modify_shot_prompt",
            "analyze_shot_variants",
            "character_profile_confirm",
            "character_profile_batch_confirm",
            "reference_to_character",
            "asset_hub_reference_to_character",
            "asset_hub_ai_design_character",
            "asset_hub_ai_design_location",
            "asset_hub_ai_modify_character",
            "asset_hub_ai_modify_location",
            "regenerate_storyboard_text",
            "insert_panel",
        ],
        _ => &[],
    }
}

pub async fn consume_next(
    mysql: &sqlx::MySqlPool,
    _redis: &RedisPool,
    queue_name: &str,
) -> Result<Option<WorkerTask>, AppError> {
    let queue = queue_name.trim();
    let types = queue_task_types(queue);
    if types.is_empty() {
        return Err(AppError::invalid_params(format!("unknown queue: {queue}")));
    }

    let mut tx = mysql.begin().await?;

    let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
        "SELECT id, userId, projectId, episodeId, type, targetType, targetId, attempt, maxAttempts, payload, billingInfo FROM tasks WHERE status = 'queued' AND attempt < maxAttempts AND type IN (",
    );
    let mut separated = qb.separated(",");
    for task_type in types {
        separated.push_bind(task_type);
    }
    separated
        .push_unseparated(") ORDER BY priority DESC, createdAt ASC LIMIT 1 FOR UPDATE SKIP LOCKED");

    let row = qb
        .build_query_as::<TaskClaimRow>()
        .fetch_optional(&mut *tx)
        .await?;

    let Some(row) = row else {
        tx.rollback().await?;
        sleep(Duration::from_millis(IDLE_POLL_INTERVAL_MS)).await;
        return Ok(None);
    };

    sqlx::query(
        "UPDATE tasks SET status = 'processing', progress = GREATEST(progress, ?), startedAt = COALESCE(startedAt, NOW(3)), heartbeatAt = NOW(3), attempt = attempt + 1, updatedAt = NOW(3) WHERE id = ?",
    )
    .bind(RECEIVED_PROGRESS)
    .bind(&row.id)
    .execute(&mut *tx)
    .await?;

    let claimed_attempt = row.attempt.saturating_add(1).max(1);
    let max_attempts = row.max_attempts.max(1);

    let event_payload = json!({
      "status": "processing",
      "progress": RECEIVED_PROGRESS,
      "stage": "received",
      "queue": queue,
      "targetType": &row.target_type,
      "targetId": &row.target_id,
      "episodeId": &row.episode_id,
      "attempt": claimed_attempt,
      "maxAttempts": max_attempts,
    });

    sqlx::query(
        "INSERT INTO task_events (taskId, projectId, userId, eventType, payload, createdAt) VALUES (?, ?, ?, 'task.processing', ?, NOW(3))",
    )
    .bind(&row.id)
    .bind(&row.project_id)
    .bind(&row.user_id)
    .bind(sqlx::types::Json(event_payload))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Some(WorkerTask {
        task_id: row.id,
        user_id: row.user_id,
        project_id: row.project_id,
        episode_id: row.episode_id,
        task_type: row.task_type,
        target_type: row.target_type,
        target_id: row.target_id,
        attempt: claimed_attempt,
        max_attempts,
        payload: row.payload.map(|item| item.0).unwrap_or_else(|| json!({})),
        billing_info: row.billing_info.map(|item| item.0),
    }))
}
