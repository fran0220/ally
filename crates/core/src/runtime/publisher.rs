use chrono::{SecondsFormat, Utc};
use deadpool_redis::Pool as RedisPool;
use serde_json::{Map, Value, json};
use sqlx::MySqlPool;

use crate::errors::AppError;

use super::{service::append_run_event_with_seq, types::RunEventInput};

const RUN_CHANNEL_PREFIX: &str = "run-events:project:";
const TASK_CHANNEL_PREFIX: &str = "task-events:project:";

pub struct TaskLifecycleMessageInput<'a> {
    pub id: String,
    pub event_type: &'a str,
    pub task_id: &'a str,
    pub project_id: &'a str,
    pub user_id: &'a str,
    pub task_type: &'a str,
    pub target_type: &'a str,
    pub target_id: &'a str,
    pub episode_id: Option<&'a str>,
    pub payload: Value,
}

fn should_fill_payload_field(payload: &Map<String, Value>, key: &str) -> bool {
    match payload.get(key) {
        None => true,
        Some(Value::Null) => true,
        Some(Value::String(value)) => value.trim().is_empty(),
        _ => false,
    }
}

fn now_iso_string() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

async fn publish_message(
    redis_pool: &RedisPool,
    channel: String,
    payload: String,
    event_kind: &str,
) -> Result<(), AppError> {
    let mut connection = redis_pool.get().await.map_err(|error| {
        AppError::internal(format!("failed to borrow redis connection: {error}"))
    })?;

    redis::cmd("PUBLISH")
        .arg(channel)
        .arg(payload)
        .query_async::<i64>(&mut connection)
        .await
        .map_err(|error| {
            AppError::internal(format!("failed to publish {event_kind} event: {error}"))
        })?;

    Ok(())
}

pub fn project_run_channel(project_id: &str) -> String {
    format!("{RUN_CHANNEL_PREFIX}{project_id}")
}

pub fn project_task_channel(project_id: &str) -> String {
    format!("{TASK_CHANNEL_PREFIX}{project_id}")
}

pub fn build_task_lifecycle_message(input: TaskLifecycleMessageInput<'_>) -> Value {
    let mut payload = input.payload.as_object().cloned().unwrap_or_default();
    if should_fill_payload_field(&payload, "lifecycleType") {
        payload.insert(
            "lifecycleType".to_string(),
            Value::String(input.event_type.to_string()),
        );
    }

    json!({
      "id": input.id,
      "type": "task.lifecycle",
      "eventType": input.event_type,
      "taskId": input.task_id,
      "projectId": input.project_id,
      "userId": input.user_id,
      "taskType": input.task_type,
      "targetType": input.target_type,
      "targetId": input.target_id,
      "episodeId": input.episode_id,
      "payload": payload,
      "ts": now_iso_string(),
    })
}

pub async fn publish_task_message(
    redis_pool: &RedisPool,
    project_id: &str,
    message: &Value,
) -> Result<(), AppError> {
    publish_message(
        redis_pool,
        project_task_channel(project_id),
        message.to_string(),
        "task",
    )
    .await
}

pub async fn publish_run_event(
    mysql: &MySqlPool,
    redis_pool: &RedisPool,
    input: &RunEventInput,
) -> Result<Value, AppError> {
    let event = append_run_event_with_seq(mysql, input).await?;
    let message = json!({
      "id": event.id,
      "type": "run.event",
      "runId": event.run_id,
      "projectId": event.project_id,
      "userId": event.user_id,
      "seq": event.seq,
      "eventType": event.event_type.as_str(),
      "stepKey": event.step_key,
      "attempt": event.attempt,
      "lane": event.lane.map(|lane| lane.as_str()),
      "payload": event.payload,
      "ts": event.created_at,
    });

    publish_message(
        redis_pool,
        project_run_channel(&event.project_id),
        message.to_string(),
        "run",
    )
    .await?;

    Ok(message)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{TaskLifecycleMessageInput, build_task_lifecycle_message, project_task_channel};

    #[test]
    fn project_task_channel_uses_expected_prefix() {
        assert_eq!(
            project_task_channel("project-1"),
            "task-events:project:project-1"
        );
    }

    #[test]
    fn build_task_lifecycle_message_fills_missing_lifecycle_type() {
        let message = build_task_lifecycle_message(TaskLifecycleMessageInput {
            id: "7".to_string(),
            event_type: "task.failed",
            task_id: "task-1",
            project_id: "project-1",
            user_id: "user-1",
            task_type: "story_to_script",
            target_type: "episode",
            target_id: "episode-1",
            episode_id: Some("episode-1"),
            payload: json!({
                "status": "failed",
            }),
        });

        assert_eq!(
            message
                .get("payload")
                .and_then(|value| value.get("lifecycleType"))
                .and_then(serde_json::Value::as_str),
            Some("task.failed")
        );
    }

    #[test]
    fn build_task_lifecycle_message_keeps_existing_lifecycle_type() {
        let message = build_task_lifecycle_message(TaskLifecycleMessageInput {
            id: "8".to_string(),
            event_type: "task.failed",
            task_id: "task-2",
            project_id: "project-2",
            user_id: "user-2",
            task_type: "story_to_script",
            target_type: "episode",
            target_id: "episode-2",
            episode_id: None,
            payload: json!({
                "lifecycleType": "task.processing",
            }),
        });

        assert_eq!(
            message
                .get("payload")
                .and_then(|value| value.get("lifecycleType"))
                .and_then(serde_json::Value::as_str),
            Some("task.processing")
        );
    }
}
