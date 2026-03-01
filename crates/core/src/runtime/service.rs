use chrono::{NaiveDateTime, Utc};
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::{MySql, MySqlPool, QueryBuilder, Transaction};
use uuid::Uuid;

use crate::errors::AppError;

use super::types::{
    CreateRunInput, ListRunsInput, RUN_STATE_MAX_BYTES, RunEvent, RunEventInput, RunEventLane,
    RunEventType, RunStatus, RunStepStatus, StateRef,
};

#[derive(Debug, Clone, Serialize)]
pub struct RunRecord {
    pub id: String,
    pub user_id: String,
    pub project_id: String,
    pub episode_id: Option<String>,
    pub workflow_type: String,
    pub task_type: Option<String>,
    pub task_id: Option<String>,
    pub target_type: String,
    pub target_id: String,
    pub status: RunStatus,
    pub input: Option<Value>,
    pub output: Option<Value>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub cancel_requested_at: Option<NaiveDateTime>,
    pub queued_at: NaiveDateTime,
    pub started_at: Option<NaiveDateTime>,
    pub finished_at: Option<NaiveDateTime>,
    pub last_seq: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunStepRecord {
    pub id: String,
    pub run_id: String,
    pub step_key: String,
    pub step_title: String,
    pub status: RunStepStatus,
    pub current_attempt: i32,
    pub step_index: i32,
    pub step_total: i32,
    pub started_at: Option<NaiveDateTime>,
    pub finished_at: Option<NaiveDateTime>,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunSnapshot {
    pub run: RunRecord,
    pub steps: Vec<RunStepRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunCheckpointRecord {
    pub id: String,
    pub run_id: String,
    pub node_key: String,
    pub version: i32,
    pub state_json: Value,
    pub state_bytes: i32,
    pub created_at: NaiveDateTime,
}

#[derive(sqlx::FromRow)]
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

#[derive(sqlx::FromRow)]
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

#[derive(sqlx::FromRow)]
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

#[derive(sqlx::FromRow)]
struct GraphCheckpointRow {
    id: String,
    #[sqlx(rename = "runId")]
    run_id: String,
    #[sqlx(rename = "nodeKey")]
    node_key: String,
    version: i32,
    #[sqlx(rename = "stateJson")]
    state_json: sqlx::types::Json<Value>,
    #[sqlx(rename = "stateBytes")]
    state_bytes: i32,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
}

struct StepProjection {
    step_key: String,
    step_title: String,
    step_index: i32,
    step_total: i32,
    attempt: i32,
    payload: Value,
}

fn to_object(value: Option<Value>) -> Value {
    match value {
        Some(Value::Object(_)) => value.unwrap_or_else(|| json!({})),
        _ => json!({}),
    }
}

fn read_string(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn read_i32(payload: &Value, key: &str) -> Option<i32> {
    payload
        .get(key)
        .and_then(|value| {
            value.as_i64().or_else(|| {
                value
                    .as_str()
                    .and_then(|raw| raw.trim().parse::<i64>().ok())
            })
        })
        .and_then(|value| i32::try_from(value).ok())
}

fn parse_run_status(raw: &str) -> Result<RunStatus, AppError> {
    RunStatus::from_db(raw)
        .ok_or_else(|| AppError::internal(format!("unknown graph run status: {raw}")))
}

fn parse_step_status(raw: &str) -> Result<RunStepStatus, AppError> {
    RunStepStatus::from_db(raw)
        .ok_or_else(|| AppError::internal(format!("unknown graph step status: {raw}")))
}

fn parse_event_type(raw: &str) -> Result<RunEventType, AppError> {
    RunEventType::from_db(raw)
        .ok_or_else(|| AppError::internal(format!("unknown graph event type: {raw}")))
}

fn parse_lane(raw: Option<&str>) -> Option<RunEventLane> {
    raw.and_then(RunEventLane::from_db)
}

fn map_run_row(row: GraphRunRow) -> Result<RunRecord, AppError> {
    Ok(RunRecord {
        id: row.id,
        user_id: row.user_id,
        project_id: row.project_id,
        episode_id: row.episode_id,
        workflow_type: row.workflow_type,
        task_type: row.task_type,
        task_id: row.task_id,
        target_type: row.target_type,
        target_id: row.target_id,
        status: parse_run_status(&row.status)?,
        input: row.input.map(|value| value.0),
        output: row.output.map(|value| value.0),
        error_code: row.error_code,
        error_message: row.error_message,
        cancel_requested_at: row.cancel_requested_at,
        queued_at: row.queued_at,
        started_at: row.started_at,
        finished_at: row.finished_at,
        last_seq: row.last_seq,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn map_step_row(row: GraphStepRow) -> Result<RunStepRecord, AppError> {
    Ok(RunStepRecord {
        id: row.id,
        run_id: row.run_id,
        step_key: row.step_key,
        step_title: row.step_title,
        status: parse_step_status(&row.status)?,
        current_attempt: row.current_attempt,
        step_index: row.step_index,
        step_total: row.step_total,
        started_at: row.started_at,
        finished_at: row.finished_at,
        last_error_code: row.last_error_code,
        last_error_message: row.last_error_message,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn map_event_row(row: GraphEventRow) -> Result<RunEvent, AppError> {
    Ok(RunEvent {
        id: row.id.to_string(),
        run_id: row.run_id,
        project_id: row.project_id,
        user_id: row.user_id,
        seq: row.seq,
        event_type: parse_event_type(&row.event_type)?,
        step_key: row.step_key,
        attempt: row.attempt,
        lane: parse_lane(row.lane.as_deref()),
        payload: row.payload.map(|value| value.0),
        created_at: row.created_at,
    })
}

fn map_checkpoint_row(row: GraphCheckpointRow) -> RunCheckpointRecord {
    RunCheckpointRecord {
        id: row.id,
        run_id: row.run_id,
        node_key: row.node_key,
        version: row.version,
        state_json: row.state_json.0,
        state_bytes: row.state_bytes,
        created_at: row.created_at,
    }
}

fn resolve_error_message(payload: &Value) -> Option<String> {
    read_string(payload, "message")
        .or_else(|| read_string(payload, "errorMessage"))
        .or_else(|| {
            payload
                .get("error")
                .and_then(Value::as_object)
                .and_then(|error| {
                    error
                        .get("message")
                        .and_then(Value::as_str)
                        .map(|value| value.trim().to_string())
                        .filter(|value| !value.is_empty())
                        .or_else(|| {
                            error
                                .get("errorMessage")
                                .and_then(Value::as_str)
                                .map(|value| value.trim().to_string())
                                .filter(|value| !value.is_empty())
                        })
                })
        })
}

fn build_step_projection(input: &RunEventInput) -> Option<StepProjection> {
    let payload = to_object(input.payload.clone());
    let step_key = input
        .step_key
        .clone()
        .or_else(|| read_string(&payload, "stepKey"))
        .or_else(|| read_string(&payload, "stepId"))?;
    let step_title = read_string(&payload, "stepTitle").unwrap_or_else(|| step_key.clone());
    let step_index = read_i32(&payload, "stepIndex").unwrap_or(1).max(1);
    let step_total = read_i32(&payload, "stepTotal")
        .unwrap_or(step_index)
        .max(step_index);
    let attempt = input
        .attempt
        .unwrap_or_else(|| read_i32(&payload, "stepAttempt").unwrap_or(1))
        .max(1);

    Some(StepProjection {
        step_key,
        step_title,
        step_index,
        step_total,
        attempt,
        payload,
    })
}

async fn apply_run_projection(
    tx: &mut Transaction<'_, MySql>,
    input: &RunEventInput,
) -> Result<(), AppError> {
    let payload = to_object(input.payload.clone());
    let now = Utc::now().naive_utc();

    match input.event_type {
        RunEventType::RunStart => {
            sqlx::query(
                "UPDATE graph_runs SET status = 'running', startedAt = COALESCE(startedAt, ?), updatedAt = ? WHERE id = ?",
            )
            .bind(now)
            .bind(now)
            .bind(&input.run_id)
            .execute(&mut **tx)
            .await?;
            return Ok(());
        }
        RunEventType::RunComplete => {
            sqlx::query(
                "UPDATE graph_runs SET status = 'completed', output = ?, finishedAt = ?, updatedAt = ? WHERE id = ?",
            )
            .bind(sqlx::types::Json(payload.clone()))
            .bind(now)
            .bind(now)
            .bind(&input.run_id)
            .execute(&mut **tx)
            .await?;

            sqlx::query(
                "UPDATE graph_steps SET status = 'completed', finishedAt = ?, updatedAt = ? WHERE runId = ? AND status IN ('pending', 'running')",
            )
            .bind(now)
            .bind(now)
            .bind(&input.run_id)
            .execute(&mut **tx)
            .await?;
            return Ok(());
        }
        RunEventType::RunError => {
            sqlx::query(
                "UPDATE graph_runs SET status = 'failed', errorCode = ?, errorMessage = ?, finishedAt = ?, updatedAt = ? WHERE id = ?",
            )
            .bind(read_string(&payload, "errorCode"))
            .bind(resolve_error_message(&payload))
            .bind(now)
            .bind(now)
            .bind(&input.run_id)
            .execute(&mut **tx)
            .await?;

            sqlx::query(
                "UPDATE graph_steps SET status = 'failed', finishedAt = ?, lastErrorCode = ?, lastErrorMessage = ?, updatedAt = ? WHERE runId = ? AND status IN ('pending', 'running')",
            )
            .bind(now)
            .bind(read_string(&payload, "errorCode"))
            .bind(resolve_error_message(&payload))
            .bind(now)
            .bind(&input.run_id)
            .execute(&mut **tx)
            .await?;
            return Ok(());
        }
        RunEventType::RunCanceled => {
            sqlx::query(
                "UPDATE graph_runs SET status = 'canceled', finishedAt = ?, updatedAt = ? WHERE id = ?",
            )
            .bind(now)
            .bind(now)
            .bind(&input.run_id)
            .execute(&mut **tx)
            .await?;

            sqlx::query(
                "UPDATE graph_steps SET status = 'canceled', finishedAt = ?, lastErrorCode = 'CANCELED', lastErrorMessage = 'Run cancelled', updatedAt = ? WHERE runId = ? AND status IN ('pending', 'running')",
            )
            .bind(now)
            .bind(now)
            .bind(&input.run_id)
            .execute(&mut **tx)
            .await?;
            return Ok(());
        }
        RunEventType::StepStart
        | RunEventType::StepChunk
        | RunEventType::StepComplete
        | RunEventType::StepError => {}
    }

    let Some(step) = build_step_projection(input) else {
        return Ok(());
    };

    let next_status = match input.event_type {
        RunEventType::StepError => RunStepStatus::Failed,
        RunEventType::StepComplete => RunStepStatus::Completed,
        RunEventType::StepStart | RunEventType::StepChunk => RunStepStatus::Running,
        _ => RunStepStatus::Running,
    };
    let is_step_failed = next_status == RunStepStatus::Failed;
    let is_step_finished = matches!(
        next_status,
        RunStepStatus::Completed | RunStepStatus::Failed | RunStepStatus::Canceled
    );

    sqlx::query(
        "UPDATE graph_runs SET status = 'running', startedAt = COALESCE(startedAt, ?), updatedAt = ? WHERE id = ? AND status IN ('queued', 'running')",
    )
    .bind(now)
    .bind(now)
    .bind(&input.run_id)
    .execute(&mut **tx)
    .await?;

    let step_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO graph_steps (id, runId, stepKey, stepTitle, status, currentAttempt, stepIndex, stepTotal, startedAt, finishedAt, lastErrorCode, lastErrorMessage, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE stepTitle = VALUES(stepTitle), status = VALUES(status), currentAttempt = VALUES(currentAttempt), stepIndex = VALUES(stepIndex), stepTotal = VALUES(stepTotal), startedAt = COALESCE(graph_steps.startedAt, VALUES(startedAt)), finishedAt = VALUES(finishedAt), lastErrorCode = VALUES(lastErrorCode), lastErrorMessage = VALUES(lastErrorMessage), updatedAt = VALUES(updatedAt)",
    )
    .bind(step_id)
    .bind(&input.run_id)
    .bind(&step.step_key)
    .bind(&step.step_title)
    .bind(next_status.as_str())
    .bind(step.attempt)
    .bind(step.step_index)
    .bind(step.step_total)
    .bind(now)
    .bind(if is_step_finished { Some(now) } else { None })
    .bind(if is_step_failed {
        read_string(&step.payload, "errorCode")
    } else {
        None
    })
    .bind(if is_step_failed {
        resolve_error_message(&step.payload)
    } else {
        None
    })
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await?;

    let attempt_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO graph_step_attempts (id, runId, stepKey, attempt, status, outputText, outputReasoning, errorCode, errorMessage, startedAt, finishedAt, usageJson, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE status = VALUES(status), outputText = VALUES(outputText), outputReasoning = VALUES(outputReasoning), errorCode = VALUES(errorCode), errorMessage = VALUES(errorMessage), finishedAt = VALUES(finishedAt), usageJson = VALUES(usageJson), updatedAt = VALUES(updatedAt)",
    )
    .bind(attempt_id)
    .bind(&input.run_id)
    .bind(&step.step_key)
    .bind(step.attempt)
    .bind(next_status.as_str())
    .bind(if next_status == RunStepStatus::Completed {
        read_string(&step.payload, "text")
    } else {
        None
    })
    .bind(if next_status == RunStepStatus::Completed {
        read_string(&step.payload, "reasoning")
    } else {
        None
    })
    .bind(if is_step_failed {
        read_string(&step.payload, "errorCode")
    } else {
        None
    })
    .bind(if is_step_failed {
        resolve_error_message(&step.payload)
    } else {
        None
    })
    .bind(now)
    .bind(if is_step_finished { Some(now) } else { None })
    .bind(sqlx::types::Json(
        step.payload
            .get("usage")
            .cloned()
            .unwrap_or_else(|| json!({})),
    ))
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn create_run(pool: &MySqlPool, input: &CreateRunInput) -> Result<RunRecord, AppError> {
    let run_id = Uuid::new_v4().to_string();
    let now = Utc::now().naive_utc();
    sqlx::query(
        "INSERT INTO graph_runs (id, userId, projectId, episodeId, workflowType, taskType, taskId, targetType, targetId, status, input, queuedAt, lastSeq, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'queued', ?, ?, 0, ?, ?)",
    )
    .bind(&run_id)
    .bind(&input.user_id)
    .bind(&input.project_id)
    .bind(input.episode_id.clone())
    .bind(&input.workflow_type)
    .bind(input.task_type.clone())
    .bind(input.task_id.clone())
    .bind(&input.target_type)
    .bind(&input.target_id)
    .bind(input.input.clone().map(sqlx::types::Json))
    .bind(now)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    let row = sqlx::query_as::<_, GraphRunRow>(
        "SELECT id, userId, projectId, episodeId, workflowType, taskType, taskId, targetType, targetId, status, input, output, errorCode, errorMessage, cancelRequestedAt, queuedAt, startedAt, finishedAt, lastSeq, createdAt, updatedAt FROM graph_runs WHERE id = ? LIMIT 1",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;

    map_run_row(row)
}

pub async fn attach_task_to_run(
    pool: &MySqlPool,
    run_id: &str,
    task_id: &str,
) -> Result<RunRecord, AppError> {
    sqlx::query("UPDATE graph_runs SET taskId = ?, updatedAt = NOW(3) WHERE id = ?")
        .bind(task_id)
        .bind(run_id)
        .execute(pool)
        .await?;

    let row = sqlx::query_as::<_, GraphRunRow>(
        "SELECT id, userId, projectId, episodeId, workflowType, taskType, taskId, targetType, targetId, status, input, output, errorCode, errorMessage, cancelRequestedAt, queuedAt, startedAt, finishedAt, lastSeq, createdAt, updatedAt FROM graph_runs WHERE id = ? LIMIT 1",
    )
    .bind(run_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::not_found("run not found"))?;

    map_run_row(row)
}

pub async fn get_run_by_id(pool: &MySqlPool, run_id: &str) -> Result<Option<RunRecord>, AppError> {
    let row = sqlx::query_as::<_, GraphRunRow>(
        "SELECT id, userId, projectId, episodeId, workflowType, taskType, taskId, targetType, targetId, status, input, output, errorCode, errorMessage, cancelRequestedAt, queuedAt, startedAt, finishedAt, lastSeq, createdAt, updatedAt FROM graph_runs WHERE id = ? LIMIT 1",
    )
    .bind(run_id)
    .fetch_optional(pool)
    .await?;

    row.map(map_run_row).transpose()
}

pub async fn get_run_snapshot(
    pool: &MySqlPool,
    run_id: &str,
) -> Result<Option<RunSnapshot>, AppError> {
    let run = get_run_by_id(pool, run_id).await?;
    let Some(run) = run else {
        return Ok(None);
    };

    let steps = sqlx::query_as::<_, GraphStepRow>(
        "SELECT id, runId, stepKey, stepTitle, status, currentAttempt, stepIndex, stepTotal, startedAt, finishedAt, lastErrorCode, lastErrorMessage, createdAt, updatedAt FROM graph_steps WHERE runId = ? ORDER BY stepIndex ASC, updatedAt ASC",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;

    let mapped_steps = steps
        .into_iter()
        .map(map_step_row)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Some(RunSnapshot {
        run,
        steps: mapped_steps,
    }))
}

pub async fn list_runs(
    pool: &MySqlPool,
    input: &ListRunsInput,
) -> Result<Vec<RunRecord>, AppError> {
    let limit = input.limit.unwrap_or(50).clamp(1, 200);

    let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
        "SELECT id, userId, projectId, episodeId, workflowType, taskType, taskId, targetType, targetId, status, input, output, errorCode, errorMessage, cancelRequestedAt, queuedAt, startedAt, finishedAt, lastSeq, createdAt, updatedAt FROM graph_runs WHERE userId = ",
    );
    qb.push_bind(&input.user_id);

    if let Some(project_id) = input
        .project_id
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        qb.push(" AND projectId = ");
        qb.push_bind(project_id.trim());
    }
    if let Some(workflow_type) = input
        .workflow_type
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        qb.push(" AND workflowType = ");
        qb.push_bind(workflow_type.trim());
    }
    if let Some(target_type) = input
        .target_type
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        qb.push(" AND targetType = ");
        qb.push_bind(target_type.trim());
    }
    if let Some(target_id) = input
        .target_id
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        qb.push(" AND targetId = ");
        qb.push_bind(target_id.trim());
    }
    if let Some(episode_id) = input
        .episode_id
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        qb.push(" AND episodeId = ");
        qb.push_bind(episode_id.trim());
    }
    if !input.statuses.is_empty() {
        qb.push(" AND status IN (");
        let mut separated = qb.separated(",");
        for status in &input.statuses {
            separated.push_bind(status.as_str());
        }
        separated.push_unseparated(")");
    }

    qb.push(" ORDER BY createdAt DESC LIMIT ");
    qb.push_bind(limit);

    let rows = qb.build_query_as::<GraphRunRow>().fetch_all(pool).await?;
    rows.into_iter().map(map_run_row).collect()
}

pub async fn request_run_cancel(
    pool: &MySqlPool,
    run_id: &str,
    user_id: &str,
) -> Result<Option<RunRecord>, AppError> {
    sqlx::query(
        "UPDATE graph_runs SET status = CASE WHEN status IN ('queued','running') THEN 'canceling' ELSE status END, cancelRequestedAt = CASE WHEN status IN ('queued','running') THEN NOW(3) ELSE cancelRequestedAt END, updatedAt = NOW(3) WHERE id = ? AND userId = ?",
    )
    .bind(run_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    let row = sqlx::query_as::<_, GraphRunRow>(
        "SELECT id, userId, projectId, episodeId, workflowType, taskType, taskId, targetType, targetId, status, input, output, errorCode, errorMessage, cancelRequestedAt, queuedAt, startedAt, finishedAt, lastSeq, createdAt, updatedAt FROM graph_runs WHERE id = ? LIMIT 1",
    )
    .bind(run_id)
    .fetch_optional(pool)
    .await?;

    row.map(map_run_row).transpose()
}

pub async fn append_run_event_with_seq(
    pool: &MySqlPool,
    input: &RunEventInput,
) -> Result<RunEvent, AppError> {
    let mut tx = pool.begin().await?;

    let update_result =
        sqlx::query("UPDATE graph_runs SET lastSeq = lastSeq + 1, updatedAt = NOW(3) WHERE id = ?")
            .bind(&input.run_id)
            .execute(&mut *tx)
            .await?;
    if update_result.rows_affected() == 0 {
        tx.rollback().await?;
        return Err(AppError::not_found("run not found"));
    }

    let (seq,): (i32,) = sqlx::query_as("SELECT lastSeq FROM graph_runs WHERE id = ? LIMIT 1")
        .bind(&input.run_id)
        .fetch_one(&mut *tx)
        .await?;

    let insert_result = sqlx::query(
        "INSERT INTO graph_events (runId, projectId, userId, seq, eventType, stepKey, attempt, lane, payload, createdAt) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3))",
    )
    .bind(&input.run_id)
    .bind(&input.project_id)
    .bind(&input.user_id)
    .bind(seq)
    .bind(input.event_type.as_str())
    .bind(input.step_key.as_deref())
    .bind(input.attempt)
    .bind(input.lane.map(|lane| lane.as_str()))
    .bind(input.payload.clone().map(sqlx::types::Json))
    .execute(&mut *tx)
    .await?;

    apply_run_projection(&mut tx, input).await?;

    let event_id = i64::try_from(insert_result.last_insert_id())
        .map_err(|error| AppError::internal(format!("event id overflow: {error}")))?;
    let row = sqlx::query_as::<_, GraphEventRow>(
        "SELECT id, runId, projectId, userId, seq, eventType, stepKey, attempt, lane, payload, createdAt FROM graph_events WHERE id = ? LIMIT 1",
    )
    .bind(event_id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    map_event_row(row)
}

pub async fn list_run_events_after_seq(
    pool: &MySqlPool,
    run_id: &str,
    user_id: &str,
    after_seq: Option<i32>,
    limit: Option<i64>,
) -> Result<Vec<RunEvent>, AppError> {
    let owner: Option<(String,)> =
        sqlx::query_as("SELECT userId FROM graph_runs WHERE id = ? LIMIT 1")
            .bind(run_id)
            .fetch_optional(pool)
            .await?;
    let Some((owner_id,)) = owner else {
        return Ok(Vec::new());
    };
    if owner_id != user_id {
        return Ok(Vec::new());
    }

    let safe_after = after_seq.unwrap_or(0).max(0);
    let safe_limit = limit.unwrap_or(200).clamp(1, 2000);

    let rows = sqlx::query_as::<_, GraphEventRow>(
        "SELECT id, runId, projectId, userId, seq, eventType, stepKey, attempt, lane, payload, createdAt FROM graph_events WHERE runId = ? AND seq > ? ORDER BY seq ASC LIMIT ?",
    )
    .bind(run_id)
    .bind(safe_after)
    .bind(safe_limit)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(map_event_row).collect()
}

pub fn assert_checkpoint_state_size(state: &Value) -> Result<usize, AppError> {
    let bytes = serde_json::to_vec(state)
        .map_err(|error| AppError::invalid_params(format!("invalid checkpoint state: {error}")))?
        .len();
    if bytes > RUN_STATE_MAX_BYTES {
        return Err(AppError::invalid_params(format!(
            "checkpoint state too large: {bytes} bytes (max {RUN_STATE_MAX_BYTES})"
        )));
    }
    Ok(bytes)
}

pub fn build_lean_state(refs: &StateRef, meta: Option<Value>) -> Value {
    json!({
      "refs": {
        "scriptId": refs.script_id,
        "storyboardId": refs.storyboard_id,
        "voiceLineBatchId": refs.voice_line_batch_id,
        "versionHash": refs.version_hash,
        "cursor": refs.cursor,
      },
      "meta": meta.unwrap_or_else(|| json!({})),
    })
}

pub async fn create_checkpoint(
    pool: &MySqlPool,
    run_id: &str,
    node_key: &str,
    version: i32,
    state: &Value,
) -> Result<(), AppError> {
    let bytes = assert_checkpoint_state_size(state)?;
    let state_bytes = i32::try_from(bytes)
        .map_err(|error| AppError::internal(format!("checkpoint state bytes overflow: {error}")))?;

    sqlx::query(
        "INSERT INTO graph_checkpoints (id, runId, nodeKey, version, stateJson, stateBytes, createdAt) VALUES (?, ?, ?, ?, ?, ?, NOW(3))",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(run_id)
    .bind(node_key)
    .bind(version)
    .bind(sqlx::types::Json(state.clone()))
    .bind(state_bytes)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_checkpoints(
    pool: &MySqlPool,
    run_id: &str,
    node_key: Option<&str>,
    limit: Option<i64>,
) -> Result<Vec<RunCheckpointRecord>, AppError> {
    let safe_limit = limit.unwrap_or(20).clamp(1, 200);

    let rows = if let Some(node_key) = node_key.filter(|value| !value.trim().is_empty()) {
        sqlx::query_as::<_, GraphCheckpointRow>(
            "SELECT id, runId, nodeKey, version, stateJson, stateBytes, createdAt FROM graph_checkpoints WHERE runId = ? AND nodeKey = ? ORDER BY version DESC LIMIT ?",
        )
        .bind(run_id)
        .bind(node_key)
        .bind(safe_limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, GraphCheckpointRow>(
            "SELECT id, runId, nodeKey, version, stateJson, stateBytes, createdAt FROM graph_checkpoints WHERE runId = ? ORDER BY version DESC LIMIT ?",
        )
        .bind(run_id)
        .bind(safe_limit)
        .fetch_all(pool)
        .await?
    };

    Ok(rows.into_iter().map(map_checkpoint_row).collect())
}
