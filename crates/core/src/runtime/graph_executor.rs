use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sqlx::MySqlPool;
use tokio::time::{Duration, sleep, timeout};

use crate::errors::{AppError, ErrorCode};

use super::{
    service::{build_lean_state, create_checkpoint, get_run_by_id},
    types::{RunStatus, StateRef},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphExecutorState {
    pub refs: StateRef,
    pub meta: Value,
}

impl Default for GraphExecutorState {
    fn default() -> Self {
        Self {
            refs: StateRef::default(),
            meta: json!({}),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GraphNodeResult {
    pub output: Option<Value>,
    pub checkpoint_refs: Option<StateRef>,
    pub checkpoint_meta: Option<Value>,
}

impl GraphNodeResult {
    pub fn empty() -> Self {
        Self {
            output: None,
            checkpoint_refs: None,
            checkpoint_meta: None,
        }
    }
}

pub struct GraphNodeContext<'a> {
    pub pool: &'a MySqlPool,
    pub run_id: &'a str,
    pub project_id: &'a str,
    pub user_id: &'a str,
    pub node_key: &'a str,
    pub attempt: u32,
    pub state: &'a mut GraphExecutorState,
}

#[async_trait]
pub trait GraphNodeRunner: Send + Sync {
    async fn run(&self, context: GraphNodeContext<'_>) -> Result<GraphNodeResult, AppError>;
}

pub struct GraphNode {
    pub key: String,
    pub title: String,
    pub max_attempts: u32,
    pub timeout_ms: Option<u64>,
    pub runner: Arc<dyn GraphNodeRunner>,
}

pub struct GraphExecutorInput {
    pub run_id: String,
    pub project_id: String,
    pub user_id: String,
    pub state: GraphExecutorState,
    pub nodes: Vec<GraphNode>,
}

fn merge_refs(base: &mut StateRef, next: Option<StateRef>) {
    let Some(next) = next else {
        return;
    };

    fn merge_ref_value(base: &mut Option<String>, next: Option<String>) {
        let Some(value) = next else {
            return;
        };

        // Keep parity with the TypeScript `next.xxx || base.xxx` behavior.
        if value.is_empty() {
            return;
        }
        *base = Some(value);
    }

    merge_ref_value(&mut base.script_id, next.script_id);
    merge_ref_value(&mut base.storyboard_id, next.storyboard_id);
    merge_ref_value(&mut base.voice_line_batch_id, next.voice_line_batch_id);
    merge_ref_value(&mut base.version_hash, next.version_hash);
    merge_ref_value(&mut base.cursor, next.cursor);
}

fn merge_meta(base: &mut Value, patch: Option<Value>) {
    let Some(Value::Object(patch_map)) = patch else {
        return;
    };

    if !base.is_object() {
        *base = Value::Object(Map::new());
    }
    let Some(base_map) = base.as_object_mut() else {
        return;
    };
    for (key, value) in patch_map {
        base_map.insert(key, value);
    }
}

fn build_checkpoint_meta(
    state_meta: &Value,
    node_title: &str,
    attempt: u32,
    output: Option<Value>,
) -> Value {
    let mut map = match state_meta {
        Value::Object(meta) => meta.clone(),
        _ => Map::new(),
    };

    map.insert(
        "nodeTitle".to_string(),
        Value::String(node_title.to_string()),
    );
    map.insert(
        "nodeAttempt".to_string(),
        Value::Number(i64::from(attempt).into()),
    );
    if let Some(value) = output {
        map.insert("output".to_string(), value);
    }

    Value::Object(map)
}

fn is_run_canceled(status: RunStatus) -> bool {
    matches!(status, RunStatus::Canceling | RunStatus::Canceled)
}

async fn assert_run_active(pool: &MySqlPool, run_id: &str, user_id: &str) -> Result<(), AppError> {
    let run = get_run_by_id(pool, run_id).await?;
    let Some(run) = run else {
        return Err(AppError::not_found("run not found"));
    };
    if run.user_id != user_id {
        return Err(AppError::not_found("run not found"));
    }
    if is_run_canceled(run.status) {
        return Err(AppError::conflict("run canceled"));
    }
    Ok(())
}

async fn run_node_with_timeout(
    node: &GraphNode,
    context: GraphNodeContext<'_>,
) -> Result<GraphNodeResult, AppError> {
    let fut = node.runner.run(context);

    if let Some(timeout_ms) = node.timeout_ms
        && timeout_ms > 0
    {
        return timeout(Duration::from_millis(timeout_ms), fut)
            .await
            .map_err(|_| {
                AppError::new(
                    ErrorCode::GenerationTimeout,
                    format!("graph node {} timeout after {timeout_ms}ms", node.key),
                )
            })?;
    }

    fut.await
}

fn compute_backoff_ms(attempt: u32) -> u64 {
    let exp = attempt.saturating_sub(1).min(4);
    let base = 1_000_u64.saturating_mul(2_u64.saturating_pow(exp));
    let jitter = u64::from((attempt * 37) % 200);
    base.min(10_000) + jitter
}

pub async fn execute_pipeline_graph(
    pool: &MySqlPool,
    mut input: GraphExecutorInput,
) -> Result<GraphExecutorState, AppError> {
    for node in &input.nodes {
        let max_attempts = node.max_attempts.max(1);
        let mut attempt = 1_u32;

        loop {
            assert_run_active(pool, &input.run_id, &input.user_id).await?;

            let result = run_node_with_timeout(
                node,
                GraphNodeContext {
                    pool,
                    run_id: &input.run_id,
                    project_id: &input.project_id,
                    user_id: &input.user_id,
                    node_key: &node.key,
                    attempt,
                    state: &mut input.state,
                },
            )
            .await;

            match result {
                Ok(result) => {
                    let GraphNodeResult {
                        output,
                        checkpoint_refs,
                        checkpoint_meta,
                    } = result;

                    merge_refs(&mut input.state.refs, checkpoint_refs);
                    merge_meta(&mut input.state.meta, checkpoint_meta);

                    let checkpoint_meta =
                        build_checkpoint_meta(&input.state.meta, &node.title, attempt, output);
                    let checkpoint_version = i32::try_from(attempt).map_err(|error| {
                        AppError::internal(format!("checkpoint version overflow: {error}"))
                    })?;

                    let lean_state = build_lean_state(&input.state.refs, Some(checkpoint_meta));
                    create_checkpoint(
                        pool,
                        &input.run_id,
                        &node.key,
                        checkpoint_version,
                        &lean_state,
                    )
                    .await?;

                    break;
                }
                Err(error) => {
                    let should_retry = error.code.spec().retryable && attempt < max_attempts;
                    if !should_retry {
                        return Err(error);
                    }

                    sleep(Duration::from_millis(compute_backoff_ms(attempt))).await;
                    attempt = attempt.saturating_add(1);
                }
            }
        }
    }

    Ok(input.state)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn merge_refs_ignores_empty_string_values() {
        let mut base = StateRef {
            script_id: Some("script-1".to_string()),
            storyboard_id: None,
            voice_line_batch_id: Some("voice-1".to_string()),
            version_hash: None,
            cursor: None,
        };

        let patch = StateRef {
            script_id: Some(String::new()),
            storyboard_id: Some("storyboard-2".to_string()),
            voice_line_batch_id: Some(String::new()),
            version_hash: Some("hash-2".to_string()),
            cursor: None,
        };

        merge_refs(&mut base, Some(patch));

        assert_eq!(base.script_id.as_deref(), Some("script-1"));
        assert_eq!(base.storyboard_id.as_deref(), Some("storyboard-2"));
        assert_eq!(base.voice_line_batch_id.as_deref(), Some("voice-1"));
        assert_eq!(base.version_hash.as_deref(), Some("hash-2"));
    }

    #[test]
    fn build_checkpoint_meta_overrides_runtime_fields() {
        let state_meta = json!({
            "nodeTitle": "stale title",
            "nodeAttempt": 1,
            "output": {"stale": true},
            "trace": "keep-me"
        });

        let meta =
            build_checkpoint_meta(&state_meta, "Fresh Node", 3, Some(json!({"fresh": true})));

        assert_eq!(meta.get("trace"), Some(&json!("keep-me")));
        assert_eq!(meta.get("nodeTitle"), Some(&json!("Fresh Node")));
        assert_eq!(meta.get("nodeAttempt"), Some(&json!(3)));
        assert_eq!(meta.get("output"), Some(&json!({"fresh": true})));
    }
}
