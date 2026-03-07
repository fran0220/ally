use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;
use waoowaoo_core::runtime::{
    publisher::{
        TaskLifecycleMessageInput, build_task_lifecycle_message, publish_run_event,
        publish_task_message,
    },
    types::{RunEventInput, RunEventType},
};

use crate::{app_state::AppState, error::AppError, extractors::auth::AuthUser};

#[derive(Debug, Deserialize)]
pub struct CreateRunRequest {
    #[serde(rename = "projectId")]
    pub project_id: String,
    #[serde(rename = "workflowType")]
    pub workflow_type: String,
    #[serde(rename = "targetType")]
    pub target_type: String,
    #[serde(rename = "targetId")]
    pub target_id: String,
    #[serde(rename = "episodeId")]
    pub episode_id: Option<String>,
    #[serde(rename = "taskType")]
    pub task_type: Option<String>,
    #[serde(rename = "taskId")]
    pub task_id: Option<String>,
    pub input: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct ListRunsQuery {
    #[serde(default, rename = "projectId")]
    pub project_id: Option<String>,
    #[serde(default, rename = "workflowType")]
    pub workflow_type: Option<String>,
    #[serde(default, rename = "targetType")]
    pub target_type: Option<String>,
    #[serde(default, rename = "targetId")]
    pub target_id: Option<String>,
    #[serde(default, rename = "episodeId")]
    pub episode_id: Option<String>,
    #[serde(default, rename = "status")]
    pub statuses: Vec<String>,
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RunEventsQuery {
    #[serde(default, rename = "afterSeq")]
    pub after_seq: Option<i64>,
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct GraphRunRow {
    id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
    #[sqlx(rename = "projectId")]
    project_id: String,
    #[sqlx(rename = "episodeId")]
    episode_id: Option<String>,
    #[sqlx(rename = "workflowType")]
    workflow_type: String,
    #[sqlx(rename = "taskType")]
    task_type: Option<String>,
    #[sqlx(rename = "taskId")]
    task_id: Option<String>,
    #[sqlx(rename = "targetType")]
    target_type: String,
    #[sqlx(rename = "targetId")]
    target_id: String,
    status: String,
    input: Option<sqlx::types::Json<Value>>,
    output: Option<sqlx::types::Json<Value>>,
    #[sqlx(rename = "errorCode")]
    error_code: Option<String>,
    #[sqlx(rename = "errorMessage")]
    error_message: Option<String>,
    #[sqlx(rename = "cancelRequestedAt")]
    cancel_requested_at: Option<NaiveDateTime>,
    #[sqlx(rename = "queuedAt")]
    queued_at: NaiveDateTime,
    #[sqlx(rename = "startedAt")]
    started_at: Option<NaiveDateTime>,
    #[sqlx(rename = "finishedAt")]
    finished_at: Option<NaiveDateTime>,
    #[sqlx(rename = "lastSeq")]
    last_seq: i32,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct GraphStepRow {
    id: String,
    #[sqlx(rename = "runId")]
    run_id: String,
    #[sqlx(rename = "stepKey")]
    step_key: String,
    #[sqlx(rename = "stepTitle")]
    step_title: String,
    status: String,
    #[sqlx(rename = "currentAttempt")]
    current_attempt: i32,
    #[sqlx(rename = "stepIndex")]
    step_index: i32,
    #[sqlx(rename = "stepTotal")]
    step_total: i32,
    #[sqlx(rename = "startedAt")]
    started_at: Option<NaiveDateTime>,
    #[sqlx(rename = "finishedAt")]
    finished_at: Option<NaiveDateTime>,
    #[sqlx(rename = "lastErrorCode")]
    last_error_code: Option<String>,
    #[sqlx(rename = "lastErrorMessage")]
    last_error_message: Option<String>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct GraphEventRow {
    id: i64,
    #[sqlx(rename = "runId")]
    run_id: String,
    #[sqlx(rename = "projectId")]
    project_id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
    seq: i32,
    #[sqlx(rename = "eventType")]
    event_type: String,
    #[sqlx(rename = "stepKey")]
    step_key: Option<String>,
    attempt: Option<i32>,
    lane: Option<String>,
    payload: Option<sqlx::types::Json<Value>>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
}

const TASK_CANCELLED_CODE: &str = "TASK_CANCELLED";

#[derive(Debug, sqlx::FromRow)]
struct LinkedTaskRow {
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
}

fn linked_task_cancel_payload(task: &LinkedTaskRow, reason: &str) -> Value {
    json!({
      "status": "failed",
      "stage": "cancelled",
      "errorCode": TASK_CANCELLED_CODE,
      "errorMessage": reason,
      "cancelled": true,
      "targetType": task.target_type,
      "targetId": task.target_id,
      "episodeId": task.episode_id,
    })
}

fn normalize_statuses(statuses: Vec<String>) -> Vec<String> {
    let allowed = [
        "queued",
        "running",
        "completed",
        "failed",
        "canceling",
        "canceled",
    ];

    let mut out = Vec::new();
    for status in statuses {
        let normalized = status.trim().to_lowercase();
        if allowed.contains(&normalized.as_str()) && !out.contains(&normalized) {
            out.push(normalized);
        }
    }
    out
}

fn map_run(row: GraphRunRow) -> Value {
    json!({
      "id": row.id,
      "userId": row.user_id,
      "projectId": row.project_id,
      "episodeId": row.episode_id,
      "workflowType": row.workflow_type,
      "taskType": row.task_type,
      "taskId": row.task_id,
      "targetType": row.target_type,
      "targetId": row.target_id,
      "status": row.status,
      "input": row.input.map(|v| v.0).unwrap_or_else(|| json!({})),
      "output": row.output.map(|v| v.0).unwrap_or_else(|| json!({})),
      "errorCode": row.error_code,
      "errorMessage": row.error_message,
      "cancelRequestedAt": row.cancel_requested_at,
      "queuedAt": row.queued_at,
      "startedAt": row.started_at,
      "finishedAt": row.finished_at,
      "lastSeq": row.last_seq,
      "createdAt": row.created_at,
      "updatedAt": row.updated_at,
    })
}

async fn cancel_linked_task(
    state: &AppState,
    task_id: &str,
    reason: &str,
) -> Result<bool, AppError> {
    let task = sqlx::query_as::<_, LinkedTaskRow>(
        "SELECT id, projectId, userId, episodeId, type, targetType, targetId, status FROM tasks WHERE id = ? LIMIT 1",
    )
    .bind(task_id)
    .fetch_optional(&state.mysql)
    .await?;

    let Some(task) = task else {
        return Ok(false);
    };
    if task.status != "queued" && task.status != "processing" {
        return Ok(false);
    }

    let update_result = sqlx::query(
        "UPDATE tasks SET status = 'failed', errorCode = ?, errorMessage = ?, dedupeKey = NULL, finishedAt = NOW(3), heartbeatAt = NULL, updatedAt = NOW(3) WHERE id = ? AND status IN ('queued','processing')",
    )
    .bind(TASK_CANCELLED_CODE)
    .bind(reason)
    .bind(task_id)
    .execute(&state.mysql)
    .await?;

    if update_result.rows_affected() == 0 {
        return Ok(false);
    }

    let payload = linked_task_cancel_payload(&task, reason);
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

    publish_task_message(&state.redis, &task.project_id, &message).await?;
    Ok(true)
}

pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ListRunsQuery>,
) -> Result<Json<Value>, AppError> {
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let statuses = normalize_statuses(query.statuses);

    let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
        "SELECT id, userId, projectId, episodeId, workflowType, taskType, taskId, targetType, targetId, status, input, output, errorCode, errorMessage, cancelRequestedAt, queuedAt, startedAt, finishedAt, lastSeq, createdAt, updatedAt FROM graph_runs WHERE userId = ",
    );
    qb.push_bind(&user.id);

    if let Some(project_id) = query.project_id
        && !project_id.trim().is_empty()
    {
        qb.push(" AND projectId = ");
        qb.push_bind(project_id.trim().to_string());
    }
    if let Some(workflow_type) = query.workflow_type
        && !workflow_type.trim().is_empty()
    {
        qb.push(" AND workflowType = ");
        qb.push_bind(workflow_type.trim().to_string());
    }
    if let Some(target_type) = query.target_type
        && !target_type.trim().is_empty()
    {
        qb.push(" AND targetType = ");
        qb.push_bind(target_type.trim().to_string());
    }
    if let Some(target_id) = query.target_id
        && !target_id.trim().is_empty()
    {
        qb.push(" AND targetId = ");
        qb.push_bind(target_id.trim().to_string());
    }
    if let Some(episode_id) = query.episode_id
        && !episode_id.trim().is_empty()
    {
        qb.push(" AND episodeId = ");
        qb.push_bind(episode_id.trim().to_string());
    }

    if !statuses.is_empty() {
        qb.push(" AND status IN (");
        let mut separated = qb.separated(",");
        for status in statuses {
            separated.push_bind(status);
        }
        separated.push_unseparated(")");
    }

    qb.push(" ORDER BY createdAt DESC LIMIT ");
    qb.push_bind(limit);

    let rows = qb
        .build_query_as::<GraphRunRow>()
        .fetch_all(&state.mysql)
        .await?;
    let runs = rows.into_iter().map(map_run).collect::<Vec<_>>();

    Ok(Json(json!({ "runs": runs })))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateRunRequest>,
) -> Result<Json<Value>, AppError> {
    let project_id = body.project_id.trim();
    let workflow_type = body.workflow_type.trim();
    let target_type = body.target_type.trim();
    let target_id = body.target_id.trim();

    if project_id.is_empty()
        || workflow_type.is_empty()
        || target_type.is_empty()
        || target_id.is_empty()
    {
        return Err(AppError::invalid_params(
            "projectId/workflowType/targetType/targetId are required",
        ));
    }

    let run_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO graph_runs (id, userId, projectId, episodeId, workflowType, taskType, taskId, targetType, targetId, status, input, queuedAt, lastSeq, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'queued', ?, NOW(3), 0, NOW(3), NOW(3))",
    )
    .bind(&run_id)
    .bind(&user.id)
    .bind(project_id)
    .bind(body.episode_id.map(|v| v.trim().to_string()).filter(|v| !v.is_empty()))
    .bind(workflow_type)
    .bind(body.task_type.map(|v| v.trim().to_string()).filter(|v| !v.is_empty()))
    .bind(body.task_id.map(|v| v.trim().to_string()).filter(|v| !v.is_empty()))
    .bind(target_type)
    .bind(target_id)
    .bind(body.input)
    .execute(&state.mysql)
    .await?;

    let run = sqlx::query_as::<_, GraphRunRow>(
        "SELECT id, userId, projectId, episodeId, workflowType, taskType, taskId, targetType, targetId, status, input, output, errorCode, errorMessage, cancelRequestedAt, queuedAt, startedAt, finishedAt, lastSeq, createdAt, updatedAt FROM graph_runs WHERE id = ? LIMIT 1",
    )
    .bind(&run_id)
    .fetch_one(&state.mysql)
    .await?;

    Ok(Json(json!({
      "success": true,
      "runId": run_id,
      "run": map_run(run),
    })))
}

pub async fn get(
    State(state): State<AppState>,
    user: AuthUser,
    Path(run_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let run = sqlx::query_as::<_, GraphRunRow>(
        "SELECT id, userId, projectId, episodeId, workflowType, taskType, taskId, targetType, targetId, status, input, output, errorCode, errorMessage, cancelRequestedAt, queuedAt, startedAt, finishedAt, lastSeq, createdAt, updatedAt FROM graph_runs WHERE id = ? LIMIT 1",
    )
    .bind(&run_id)
    .fetch_optional(&state.mysql)
    .await?
    .ok_or_else(|| AppError::not_found("run not found"))?;

    if run.user_id != user.id {
        return Err(AppError::not_found("run not found"));
    }

    let events = sqlx::query_as::<_, GraphStepRow>(
        "SELECT id, runId, stepKey, stepTitle, status, currentAttempt, stepIndex, stepTotal, startedAt, finishedAt, lastErrorCode, lastErrorMessage, createdAt, updatedAt FROM graph_steps WHERE runId = ? ORDER BY stepIndex ASC, updatedAt ASC",
    )
    .bind(&run_id)
    .fetch_all(&state.mysql)
    .await?;

    Ok(Json(json!({
      "run": map_run(run),
      "events": events,
    })))
}

pub async fn cancel(
    State(state): State<AppState>,
    user: AuthUser,
    Path(run_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let reason = "Run cancelled by user";

    sqlx::query(
        "UPDATE graph_runs SET status = CASE WHEN status IN ('queued','running') THEN 'canceling' ELSE status END, cancelRequestedAt = CASE WHEN status IN ('queued','running') THEN NOW(3) ELSE cancelRequestedAt END, updatedAt = NOW(3) WHERE id = ? AND userId = ?",
    )
    .bind(&run_id)
    .bind(&user.id)
    .execute(&state.mysql)
    .await?;

    let run = sqlx::query_as::<_, GraphRunRow>(
        "SELECT id, userId, projectId, episodeId, workflowType, taskType, taskId, targetType, targetId, status, input, output, errorCode, errorMessage, cancelRequestedAt, queuedAt, startedAt, finishedAt, lastSeq, createdAt, updatedAt FROM graph_runs WHERE id = ? LIMIT 1",
    )
    .bind(&run_id)
    .fetch_optional(&state.mysql)
    .await?
    .ok_or_else(|| AppError::not_found("run not found"))?;

    if run.user_id != user.id {
        return Err(AppError::not_found("run not found"));
    }

    if let Some(task_id) = run.task_id.clone() {
        let _ = cancel_linked_task(&state, &task_id, reason).await?;
    }

    if run.status == "canceling" || run.status == "canceled" {
        let input = RunEventInput {
            run_id: run.id.clone(),
            project_id: run.project_id.clone(),
            user_id: run.user_id.clone(),
            event_type: RunEventType::RunCanceled,
            step_key: None,
            attempt: None,
            lane: None,
            payload: Some(json!({ "message": reason })),
        };
        let _ = publish_run_event(&state.mysql, &state.redis, &input).await?;
    }

    let run = sqlx::query_as::<_, GraphRunRow>(
        "SELECT id, userId, projectId, episodeId, workflowType, taskType, taskId, targetType, targetId, status, input, output, errorCode, errorMessage, cancelRequestedAt, queuedAt, startedAt, finishedAt, lastSeq, createdAt, updatedAt FROM graph_runs WHERE id = ? LIMIT 1",
    )
    .bind(&run_id)
    .fetch_optional(&state.mysql)
    .await?
    .ok_or_else(|| AppError::not_found("run not found"))?;

    Ok(Json(json!({
      "success": true,
      "run": map_run(run),
    })))
}

pub async fn events(
    State(state): State<AppState>,
    user: AuthUser,
    Path(run_id): Path<String>,
    Query(query): Query<RunEventsQuery>,
) -> Result<Json<Value>, AppError> {
    let owner: Option<(String,)> =
        sqlx::query_as("SELECT userId FROM graph_runs WHERE id = ? LIMIT 1")
            .bind(&run_id)
            .fetch_optional(&state.mysql)
            .await?;

    let Some((owner_id,)) = owner else {
        return Err(AppError::not_found("run not found"));
    };
    if owner_id != user.id {
        return Err(AppError::not_found("run not found"));
    }

    let after_seq = query.after_seq.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(200).clamp(1, 2000);

    let rows = sqlx::query_as::<_, GraphEventRow>(
        "SELECT id, runId, projectId, userId, seq, eventType, stepKey, attempt, lane, payload, createdAt FROM graph_events WHERE runId = ? AND seq > ? ORDER BY seq ASC LIMIT ?",
    )
    .bind(&run_id)
    .bind(after_seq)
    .bind(limit)
    .fetch_all(&state.mysql)
    .await?;

    Ok(Json(json!({
      "runId": run_id,
      "afterSeq": after_seq,
      "events": rows,
    })))
}

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/api/runs", axum::routing::get(list).post(create))
        .route("/api/runs/{runId}", axum::routing::get(get))
        .route("/api/runs/{runId}/cancel", axum::routing::post(cancel))
        .route("/api/runs/{runId}/events", axum::routing::get(events))
}

#[cfg(test)]
mod tests {
    use super::{LinkedTaskRow, TASK_CANCELLED_CODE, linked_task_cancel_payload};

    #[test]
    fn linked_task_cancel_payload_marks_failed_with_reason() {
        let task = LinkedTaskRow {
            id: "task-1".to_string(),
            project_id: "project-1".to_string(),
            user_id: "user-1".to_string(),
            episode_id: Some("episode-1".to_string()),
            task_type: "story_to_script".to_string(),
            target_type: "episode".to_string(),
            target_id: "episode-1".to_string(),
            status: "processing".to_string(),
        };

        let payload = linked_task_cancel_payload(&task, "Run cancelled by user");

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
            Some("Run cancelled by user")
        );
    }
}
