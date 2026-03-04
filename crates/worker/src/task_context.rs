use std::{
    ops::Deref,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use chrono::{SecondsFormat, Utc};
use deadpool_redis::Pool as RedisPool;
use serde_json::{Map, Value, json};
use sqlx::MySqlPool;
use uuid::Uuid;
use waoowaoo_core::{
    billing::{
        BillingStatus, TaskBillingInfo, parse_task_billing_info, rollback_task_billing,
        serialize_task_billing_info, settle_task_billing,
    },
    errors::AppError,
    runtime::{
        publisher::{
            TaskLifecycleMessageInput, build_task_lifecycle_message, publish_run_event,
            publish_task_message,
        },
        task_bridge::{TaskSseEvent, map_task_sse_event_to_run_events},
    },
};

use crate::consumer::WorkerTask;

#[derive(Debug, Clone, Default)]
struct FlowFields {
    flow_id: Option<String>,
    flow_stage_title: Option<String>,
    flow_stage_index: Option<i64>,
    flow_stage_total: Option<i64>,
}

impl FlowFields {
    fn from_payload(payload: &Value) -> Self {
        let Some(payload) = payload.as_object() else {
            return Self::default();
        };

        Self {
            flow_id: read_string_field(payload, "flowId"),
            flow_stage_title: read_string_field(payload, "flowStageTitle"),
            flow_stage_index: read_positive_i64_field(payload, "flowStageIndex"),
            flow_stage_total: read_positive_i64_field(payload, "flowStageTotal"),
        }
    }

    fn apply(&self, payload: &mut Map<String, Value>) {
        if should_fill(payload, "flowId")
            && let Some(value) = &self.flow_id
        {
            payload.insert("flowId".to_string(), Value::String(value.clone()));
        }
        if should_fill(payload, "flowStageTitle")
            && let Some(value) = &self.flow_stage_title
        {
            payload.insert("flowStageTitle".to_string(), Value::String(value.clone()));
        }
        if should_fill(payload, "flowStageIndex")
            && let Some(value) = self.flow_stage_index
        {
            payload.insert("flowStageIndex".to_string(), Value::from(value));
        }
        if should_fill(payload, "flowStageTotal")
            && let Some(value) = self.flow_stage_total
        {
            payload.insert("flowStageTotal".to_string(), Value::from(value));
        }
    }
}

fn read_string_field(payload: &Map<String, Value>, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn read_positive_i64_field(payload: &Map<String, Value>, key: &str) -> Option<i64> {
    let raw = payload.get(key)?;
    let value = if let Some(value) = raw.as_i64() {
        Some(value)
    } else {
        raw.as_str()
            .and_then(|item| item.trim().parse::<i64>().ok())
    }?;

    if value > 0 { Some(value) } else { None }
}

fn should_fill(payload: &Map<String, Value>, key: &str) -> bool {
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

fn create_ephemeral_id() -> String {
    format!("ephemeral:{}", Uuid::new_v4())
}

#[allow(dead_code)]
fn read_string_value(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

#[allow(dead_code)]
fn read_positive_u64(value: Option<&Value>) -> Option<u64> {
    let value = value?;
    if let Some(raw) = value.as_u64() {
        return Some(raw);
    }
    if let Some(raw) = value.as_i64() {
        return u64::try_from(raw).ok();
    }
    value
        .as_str()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
}

#[allow(dead_code)]
fn read_bool_value(value: Option<&Value>) -> Option<bool> {
    let value = value?;
    if let Some(flag) = value.as_bool() {
        return Some(flag);
    }
    value
        .as_str()
        .and_then(|raw| match raw.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
}

#[derive(Debug, Clone)]
pub struct TaskContext {
    task: Arc<WorkerTask>,
    mysql: MySqlPool,
    redis: RedisPool,
    queue: Arc<str>,
    flow: FlowFields,
    #[allow(dead_code)]
    stream_seq: Arc<AtomicU64>,
}

impl TaskContext {
    pub fn new(task: WorkerTask, mysql: MySqlPool, redis: RedisPool, queue: &str) -> Self {
        let flow = FlowFields::from_payload(&task.payload);
        Self {
            task: Arc::new(task),
            mysql,
            redis,
            queue: Arc::<str>::from(queue.trim().to_string()),
            flow,
            stream_seq: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn with_task(&self, task: WorkerTask) -> Self {
        let flow = FlowFields::from_payload(&task.payload);
        Self {
            task: Arc::new(task),
            mysql: self.mysql.clone(),
            redis: self.redis.clone(),
            queue: self.queue.clone(),
            flow,
            stream_seq: self.stream_seq.clone(),
        }
    }

    pub fn task(&self) -> &WorkerTask {
        self.task.as_ref()
    }

    pub fn queue(&self) -> &str {
        self.queue.as_ref()
    }

    fn task_billing_info(&self) -> Result<Option<TaskBillingInfo>, AppError> {
        parse_task_billing_info(self.task.billing_info.clone())
    }

    fn billing_info_bind(
        info: Option<&TaskBillingInfo>,
    ) -> Result<Option<sqlx::types::Json<Value>>, AppError> {
        Ok(serialize_task_billing_info(info)?.map(sqlx::types::Json))
    }

    pub fn should_retry(&self, error: &AppError) -> bool {
        error.code.spec().retryable && self.task.attempt < self.task.max_attempts
    }

    pub fn failed_attempt(&self) -> i32 {
        self.task.attempt
    }

    pub fn max_attempts(&self) -> i32 {
        self.task.max_attempts
    }

    pub async fn refresh_heartbeat(&self) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE tasks SET heartbeatAt = NOW(3), updatedAt = NOW(3) WHERE id = ? AND status = 'processing'",
        )
        .bind(&self.task.task_id)
        .execute(&self.mysql)
        .await?;
        Ok(())
    }

    pub async fn report_progress(
        &self,
        progress: i32,
        message: Option<&str>,
    ) -> Result<bool, AppError> {
        let value = progress.clamp(0, 99);
        let updated = sqlx::query(
            "UPDATE tasks SET progress = ?, updatedAt = NOW(3) WHERE id = ? AND status = 'processing'",
        )
        .bind(value)
        .bind(&self.task.task_id)
        .execute(&self.mysql)
        .await?;

        if updated.rows_affected() == 0 {
            return Ok(false);
        }

        let mut payload = self.base_payload("processing");
        payload.insert("progress".to_string(), Value::from(value));
        if let Some(message) = message
            .map(|item| item.trim())
            .filter(|item| !item.is_empty())
        {
            payload.insert("message".to_string(), Value::String(message.to_string()));
        }

        let payload_value = Value::Object(payload);
        let event_id = self
            .insert_task_event("task.progress", &payload_value)
            .await?;
        self.publish_lifecycle_event("task.progress", payload_value, Some(event_id))
            .await?;
        Ok(true)
    }

    #[allow(dead_code)]
    pub async fn report_stream_chunk(&self, chunk: Value) -> Result<(), AppError> {
        let payload = self.normalize_stream_payload(chunk)?;
        self.publish_stream_event(Value::Object(payload)).await
    }

    pub async fn mark_completed(&self, result: &Value) -> Result<bool, AppError> {
        let current_billing = self.task_billing_info()?;
        let settled_billing = match settle_task_billing(
            &self.mysql,
            &self.task.task_id,
            &self.task.user_id,
            &self.task.project_id,
            self.task.episode_id.as_deref(),
            current_billing.as_ref(),
        )
        .await
        {
            Ok(value) => value,
            Err(error) => {
                let _ = self.mark_failed(&error).await?;
                return Ok(false);
            }
        };
        let billed = settled_billing
            .as_ref()
            .is_some_and(|info| info.billable && info.status == Some(BillingStatus::Settled));
        let billing_info = Self::billing_info_bind(settled_billing.as_ref())?;

        let updated = sqlx::query(
            "UPDATE tasks SET status = 'completed', progress = 100, result = ?, errorCode = NULL, errorMessage = NULL, billingInfo = ?, billedAt = CASE WHEN ? THEN COALESCE(billedAt, NOW(3)) ELSE billedAt END, dedupeKey = NULL, finishedAt = NOW(3), heartbeatAt = NULL, updatedAt = NOW(3) WHERE id = ? AND status = 'processing'",
        )
        .bind(sqlx::types::Json(result.clone()))
        .bind(billing_info)
        .bind(billed)
        .bind(&self.task.task_id)
        .execute(&self.mysql)
        .await?;
        if updated.rows_affected() == 0 {
            return Ok(false);
        }

        let mut payload = self.base_payload("completed");
        payload.insert("result".to_string(), result.clone());
        let payload_value = Value::Object(payload);
        let event_id = self
            .insert_task_event("task.completed", &payload_value)
            .await?;
        self.publish_lifecycle_event("task.completed", payload_value, Some(event_id))
            .await?;
        Ok(true)
    }

    pub async fn mark_failed(&self, error: &AppError) -> Result<bool, AppError> {
        let code = error.code.as_str().to_string();
        let mut message = error.message.clone();

        let current_billing = self.task_billing_info()?;
        let next_billing = match rollback_task_billing(&self.mysql, current_billing.as_ref()).await
        {
            Ok(value) => value,
            Err(rollback_error) => {
                message = format!("{}; billing rollback failed: {}", message, rollback_error);
                current_billing.map(|mut info| {
                    info.status = Some(BillingStatus::Failed);
                    info
                })
            }
        };
        let billing_info = Self::billing_info_bind(next_billing.as_ref())?;

        let updated = sqlx::query(
            "UPDATE tasks SET status = 'failed', errorCode = ?, errorMessage = ?, billingInfo = ?, dedupeKey = NULL, finishedAt = NOW(3), heartbeatAt = NULL, updatedAt = NOW(3) WHERE id = ? AND status = 'processing'",
        )
        .bind(&code)
        .bind(&message)
        .bind(billing_info)
        .bind(&self.task.task_id)
        .execute(&self.mysql)
        .await?;
        if updated.rows_affected() == 0 {
            return Ok(false);
        }

        let mut payload = self.base_payload("failed");
        payload.insert("errorCode".to_string(), Value::String(code));
        payload.insert("errorMessage".to_string(), Value::String(message));
        let payload_value = Value::Object(payload);
        let event_id = self
            .insert_task_event("task.failed", &payload_value)
            .await?;
        self.publish_lifecycle_event("task.failed", payload_value, Some(event_id))
            .await?;
        Ok(true)
    }

    pub async fn requeue_for_retry(&self) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE tasks SET status = 'queued', heartbeatAt = NULL, errorCode = NULL, errorMessage = NULL, updatedAt = NOW(3) WHERE id = ? AND status = 'processing'",
        )
        .bind(&self.task.task_id)
        .execute(&self.mysql)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn publish_retry_scheduled(&self, error: &AppError) -> Result<(), AppError> {
        let mut payload = self.base_payload("processing");
        payload.insert("stage".to_string(), Value::String("retrying".to_string()));
        payload.insert(
            "retry".to_string(),
            json!({
                "failedAttempt": self.failed_attempt(),
                "maxAttempts": self.max_attempts(),
            }),
        );
        payload.insert(
            "error".to_string(),
            json!({
                "code": error.code.as_str(),
                "message": error.message,
                "retryable": error.code.spec().retryable,
            }),
        );
        payload.insert(
            "message".to_string(),
            Value::String(format!(
                "Retry scheduled ({}/{}): {}",
                self.failed_attempt(),
                self.max_attempts(),
                error.message
            )),
        );

        self.publish_lifecycle_event("task.progress", Value::Object(payload), None)
            .await
    }

    fn base_payload(&self, status: &str) -> Map<String, Value> {
        let mut payload = Map::new();
        payload.insert("status".to_string(), Value::String(status.to_string()));
        payload.insert("queue".to_string(), Value::String(self.queue().to_string()));
        payload.insert(
            "targetType".to_string(),
            Value::String(self.task.target_type.clone()),
        );
        payload.insert(
            "targetId".to_string(),
            Value::String(self.task.target_id.clone()),
        );
        payload.insert(
            "episodeId".to_string(),
            self.task
                .episode_id
                .as_ref()
                .map(|value| Value::String(value.clone()))
                .unwrap_or(Value::Null),
        );
        self.flow.apply(&mut payload);
        payload
    }

    #[allow(dead_code)]
    fn next_stream_seq(&self) -> u64 {
        self.stream_seq.fetch_add(1, Ordering::Relaxed) + 1
    }

    #[allow(dead_code)]
    fn bump_stream_seq(&self, value: u64) {
        let mut current = self.stream_seq.load(Ordering::Relaxed);
        while value > current {
            match self.stream_seq.compare_exchange_weak(
                current,
                value,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(next) => current = next,
            }
        }
    }

    #[allow(dead_code)]
    fn normalize_stream_payload(&self, chunk: Value) -> Result<Map<String, Value>, AppError> {
        let mut payload = chunk
            .as_object()
            .cloned()
            .ok_or_else(|| AppError::invalid_params("stream chunk payload must be an object"))?;

        let nested_stream = payload
            .get("stream")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        let kind = read_string_value(payload.get("kind"))
            .or_else(|| read_string_value(nested_stream.get("kind")))
            .unwrap_or_else(|| "text".to_string());
        let lane = read_string_value(payload.get("lane"))
            .or_else(|| read_string_value(nested_stream.get("lane")))
            .unwrap_or_else(|| "main".to_string());
        let done = read_bool_value(payload.get("done"))
            .or_else(|| read_bool_value(nested_stream.get("done")))
            .unwrap_or(false);
        let delta = read_string_value(payload.get("delta"))
            .or_else(|| read_string_value(nested_stream.get("delta")))
            .unwrap_or_default();

        if !done && delta.is_empty() {
            return Err(AppError::invalid_params(
                "stream chunk delta is required when done=false",
            ));
        }

        let stream_run_id = read_string_value(payload.get("streamRunId"))
            .or_else(|| read_string_value(nested_stream.get("streamRunId")));
        let seq = read_positive_u64(payload.get("seq"))
            .or_else(|| read_positive_u64(nested_stream.get("seq")))
            .unwrap_or_else(|| self.next_stream_seq());
        self.bump_stream_seq(seq);

        payload.insert("kind".to_string(), Value::String(kind.clone()));
        payload.insert("delta".to_string(), Value::String(delta.clone()));
        payload.insert("seq".to_string(), Value::from(seq));
        payload.insert("lane".to_string(), Value::String(lane.clone()));
        payload.insert("done".to_string(), Value::Bool(done));
        if let Some(stream_run_id) = stream_run_id.clone() {
            payload.insert("streamRunId".to_string(), Value::String(stream_run_id));
        }

        let mut stream = Map::new();
        stream.insert("kind".to_string(), Value::String(kind));
        stream.insert("delta".to_string(), Value::String(delta));
        stream.insert("seq".to_string(), Value::from(seq));
        stream.insert("lane".to_string(), Value::String(lane));
        stream.insert("done".to_string(), Value::Bool(done));
        if let Some(stream_run_id) = stream_run_id {
            stream.insert("streamRunId".to_string(), Value::String(stream_run_id));
        }
        payload.insert("stream".to_string(), Value::Object(stream));

        if should_fill(&payload, "queue") {
            payload.insert("queue".to_string(), Value::String(self.queue().to_string()));
        }
        if should_fill(&payload, "targetType") {
            payload.insert(
                "targetType".to_string(),
                Value::String(self.task.target_type.clone()),
            );
        }
        if should_fill(&payload, "targetId") {
            payload.insert(
                "targetId".to_string(),
                Value::String(self.task.target_id.clone()),
            );
        }
        if should_fill(&payload, "episodeId") {
            payload.insert(
                "episodeId".to_string(),
                self.task
                    .episode_id
                    .as_ref()
                    .map(|value| Value::String(value.clone()))
                    .unwrap_or(Value::Null),
            );
        }
        self.flow.apply(&mut payload);

        Ok(payload)
    }

    async fn insert_task_event(&self, event_type: &str, payload: &Value) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO task_events (taskId, projectId, userId, eventType, payload, createdAt) VALUES (?, ?, ?, ?, ?, NOW(3))",
        )
        .bind(&self.task.task_id)
        .bind(&self.task.project_id)
        .bind(&self.task.user_id)
        .bind(event_type)
        .bind(sqlx::types::Json(payload.clone()))
        .execute(&self.mysql)
        .await?;

        i64::try_from(result.last_insert_id())
            .map_err(|error| AppError::internal(format!("task event id overflow: {error}")))
    }

    async fn publish_lifecycle_event(
        &self,
        event_type: &str,
        payload: Value,
        event_id: Option<i64>,
    ) -> Result<(), AppError> {
        let payload = payload
            .as_object()
            .cloned()
            .ok_or_else(|| AppError::invalid_params("lifecycle payload must be an object"))?;
        let message = build_task_lifecycle_message(TaskLifecycleMessageInput {
            id: event_id
                .map(|value| value.to_string())
                .unwrap_or_else(create_ephemeral_id),
            event_type,
            task_id: &self.task.task_id,
            project_id: &self.task.project_id,
            user_id: &self.task.user_id,
            task_type: &self.task.task_type,
            target_type: &self.task.target_type,
            target_id: &self.task.target_id,
            episode_id: self.task.episode_id.as_deref(),
            payload: Value::Object(payload),
        });

        self.publish_message(&message).await?;
        self.publish_run_bridge_events("task.lifecycle", &message)
            .await
    }

    #[allow(dead_code)]
    async fn publish_stream_event(&self, payload: Value) -> Result<(), AppError> {
        let message = json!({
            "id": create_ephemeral_id(),
            "type": "task.stream",
            "eventType": "task.stream",
            "taskId": self.task.task_id,
            "projectId": self.task.project_id,
            "userId": self.task.user_id,
            "taskType": self.task.task_type,
            "targetType": self.task.target_type,
            "targetId": self.task.target_id,
            "episodeId": self.task.episode_id,
            "payload": payload,
            "ts": now_iso_string(),
        });

        self.publish_message(&message).await?;
        self.publish_run_bridge_events("task.stream", &message)
            .await
    }

    async fn publish_message(&self, message: &Value) -> Result<(), AppError> {
        publish_task_message(&self.redis, &self.task.project_id, message).await
    }

    async fn publish_run_bridge_events(
        &self,
        event_name: &str,
        payload: &Value,
    ) -> Result<(), AppError> {
        let event = TaskSseEvent {
            event_name: event_name.to_string(),
            project_id: self.task.project_id.clone(),
            user_id: self.task.user_id.clone(),
            task_type: Some(self.task.task_type.clone()),
            payload: payload.clone(),
        };

        for input in map_task_sse_event_to_run_events(&event) {
            let _ = publish_run_event(&self.mysql, &self.redis, &input).await?;
        }

        Ok(())
    }
}

impl Deref for TaskContext {
    type Target = WorkerTask;

    fn deref(&self) -> &Self::Target {
        self.task()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{FlowFields, should_fill};

    #[test]
    fn flow_fields_extracts_top_level_payload_values() {
        let flow = FlowFields::from_payload(&json!({
            "flowId": "flow-1",
            "flowStageTitle": "Draft",
            "flowStageIndex": "2",
            "flowStageTotal": 5,
        }));

        assert_eq!(flow.flow_id.as_deref(), Some("flow-1"));
        assert_eq!(flow.flow_stage_title.as_deref(), Some("Draft"));
        assert_eq!(flow.flow_stage_index, Some(2));
        assert_eq!(flow.flow_stage_total, Some(5));
    }

    #[test]
    fn flow_fields_apply_only_fills_missing_values() {
        let flow = FlowFields::from_payload(&json!({
            "flowId": "flow-2",
            "flowStageTitle": "Stage",
            "flowStageIndex": 3,
            "flowStageTotal": 4,
        }));

        let mut payload = json!({
            "flowId": "existing",
            "flowStageTitle": "",
            "flowStageIndex": null,
            "flowStageTotal": 1,
        })
        .as_object()
        .cloned()
        .expect("payload object");
        flow.apply(&mut payload);

        assert_eq!(payload.get("flowId"), Some(&json!("existing")));
        assert_eq!(payload.get("flowStageTitle"), Some(&json!("Stage")));
        assert_eq!(payload.get("flowStageIndex"), Some(&json!(3)));
        assert_eq!(payload.get("flowStageTotal"), Some(&json!(1)));
    }

    #[test]
    fn should_fill_accepts_missing_null_and_empty_string() {
        let payload = json!({
            "nullValue": null,
            "empty": "  ",
            "filled": "value",
            "number": 1,
        })
        .as_object()
        .cloned()
        .expect("payload object");

        assert!(should_fill(&payload, "missing"));
        assert!(should_fill(&payload, "nullValue"));
        assert!(should_fill(&payload, "empty"));
        assert!(!should_fill(&payload, "filled"));
        assert!(!should_fill(&payload, "number"));
    }

    #[test]
    fn task_context_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<super::TaskContext>();
    }
}
