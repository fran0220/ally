use serde_json::{Map, Value};

use super::types::{RunEventInput, RunEventLane, RunEventType};

#[derive(Debug, Clone)]
pub struct TaskSseEvent {
    pub event_name: String,
    pub project_id: String,
    pub user_id: String,
    pub task_type: Option<String>,
    pub payload: Value,
}

fn to_object(value: &Value) -> Option<&Map<String, Value>> {
    value.as_object()
}

fn read_string(payload: &Map<String, Value>, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn read_i32(payload: &Map<String, Value>, key: &str) -> Option<i32> {
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
        .map(|value| value.max(1))
}

fn resolve_payload_scope(root: &Map<String, Value>) -> Map<String, Value> {
    root.get("payload")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(|| root.clone())
}

fn resolve_run_id(root: &Map<String, Value>, scoped: &Map<String, Value>) -> Option<String> {
    read_string(root, "runId")
        .or_else(|| read_string(scoped, "runId"))
        .or_else(|| {
            root.get("meta")
                .and_then(Value::as_object)
                .and_then(|meta| read_string(meta, "runId"))
        })
        .or_else(|| {
            scoped
                .get("meta")
                .and_then(Value::as_object)
                .and_then(|meta| read_string(meta, "runId"))
        })
}

fn resolve_step_key(scoped: &Map<String, Value>) -> Option<String> {
    read_string(scoped, "stepKey").or_else(|| read_string(scoped, "stepId"))
}

fn normalize_lifecycle_type(raw: Option<String>) -> Option<&'static str> {
    match raw?.trim().to_ascii_lowercase().as_str() {
        "task.progress" => Some("task.processing"),
        "task.created" => Some("task.created"),
        "task.processing" => Some("task.processing"),
        "task.completed" => Some("task.completed"),
        "task.failed" => Some("task.failed"),
        _ => None,
    }
}

fn stage_completed(stage: Option<String>) -> bool {
    matches!(
        stage.as_deref(),
        Some("llm_completed")
            | Some("worker_llm_completed")
            | Some("worker_llm_complete")
            | Some("llm_proxy_persist")
            | Some("completed")
    )
}

fn stage_failed(stage: Option<String>) -> bool {
    matches!(
        stage.as_deref(),
        Some("llm_error") | Some("worker_llm_error") | Some("error")
    )
}

