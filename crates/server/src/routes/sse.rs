use std::convert::Infallible;
use std::time::Duration;

use axum::{
    extract::{Query, State},
    response::sse::{Event, KeepAlive, Sse},
};
use chrono::{NaiveDateTime, SecondsFormat, Utc};
use deadpool_redis::redis;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};
use tracing::{error, warn};
use waoowaoo_core::runtime::publisher::{project_run_channel, project_task_channel};

use crate::{app_state::AppState, error::AppError, extractors::auth::AuthUser};

const HEARTBEAT_INTERVAL_SECS: u64 = 15;
const REDIS_RECONNECT_DELAY_SECS: u64 = 1;

#[derive(Debug, Deserialize)]
pub struct SseQuery {
    #[serde(default, rename = "projectId")]
    project_id: Option<String>,
    #[serde(default, rename = "episodeId")]
    episode_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
struct TaskEventRow {
    id: i64,
    #[sqlx(rename = "taskId")]
    task_id: String,
    #[sqlx(rename = "projectId")]
    project_id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
    #[sqlx(rename = "eventType")]
    event_type: String,
    #[sqlx(rename = "episodeId")]
    episode_id: Option<String>,
    #[sqlx(rename = "type")]
    task_type: Option<String>,
    #[sqlx(rename = "targetType")]
    target_type: Option<String>,
    #[sqlx(rename = "targetId")]
    target_id: Option<String>,
    payload: Option<sqlx::types::Json<Value>>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
struct TaskSnapshotRow {
    id: String,
    #[sqlx(rename = "projectId")]
    project_id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
    #[sqlx(rename = "episodeId")]
    episode_id: Option<String>,
    #[sqlx(rename = "type")]
    task_type: String,
    #[sqlx(rename = "targetType")]
    target_type: String,
    #[sqlx(rename = "targetId")]
    target_id: String,
    status: String,
    progress: i32,
    payload: Option<sqlx::types::Json<Value>>,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, sqlx::FromRow)]
struct ProjectOwnerRow {
    #[sqlx(rename = "userId")]
    user_id: String,
}

fn parse_last_event_id(raw: Option<&str>) -> i64 {
    let Some(raw) = raw else {
        return 0;
    };
    raw.trim()
        .parse::<i64>()
        .ok()
        .filter(|v| *v > 0)
        .unwrap_or(0)
}

fn is_numeric_event_id(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit())
}

fn parse_pubsub_event_meta(raw: &str) -> (Option<String>, Option<String>) {
    let Ok(parsed) = serde_json::from_str::<Value>(raw) else {
        return (None, None);
    };
    let Some(obj) = parsed.as_object() else {
        return (None, None);
    };

    let event_name = obj
        .get("type")
        .and_then(Value::as_str)
        .map(std::borrow::ToOwned::to_owned);
    let event_id = if matches!(
        event_name.as_deref(),
        Some("task.lifecycle") | Some("task.stream")
    ) {
        obj.get("id")
            .and_then(Value::as_str)
            .filter(|value| is_numeric_event_id(value))
            .map(std::borrow::ToOwned::to_owned)
    } else {
        None
    };

    (event_name, event_id)
}

fn to_pubsub_sse_event(raw: &str) -> Event {
    let mut event = Event::default().data(raw);
    let (event_name, event_id) = parse_pubsub_event_meta(raw);

    if let Some(event_name) = event_name {
        event = event.event(event_name);
    }
    if let Some(event_id) = event_id {
        event = event.id(event_id);
    }

    event
}

fn heartbeat_event() -> Event {
    Event::default().event("heartbeat").data(
        json!({
            "ts": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
        })
        .to_string(),
    )
}

fn with_lifecycle_type(payload: Value, lifecycle_type: &str) -> Value {
    let mut payload = match payload {
        Value::Object(map) => map,
        _ => Map::new(),
    };

    let has_lifecycle_type = payload
        .get("lifecycleType")
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty());
    if !has_lifecycle_type {
        payload.insert(
            "lifecycleType".to_string(),
            Value::String(lifecycle_type.to_string()),
        );
    }

    Value::Object(payload)
}

