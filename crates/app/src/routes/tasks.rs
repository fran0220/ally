use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{MySql, QueryBuilder};
use waoowaoo_core::runtime::publisher::{
    TaskLifecycleMessageInput, build_task_lifecycle_message, publish_task_message,
};

use crate::{app_state::AppState, error::AppError, extractors::auth::AuthUser};

const TASK_CANCELLED_CODE: &str = "TASK_CANCELLED";
const TASK_CANCELLED_MESSAGE: &str = "Task cancelled by user";

#[derive(Debug, Deserialize)]
pub struct TaskQuery {
    #[serde(default, rename = "includeEvents")]
    include_events: Option<String>,
    #[serde(default, rename = "eventsLimit")]
    events_limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    #[serde(default, rename = "projectId")]
    project_id: Option<String>,
    #[serde(default, rename = "targetType")]
    target_type: Option<String>,
    #[serde(default, rename = "targetId")]
    target_id: Option<String>,
    #[serde(default)]
    status: Vec<String>,
    #[serde(default, rename = "type")]
    task_types: Vec<String>,
    #[serde(default)]
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct DismissTasksRequest {
    #[serde(rename = "taskIds")]
    task_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct TaskTargetStatesRequest {
    #[serde(rename = "projectId")]
    project_id: String,
    targets: Vec<TargetStateRequest>,
}

#[derive(Debug, Deserialize)]
pub struct TargetStateRequest {
    #[serde(rename = "targetType")]
    target_type: String,
    #[serde(rename = "targetId")]
    target_id: String,
    #[serde(default)]
    types: Option<Vec<String>>,
}

#[derive(Debug, sqlx::FromRow)]
struct TaskRow {
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
    status: String,
    progress: i32,
    payload: Option<sqlx::types::Json<Value>>,
    result: Option<sqlx::types::Json<Value>>,
    #[sqlx(rename = "errorCode")]
    error_code: Option<String>,
    #[sqlx(rename = "errorMessage")]
    error_message: Option<String>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct TaskEventRow {
    id: i64,
    #[sqlx(rename = "taskId")]
    task_id: String,
    #[sqlx(rename = "projectId")]
    project_id: String,
    #[sqlx(rename = "eventType")]
    event_type: String,
    payload: Option<sqlx::types::Json<Value>>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
}

#[derive(Debug, sqlx::FromRow)]
struct ProjectOwnerRow {
    #[sqlx(rename = "userId")]
    user_id: String,
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

fn task_to_json(task: TaskRow) -> Value {
    json!({
      "id": task.id,
      "userId": task.user_id,
      "projectId": task.project_id,
      "episodeId": task.episode_id,
      "type": task.task_type,
      "targetType": task.target_type,
      "targetId": task.target_id,
      "status": task.status,
      "progress": task.progress,
      "payload": task.payload.map(|item| item.0),
      "result": task.result.map(|item| item.0),
      "errorCode": task.error_code,
      "errorMessage": task.error_message,
      "createdAt": task.created_at,
      "updatedAt": task.updated_at,
    })
}

fn task_cancel_payload(task: &TaskRow) -> Value {
    json!({
      "status": "failed",
      "stage": "cancelled",
      "errorCode": TASK_CANCELLED_CODE,
      "errorMessage": TASK_CANCELLED_MESSAGE,
      "cancelled": true,
      "targetType": task.target_type,
      "targetId": task.target_id,
      "episodeId": task.episode_id,
    })
}

async fn persist_and_publish_task_cancelled(
    state: &AppState,
    task: &TaskRow,
    payload: Value,
) -> Result<(), AppError> {
    let insert_result = sqlx::query(
        "INSERT INTO task_events (taskId, projectId, userId, eventType, payload, createdAt) VALUES (?, ?, ?, 'task.failed', ?, NOW(3))",
    )
    .bind(&task.id)
    .bind(&task.project_id)
    .bind(&task.user_id)
    .bind(sqlx::types::Json(payload.clone()))
    .execute(&state.mysql)
    .await?;

    let event_id = i64::try_from(insert_result.last_insert_id())
        .map_err(|error| AppError::internal(format!("task event id overflow: {error}")))?;

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

    publish_task_message(&state.redis, &task.project_id, &message)
        .await
        .map_err(AppError::from)
}

pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ListTasksQuery>,
) -> Result<Json<Value>, AppError> {
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
        "SELECT id, userId, projectId, episodeId, type, targetType, targetId, status, progress, payload, result, errorCode, errorMessage, createdAt, updatedAt FROM tasks WHERE userId = ",
    );
    qb.push_bind(&user.id);

    if let Some(project_id) = query.project_id {
        let project_id = project_id.trim().to_string();
        if !project_id.is_empty() {
            verify_project_access(&state, &project_id, &user.id).await?;
            qb.push(" AND projectId = ");
            qb.push_bind(project_id);
        }
    }

    if let Some(target_type) = query.target_type {
        let target_type = target_type.trim().to_string();
        if !target_type.is_empty() {
            qb.push(" AND targetType = ");
            qb.push_bind(target_type);
        }
    }

    if let Some(target_id) = query.target_id {
        let target_id = target_id.trim().to_string();
        if !target_id.is_empty() {
            qb.push(" AND targetId = ");
            qb.push_bind(target_id);
        }
    }

    let statuses = query
        .status
        .into_iter()
        .map(|item| item.trim().to_lowercase())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    if !statuses.is_empty() {
        qb.push(" AND status IN (");
        let mut separated = qb.separated(",");
        for status in statuses {
            separated.push_bind(status);
        }
        separated.push_unseparated(")");
    }

    let task_types = query
        .task_types
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    if !task_types.is_empty() {
        qb.push(" AND type IN (");
        let mut separated = qb.separated(",");
        for task_type in task_types {
            separated.push_bind(task_type);
        }
        separated.push_unseparated(")");
    }

    qb.push(" ORDER BY createdAt DESC LIMIT ");
    qb.push_bind(limit);

    let rows = qb
        .build_query_as::<TaskRow>()
        .fetch_all(&state.mysql)
        .await?;
    let tasks = rows.into_iter().map(task_to_json).collect::<Vec<_>>();

    Ok(Json(json!({ "tasks": tasks })))
}

pub async fn get(
    State(state): State<AppState>,
    user: AuthUser,
    Path(task_id): Path<String>,
    Query(params): Query<TaskQuery>,
) -> Result<Json<Value>, AppError> {
    let task = sqlx::query_as::<_, TaskRow>(
        "SELECT id, userId, projectId, episodeId, type, targetType, targetId, status, progress, payload, result, errorCode, errorMessage, createdAt, updatedAt FROM tasks WHERE id = ? LIMIT 1",
    )
    .bind(&task_id)
    .fetch_optional(&state.mysql)
    .await?
    .ok_or_else(|| AppError::not_found("task not found"))?;

    if task.user_id != user.id {
        return Err(AppError::not_found("task not found"));
    }

    let include_events = params
        .include_events
        .as_deref()
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let events = if include_events {
        let limit = params.events_limit.unwrap_or(500).clamp(1, 5000);
        let rows = sqlx::query_as::<_, TaskEventRow>(
            "SELECT id, taskId, projectId, eventType, payload, createdAt FROM task_events WHERE taskId = ? ORDER BY id DESC LIMIT ?",
        )
        .bind(&task_id)
        .bind(limit)
        .fetch_all(&state.mysql)
        .await?;

        Some(rows)
    } else {
        None
    };

    Ok(Json(json!({
      "task": task_to_json(task),
      "events": events,
    })))
}

pub async fn delete(
    State(state): State<AppState>,
    user: AuthUser,
    Path(task_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let task: Option<TaskRow> = sqlx::query_as(
        "SELECT id, userId, projectId, episodeId, type, targetType, targetId, status, progress, payload, result, errorCode, errorMessage, createdAt, updatedAt FROM tasks WHERE id = ? LIMIT 1",
    )
    .bind(&task_id)
    .fetch_optional(&state.mysql)
    .await?;

    let Some(task) = task else {
        return Err(AppError::not_found("task not found"));
    };

    if task.user_id != user.id {
        return Err(AppError::not_found("task not found"));
    }

    if task.status == "completed" || task.status == "failed" || task.status == "dismissed" {
        return Ok(Json(json!({
          "success": true,
          "cancelled": false,
          "task": {
            "id": task.id,
            "status": task.status,
          }
        })));
    }

    let update_result = sqlx::query(
        "UPDATE tasks SET status = 'failed', errorCode = ?, errorMessage = ?, finishedAt = NOW(3), heartbeatAt = NULL, updatedAt = NOW(3) WHERE id = ? AND status IN ('queued', 'processing')",
    )
    .bind(TASK_CANCELLED_CODE)
    .bind(TASK_CANCELLED_MESSAGE)
    .bind(&task_id)
    .execute(&state.mysql)
    .await?;

    if update_result.rows_affected() == 0 {
        let latest_status =
            sqlx::query_scalar::<_, String>("SELECT status FROM tasks WHERE id = ? LIMIT 1")
                .bind(&task_id)
                .fetch_optional(&state.mysql)
                .await?
                .unwrap_or_else(|| task.status.clone());

        return Ok(Json(json!({
          "success": true,
          "cancelled": false,
          "task": {
            "id": task.id,
            "status": latest_status,
          }
        })));
    }

    let payload = task_cancel_payload(&task);
    persist_and_publish_task_cancelled(&state, &task, payload).await?;

    Ok(Json(json!({
      "success": true,
      "cancelled": true,
      "task": {
        "id": task.id,
        "status": "failed",
      }
    })))
}

pub async fn dismiss(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<DismissTasksRequest>,
) -> Result<Json<Value>, AppError> {
    if payload.task_ids.is_empty() {
        return Err(AppError::invalid_params("taskIds cannot be empty"));
    }
    if payload.task_ids.len() > 200 {
        return Err(AppError::invalid_params("too many taskIds"));
    }

    let mut builder: QueryBuilder<'_, MySql> = QueryBuilder::new(
        "UPDATE tasks SET status = 'dismissed', updatedAt = NOW(3) WHERE userId = ",
    );
    builder.push_bind(&user.id);
    builder.push(" AND status = 'failed' AND id IN (");

    let mut separated = builder.separated(",");
    for id in &payload.task_ids {
        separated.push_bind(id);
    }
    separated.push_unseparated(")");

    let result = builder.build().execute(&state.mysql).await?;

    Ok(Json(json!({
      "success": true,
      "dismissed": result.rows_affected(),
    })))
}

fn map_status_to_phase(status: &str) -> &'static str {
    match status {
        "queued" => "queued",
        "processing" => "processing",
        "completed" => "completed",
        "failed" => "failed",
        "dismissed" => "idle",
        _ => "idle",
    }
}

pub async fn target_states(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<TaskTargetStatesRequest>,
) -> Result<Json<Value>, AppError> {
    if payload.project_id.trim().is_empty() {
        return Err(AppError::invalid_params("projectId is required"));
    }
    if payload.targets.is_empty() {
        return Ok(Json(json!({ "states": Vec::<Value>::new() })));
    }
    if payload.targets.len() > 500 {
        return Err(AppError::invalid_params("too many targets"));
    }

    verify_project_access(&state, payload.project_id.trim(), &user.id).await?;

    let mut states = Vec::with_capacity(payload.targets.len());

    for target in payload.targets {
        let target_type = target.target_type.trim();
        let target_id = target.target_id.trim();

        if target_type.is_empty() || target_id.is_empty() {
            return Err(AppError::invalid_params(
                "targetType and targetId are required",
            ));
        }

        let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
            "SELECT id, userId, projectId, episodeId, type, targetType, targetId, status, progress, payload, result, errorCode, errorMessage, createdAt, updatedAt FROM tasks WHERE projectId = ",
        );
        qb.push_bind(payload.project_id.trim());
        qb.push(" AND userId = ");
        qb.push_bind(&user.id);
        qb.push(" AND targetType = ");
        qb.push_bind(target_type);
        qb.push(" AND targetId = ");
        qb.push_bind(target_id);

        if let Some(types) = target.types {
            let clean = types
                .into_iter()
                .filter(|item| !item.trim().is_empty())
                .collect::<Vec<_>>();
            if !clean.is_empty() {
                qb.push(" AND type IN (");
                let mut separated = qb.separated(",");
                for task_type in clean {
                    separated.push_bind(task_type);
                }
                separated.push_unseparated(")");
            }
        }

        qb.push(" ORDER BY createdAt DESC LIMIT 1");

        let latest = qb
            .build_query_as::<TaskRow>()
            .fetch_optional(&state.mysql)
            .await?;

        if let Some(task) = latest {
            states.push(json!({
                "targetType": target_type,
                "targetId": target_id,
                "phase": map_status_to_phase(&task.status),
                "status": task.status,
                "taskId": task.id,
                "progress": task.progress,
                "lastError": task.error_message,
            }));
        } else {
            states.push(json!({
                "targetType": target_type,
                "targetId": target_id,
                "phase": "idle",
                "status": "idle",
                "taskId": null,
                "progress": 0,
                "lastError": null,
            }));
        }
    }