pub fn map_task_sse_event_to_run_events(event: &TaskSseEvent) -> Vec<RunEventInput> {
    let Some(root) = to_object(&event.payload) else {
        return Vec::new();
    };
    let scoped = resolve_payload_scope(root);
    let Some(run_id) = resolve_run_id(root, &scoped) else {
        return Vec::new();
    };

    if event.event_name == "task.stream" {
        let stream = scoped
            .get("stream")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let delta = read_string(&stream, "delta");
        if delta.is_none() {
            return Vec::new();
        }

        let step_key = resolve_step_key(&scoped).or_else(|| {
            event
                .task_type
                .as_ref()
                .map(|task_type| format!("step:{task_type}"))
        });
        let Some(step_key) = step_key else {
            return Vec::new();
        };
        let lane_value = read_string(&stream, "lane");
        let kind_value = read_string(&stream, "kind");
        let lane = if lane_value.as_deref() == Some("reasoning")
            || kind_value.as_deref() == Some("reasoning")
        {
            Some(RunEventLane::Reasoning)
        } else {
            Some(RunEventLane::Text)
        };

        return vec![RunEventInput {
            run_id,
            project_id: event.project_id.clone(),
            user_id: event.user_id.clone(),
            event_type: RunEventType::StepChunk,
            step_key: Some(step_key),
            attempt: read_i32(&scoped, "stepAttempt").or_else(|| read_i32(&scoped, "attempt")),
            lane,
            payload: Some(Value::Object(root.clone())),
        }];
    }

    let lifecycle_type = normalize_lifecycle_type(read_string(&scoped, "lifecycleType"));
    let Some(lifecycle_type) = lifecycle_type else {
        return Vec::new();
    };

    let step_key = resolve_step_key(&scoped);
    let attempt = read_i32(&scoped, "stepAttempt").or_else(|| read_i32(&scoped, "attempt"));
    let payload = Value::Object(root.clone());

    if lifecycle_type == "task.created" {
        return vec![RunEventInput {
            run_id,
            project_id: event.project_id.clone(),
            user_id: event.user_id.clone(),
            event_type: RunEventType::RunStart,
            step_key: None,
            attempt: None,
            lane: None,
            payload: Some(payload),
        }];
    }

    if lifecycle_type == "task.processing" {
        let Some(step_key) = step_key else {
            return Vec::new();
        };
        let mut events = vec![RunEventInput {
            run_id: run_id.clone(),
            project_id: event.project_id.clone(),
            user_id: event.user_id.clone(),
            event_type: RunEventType::StepStart,
            step_key: Some(step_key.clone()),
            attempt,
            lane: None,
            payload: Some(payload.clone()),
        }];

        let stage = read_string(&scoped, "stage");
        let done = scoped.get("done").and_then(Value::as_bool).unwrap_or(false);
        let has_error = scoped
            .get("error")
            .and_then(Value::as_object)
            .map(|error| !error.is_empty())
            .unwrap_or(false);

        if done || stage_completed(stage.clone()) {
            events.push(RunEventInput {
                run_id,
                project_id: event.project_id.clone(),
                user_id: event.user_id.clone(),
                event_type: RunEventType::StepComplete,
                step_key: Some(step_key),
                attempt,
                lane: None,
                payload: Some(payload),
            });
            return events;
        }

        if stage_failed(stage) || has_error {
            events.push(RunEventInput {
                run_id,
                project_id: event.project_id.clone(),
                user_id: event.user_id.clone(),
                event_type: RunEventType::StepError,
                step_key: Some(step_key),
                attempt,
                lane: None,
                payload: Some(payload),
            });
        }

        return events;
    }

    if lifecycle_type == "task.completed" {
        let mut events = Vec::new();
        if let Some(step_key) = step_key {
            events.push(RunEventInput {
                run_id: run_id.clone(),
                project_id: event.project_id.clone(),
                user_id: event.user_id.clone(),
                event_type: RunEventType::StepComplete,
                step_key: Some(step_key),
                attempt,
                lane: None,
                payload: Some(payload.clone()),
            });
        }
        events.push(RunEventInput {
            run_id,
            project_id: event.project_id.clone(),
            user_id: event.user_id.clone(),
            event_type: RunEventType::RunComplete,
            step_key: None,
            attempt: None,
            lane: None,
            payload: Some(payload),
        });
        return events;
    }

    if lifecycle_type == "task.failed" {
        let mut events = Vec::new();
        if let Some(step_key) = step_key {
            events.push(RunEventInput {
                run_id: run_id.clone(),
                project_id: event.project_id.clone(),
                user_id: event.user_id.clone(),
                event_type: RunEventType::StepError,
                step_key: Some(step_key),
                attempt,
                lane: None,
                payload: Some(payload.clone()),
            });
        }
        events.push(RunEventInput {
            run_id,
            project_id: event.project_id.clone(),
            user_id: event.user_id.clone(),
            event_type: RunEventType::RunError,
            step_key: None,
            attempt: None,
            lane: None,
            payload: Some(payload),
        });
        return events;
    }

    Vec::new()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn sample_event(event_name: &str, payload: Value) -> TaskSseEvent {
        TaskSseEvent {
            event_name: event_name.to_string(),
            project_id: "project-1".to_string(),
            user_id: "user-1".to_string(),
            task_type: Some("analyze_novel".to_string()),
            payload,
        }
    }

    #[test]
    fn stream_event_falls_back_to_task_type_step_key() {
        let event = sample_event(
            "task.stream",
            json!({
                "runId": "run-1",
                "stream": {
                    "delta": "hello"
                }
            }),
        );

        let mapped = map_task_sse_event_to_run_events(&event);
        assert_eq!(mapped.len(), 1);
        let first = &mapped[0];
        assert_eq!(first.event_type, RunEventType::StepChunk);
        assert_eq!(first.step_key.as_deref(), Some("step:analyze_novel"));
        assert_eq!(first.lane, Some(RunEventLane::Text));
    }

    #[test]
    fn stream_event_kind_reasoning_overrides_lane_text() {
        let event = sample_event(
            "task.stream",
            json!({
                "runId": "run-1",
                "stepKey": "step-a",
                "stream": {
                    "delta": "trace",
                    "lane": "text",
                    "kind": "reasoning"
                }
            }),
        );

        let mapped = map_task_sse_event_to_run_events(&event);
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].lane, Some(RunEventLane::Reasoning));
    }

    #[test]
    fn processing_without_step_key_returns_empty() {
        let event = sample_event(
            "task.lifecycle",
            json!({
                "runId": "run-1",
                "lifecycleType": "task.processing"
            }),
        );

        let mapped = map_task_sse_event_to_run_events(&event);
        assert!(mapped.is_empty());
    }

    #[test]
    fn progress_lifecycle_normalizes_to_processing() {
        let event = sample_event(
            "task.lifecycle",
            json!({
                "runId": "run-1",
                "lifecycleType": "task.progress",
                "stepKey": "step-a",
                "stage": "llm_completed"
            }),
        );

        let mapped = map_task_sse_event_to_run_events(&event);
        assert_eq!(mapped.len(), 2);
        assert_eq!(mapped[0].event_type, RunEventType::StepStart);
        assert_eq!(mapped[1].event_type, RunEventType::StepComplete);
    }

    #[test]
    fn created_lifecycle_emits_run_start() {
        let event = sample_event(
            "task.lifecycle",
            json!({
                "runId": "run-1",
                "lifecycleType": "task.created"
            }),
        );

        let mapped = map_task_sse_event_to_run_events(&event);
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].event_type, RunEventType::RunStart);
    }

    #[test]
    fn failed_lifecycle_emits_step_error_and_run_error() {
        let event = sample_event(
            "task.lifecycle",
            json!({
                "runId": "run-1",
                "lifecycleType": "task.failed",
                "stepKey": "step-a",
                "error": {
                    "message": "boom"
                }
            }),
        );

        let mapped = map_task_sse_event_to_run_events(&event);
        assert_eq!(mapped.len(), 2);
        assert_eq!(mapped[0].event_type, RunEventType::StepError);
        assert_eq!(mapped[1].event_type, RunEventType::RunError);
    }

    #[test]
    fn completed_without_step_key_only_emits_run_complete() {
        let event = sample_event(
            "task.lifecycle",
            json!({
                "runId": "run-1",
                "lifecycleType": "task.completed"
            }),
        );

        let mapped = map_task_sse_event_to_run_events(&event);
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].event_type, RunEventType::RunComplete);
        assert!(mapped[0].step_key.is_none());
    }
}