fn open_redis_client(state: &AppState) -> Result<redis::Client, AppError> {
    redis::Client::open(state.config.redis_url.as_str())
        .map_err(|error| AppError::internal(format!("failed to create redis client: {error}")))
}

async fn verify_project_access(
    state: &AppState,
    project_id: &str,
    user_id: &str,
) -> Result<(), AppError> {
    if project_id == "global-asset-hub" {
        return Ok(());
    }

    let owner =
        sqlx::query_as::<_, ProjectOwnerRow>("SELECT userId FROM projects WHERE id = ? LIMIT 1")
            .bind(project_id)
            .fetch_optional(&state.mysql)
            .await?;

    let Some(owner) = owner else {
        return Err(AppError::not_found("project not found"));
    };
    if owner.user_id != user_id {
        return Err(AppError::forbidden("project access denied"));
    }
    Ok(())
}

fn to_sse_event(row: TaskEventRow) -> Event {
    let payload = row
        .payload
        .map(|value| value.0)
        .unwrap_or_else(|| json!({}));
    let payload = with_lifecycle_type(payload, &row.event_type);

    let body = json!({
      "id": row.id.to_string(),
      "type": "task.lifecycle",
      "taskId": row.task_id,
      "projectId": row.project_id,
      "userId": row.user_id,
      "eventType": row.event_type,
      "taskType": row.task_type,
      "targetType": row.target_type,
      "targetId": row.target_id,
      "episodeId": row.episode_id,
      "payload": payload,
      "ts": row.created_at.and_utc().to_rfc3339_opts(SecondsFormat::Millis, true),
    });

    Event::default()
        .id(row.id.to_string())
        .event("task.lifecycle")
        .data(body.to_string())
}

fn to_snapshot_event(row: TaskSnapshotRow) -> Event {
    let payload = row
        .payload
        .map(|value| value.0)
        .unwrap_or_else(|| json!({}));
    let lifecycle_type = if row.status == "queued" {
        "task.created"
    } else {
        "task.processing"
    };

    let body = json!({
      "id": format!("snapshot:{}:{}", row.id, row.updated_at.and_utc().timestamp_millis()),
      "type": "task.lifecycle",
      "taskId": row.id,
      "projectId": row.project_id,
      "userId": row.user_id,
      "taskType": row.task_type,
      "targetType": row.target_type,
      "targetId": row.target_id,
      "episodeId": row.episode_id,
      "eventType": lifecycle_type,
      "payload": {
        "progress": row.progress,
        "status": row.status,
        "lifecycleType": lifecycle_type,
        "snapshot": true,
        "sourcePayload": payload,
      },
      "ts": row.updated_at.and_utc().to_rfc3339_opts(SecondsFormat::Millis, true),
    });

    Event::default()
        .event("task.lifecycle")
        .data(body.to_string())
}

async fn read_replay_events(
    state: &AppState,
    project_id: &str,
    after_id: i64,
    limit: i64,
) -> Result<Vec<TaskEventRow>, AppError> {
    let rows = sqlx::query_as::<_, TaskEventRow>(
        "SELECT e.id, e.taskId, e.projectId, e.userId, e.eventType, t.episodeId, t.type, t.targetType, t.targetId, e.payload, e.createdAt FROM task_events e LEFT JOIN tasks t ON t.id = e.taskId WHERE e.projectId = ? AND e.id > ? ORDER BY e.id ASC LIMIT ?",
    )
    .bind(project_id)
    .bind(after_id)
    .bind(limit)
    .fetch_all(&state.mysql)
    .await?;
    Ok(rows)
}