    Ok(Json(json!({ "states": states })))
}

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/api/tasks", axum::routing::get(list))
        .route("/api/tasks/{id}", axum::routing::get(get).delete(delete))
        .route("/api/tasks/dismiss", axum::routing::post(dismiss))
        .route(
            "/api/task-target-states",
            axum::routing::post(target_states),
        )
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;

    use super::{TASK_CANCELLED_CODE, TASK_CANCELLED_MESSAGE, TaskRow, task_cancel_payload};

    #[test]
    fn task_cancel_payload_marks_failed_with_cancel_metadata() {
        let task = TaskRow {
            id: "task-1".to_string(),
            user_id: "user-1".to_string(),
            project_id: "project-1".to_string(),
            episode_id: Some("episode-1".to_string()),
            task_type: "story_to_script".to_string(),
            target_type: "episode".to_string(),
            target_id: "episode-1".to_string(),
            status: "processing".to_string(),
            progress: 40,
            payload: None,
            result: None,
            error_code: None,
            error_message: None,
            created_at: NaiveDateTime::default(),
            updated_at: NaiveDateTime::default(),
        };

        let payload = task_cancel_payload(&task);

        assert_eq!(
            payload.get("status").and_then(serde_json::Value::as_str),
            Some("failed")
        );
        assert_eq!(
            payload.get("stage").and_then(serde_json::Value::as_str),
            Some("cancelled")
        );
        assert_eq!(
            payload.get("errorCode").and_then(serde_json::Value::as_str),
            Some(TASK_CANCELLED_CODE)
        );
        assert_eq!(
            payload
                .get("errorMessage")
                .and_then(serde_json::Value::as_str),
            Some(TASK_CANCELLED_MESSAGE)
        );
        assert_eq!(
            payload
                .get("targetType")
                .and_then(serde_json::Value::as_str),
            Some("episode")
        );
    }
}