async fn read_active_snapshot(
    state: &AppState,
    project_id: &str,
    user_id: &str,
    episode_id: Option<&str>,
) -> Result<Vec<TaskSnapshotRow>, AppError> {
    let mut query = String::from(
        "SELECT id, projectId, userId, episodeId, type, targetType, targetId, status, progress, payload, updatedAt FROM tasks WHERE projectId = ? AND userId = ? AND status IN ('queued', 'processing')",
    );
    if episode_id.is_some() {
        query.push_str(" AND episodeId = ?");
    }
    query.push_str(" ORDER BY updatedAt DESC LIMIT 500");

    let mut q = sqlx::query_as::<_, TaskSnapshotRow>(&query)
        .bind(project_id)
        .bind(user_id);

    if let Some(episode_id) = episode_id {
        q = q.bind(episode_id);
    }

    Ok(q.fetch_all(&state.mysql).await?)
}

pub async fn handler(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<SseQuery>,
    headers: axum::http::HeaderMap,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, AppError> {
    let project_id = query.project_id.unwrap_or_default().trim().to_string();
    if project_id.is_empty() {
        return Err(AppError::invalid_params("projectId is required"));
    }

    verify_project_access(&state, &project_id, &user.id).await?;

    let last_event_id = parse_last_event_id(
        headers
            .get("last-event-id")
            .and_then(|value| value.to_str().ok()),
    );

    let replay = if last_event_id > 0 {
        read_replay_events(&state, &project_id, last_event_id, 5000).await?
    } else {
        Vec::new()
    };

    let snapshot = if last_event_id == 0 {
        read_active_snapshot(&state, &project_id, &user.id, query.episode_id.as_deref()).await?
    } else {
        Vec::new()
    };

    let redis_client = open_redis_client(&state)?;
    let task_channel = project_task_channel(&project_id);
    let run_channel = project_run_channel(&project_id);

    let (tx, rx) = mpsc::channel::<Event>(256);

    let project_id_clone = project_id.clone();
    tokio::spawn(async move {
        for event in replay {
            if tx.send(to_sse_event(event)).await.is_err() {
                return;
            }
        }

        for item in snapshot {
            if tx.send(to_snapshot_event(item)).await.is_err() {
                return;
            }
        }

        // Send an immediate heartbeat so the client marks the connection as open
        // without waiting for the first interval tick.
        if tx.send(heartbeat_event()).await.is_err() {
            return;
        }

        let mut ticker = tokio::time::interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        // Consume the immediate first tick so heartbeat starts after interval.
        ticker.tick().await;

        loop {
            let mut pubsub = match redis_client.get_async_pubsub().await {
                Ok(pubsub) => pubsub,
                Err(err) => {
                    error!(project_id = project_id_clone, error = %err, "failed to create redis pubsub, retrying");
                    if tx.send(heartbeat_event()).await.is_err() {
                        return;
                    }
                    tokio::time::sleep(Duration::from_secs(REDIS_RECONNECT_DELAY_SECS)).await;
                    continue;
                }
            };

            if let Err(err) = pubsub.subscribe(&task_channel).await {
                error!(project_id = project_id_clone, error = %err, "failed to subscribe task event channel, retrying");
                if tx.send(heartbeat_event()).await.is_err() {
                    return;
                }
                tokio::time::sleep(Duration::from_secs(REDIS_RECONNECT_DELAY_SECS)).await;
                continue;
            }
            if let Err(err) = pubsub.subscribe(&run_channel).await {
                error!(project_id = project_id_clone, error = %err, "failed to subscribe run event channel, retrying");
                if tx.send(heartbeat_event()).await.is_err() {
                    return;
                }
                tokio::time::sleep(Duration::from_secs(REDIS_RECONNECT_DELAY_SECS)).await;
                continue;
            }

            let mut message_stream = pubsub.on_message();
            loop {
                tokio::select! {
                    maybe_message = message_stream.next() => {
                        let Some(message) = maybe_message else {
                            warn!(project_id = project_id_clone, "redis pubsub stream closed, reconnecting");
                            break;
                        };

                        match message.get_payload::<String>() {
                            Ok(payload) => {
                                if tx.send(to_pubsub_sse_event(&payload)).await.is_err() {
                                    return;
                                }
                            }
                            Err(err) => {
                                warn!(project_id = project_id_clone, error = %err, "failed to decode redis pubsub payload");
                            }
                        }
                    }
                    _ = ticker.tick() => {
                        if tx.send(heartbeat_event()).await.is_err() {
                            return;
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(REDIS_RECONNECT_DELAY_SECS)).await;
        }
    });

    let stream = ReceiverStream::new(rx).map(Ok::<Event, Infallible>);

    Ok(Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS))))
}

pub fn router() -> axum::Router<AppState> {
    axum::Router::new().route("/api/sse", axum::routing::get(handler))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{SseQuery, parse_last_event_id, parse_pubsub_event_meta, with_lifecycle_type};

    #[test]
    fn sse_query_accepts_camel_case_project_id() {
        let parsed: SseQuery = serde_json::from_value(json!({
            "projectId": "project-1",
            "episodeId": "episode-9"
        }))
        .expect("camelCase query payload should deserialize");

        assert_eq!(parsed.project_id.as_deref(), Some("project-1"));
        assert_eq!(parsed.episode_id.as_deref(), Some("episode-9"));
    }

    #[test]
    fn sse_query_ignores_legacy_snake_case_alias() {
        let parsed: SseQuery = serde_json::from_value(json!({
            "project_id": "legacy-project",
            "episode_id": "legacy-episode"
        }))
        .expect("unknown query keys should be ignored");

        assert_eq!(parsed.project_id, None);
        assert_eq!(parsed.episode_id, None);
    }

    #[test]
    fn parse_pubsub_event_meta_extracts_lifecycle_event() {
        let raw = r#"{"id":"123","type":"task.lifecycle","taskId":"task-1"}"#;
        let (event_name, event_id) = parse_pubsub_event_meta(raw);

        assert_eq!(event_name.as_deref(), Some("task.lifecycle"));
        assert_eq!(event_id.as_deref(), Some("123"));
    }

    #[test]
    fn parse_pubsub_event_meta_ignores_non_numeric_event_id() {
        let raw = r#"{"id":"snapshot:abc","type":"task.stream"}"#;
        let (event_name, event_id) = parse_pubsub_event_meta(raw);

        assert_eq!(event_name.as_deref(), Some("task.stream"));
        assert_eq!(event_id, None);
    }

    #[test]
    fn parse_pubsub_event_meta_handles_invalid_json() {
        let (event_name, event_id) = parse_pubsub_event_meta("not-json");

        assert_eq!(event_name, None);
        assert_eq!(event_id, None);
    }

    #[test]
    fn parse_pubsub_event_meta_does_not_emit_run_event_cursor() {
        let raw = r#"{"id":"456","type":"run.event","runId":"run-1"}"#;
        let (event_name, event_id) = parse_pubsub_event_meta(raw);

        assert_eq!(event_name.as_deref(), Some("run.event"));
        assert_eq!(event_id, None);
    }

    #[test]
    fn parse_last_event_id_rejects_invalid_values() {
        assert_eq!(parse_last_event_id(Some("0")), 0);
        assert_eq!(parse_last_event_id(Some("-9")), 0);
        assert_eq!(parse_last_event_id(Some("abc")), 0);
        assert_eq!(parse_last_event_id(Some(" 101 ")), 101);
        assert_eq!(parse_last_event_id(None), 0);
    }

    #[test]
    fn with_lifecycle_type_fills_missing_value() {
        let payload = with_lifecycle_type(json!({ "status": "failed" }), "task.failed");

        assert_eq!(
            payload
                .get("lifecycleType")
                .and_then(serde_json::Value::as_str),
            Some("task.failed")
        );
    }

    #[test]
    fn with_lifecycle_type_keeps_existing_value() {
        let payload =
            with_lifecycle_type(json!({ "lifecycleType": "task.processing" }), "task.failed");

        assert_eq!(
            payload
                .get("lifecycleType")
                .and_then(serde_json::Value::as_str),
            Some("task.processing")
        );
    }
}
