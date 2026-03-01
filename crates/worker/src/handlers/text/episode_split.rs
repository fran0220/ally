use serde_json::{Value, json};
use waoowaoo_core::errors::AppError;
use waoowaoo_core::prompt_i18n::{PromptIds, PromptVariables};

use crate::task_context::TaskContext;

use super::shared;

const MAX_EPISODE_SPLIT_ATTEMPTS: usize = 2;
const EPISODE_SPLIT_BOUNDARY_SUFFIX: &str = "\n\n[Boundary Constraints]\n1. Each episode MUST include both startMarker and endMarker from the original text.\n2. Markers must be locatable in the original text; allow punctuation/whitespace differences only.\n3. If boundaries cannot be located reliably, return an empty episodes array.";

fn read_boundary_marker(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn read_boundary_index(value: Option<&Value>, content_len: usize) -> Option<usize> {
    let raw = value.and_then(|item| {
        item.as_i64().or_else(|| {
            item.as_str()
                .and_then(|item| item.trim().parse::<i64>().ok())
        })
    })?;
    let normalized = usize::try_from(raw).ok()?;
    if normalized > content_len {
        return None;
    }
    Some(normalized)
}

fn parse_split_response(raw: &str) -> Result<Vec<Value>, AppError> {
    if let Ok(parsed) = shared::parse_json_object_response(raw)
        && let Some(items) = parsed.get("episodes").and_then(Value::as_array)
    {
        return Ok(items.clone());
    }
    shared::parse_json_array_response(raw)
}

fn resolve_episode_chunks(content: &str, episodes: &[Value]) -> Result<Vec<Value>, AppError> {
    let mut resolved = Vec::with_capacity(episodes.len());
    let mut search_from = 0usize;

    for (index, item) in episodes.iter().enumerate() {
        let episode_number = item
            .get("number")
            .and_then(|item| item.as_i64())
            .and_then(|item| i32::try_from(item).ok())
            .filter(|item| *item > 0)
            .ok_or_else(|| {
                AppError::invalid_params(format!("episode_{} missing valid number", index + 1))
            })?;

        let title = item
            .get("title")
            .and_then(Value::as_str)
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .ok_or_else(|| {
                AppError::invalid_params(format!("episode_{} missing title", index + 1))
            })?;

        let start_marker = read_boundary_marker(item.get("startMarker")).ok_or_else(|| {
            AppError::invalid_params(format!("episode_{} must include startMarker", index + 1))
        })?;
        let end_marker = read_boundary_marker(item.get("endMarker")).ok_or_else(|| {
            AppError::invalid_params(format!("episode_{} must include endMarker", index + 1))
        })?;

        let (start_index, start_end) =
            shared::match_text_marker(content, &start_marker, search_from).ok_or_else(|| {
                AppError::invalid_params(format!(
                    "episode_{} startMarker cannot be located",
                    index + 1
                ))
            })?;
        let (_, end_index) = shared::match_text_marker(content, &end_marker, start_end)
            .ok_or_else(|| {
                AppError::invalid_params(format!(
                    "episode_{} endMarker cannot be located",
                    index + 1
                ))
            })?;

        if let Some(raw_start_index) = read_boundary_index(item.get("startIndex"), content.len())
            && raw_start_index.abs_diff(start_index) > 200
        {
            return Err(AppError::invalid_params(format!(
                "episode_{} startIndex mismatches marker",
                index + 1
            )));
        }
        if let Some(raw_end_index) = read_boundary_index(item.get("endIndex"), content.len())
            && raw_end_index.abs_diff(end_index) > 200
        {
            return Err(AppError::invalid_params(format!(
                "episode_{} endIndex mismatches marker",
                index + 1
            )));
        }

        if start_index < search_from || end_index <= start_index || end_index > content.len() {
            return Err(AppError::invalid_params(format!(
                "episode_{} has invalid boundary range",
                index + 1
            )));
        }

        let episode_content = content
            .get(start_index..end_index)
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(|item| item.to_string())
            .ok_or_else(|| {
                AppError::invalid_params(format!("episode_{} matched content is empty", index + 1))
            })?;
        let summary = item
            .get("summary")
            .and_then(Value::as_str)
            .map(|item| item.trim().to_string())
            .unwrap_or_default();

        resolved.push(json!({
            "number": episode_number,
            "title": title,
            "summary": summary,
            "content": episode_content,
            "wordCount": shared::count_words_like_word(&episode_content),
        }));
        search_from = end_index;
    }

    Ok(resolved)
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    let payload = &task.payload;
    let content = shared::read_string(payload, "content")
        .ok_or_else(|| AppError::invalid_params("content is required"))?;
    if content.chars().count() < 100 {
        return Err(AppError::invalid_params(
            "content is too short, at least 100 characters",
        ));
    }

    let analysis_model = shared::resolve_analysis_model(task, payload).await?;
    let mut prompt_variables = PromptVariables::new();
    prompt_variables.insert("CONTENT".to_string(), content.clone());
    let prompt_base =
        shared::render_prompt_template(payload, PromptIds::NP_EPISODE_SPLIT, &prompt_variables)?;
    let prompt = format!("{prompt_base}{EPISODE_SPLIT_BOUNDARY_SUFFIX}");

    let _ = task
        .report_progress(20, Some("progress.stage.episodeSplitPrepare"))
        .await?;

    let run_id = shared::create_stream_run_id(task, "episode_split");
    let mut resolved_episodes: Option<Vec<Value>> = None;
    let mut last_error: Option<AppError> = None;

    for attempt in 1..=MAX_EPISODE_SPLIT_ATTEMPTS {
        let meta = shared::LlmStepMeta {
            run_id: Some(run_id.clone()),
            stream_run_id: Some(run_id.clone()),
            step_id: Some(if attempt == 1 {
                "episode_split".to_string()
            } else {
                format!("episode_split_retry_{attempt}")
            }),
            step_title: Some("智能分集".to_string()),
            step_attempt: Some(attempt as i32),
            step_index: Some(1),
            step_total: Some(1),
        };

        let response = match shared::chat_with_step(task, &analysis_model, &prompt, &meta).await {
            Ok(value) => value,
            Err(err) => {
                last_error = Some(err);
                continue;
            }
        };

        let _ = task
            .report_progress(60, Some("progress.stage.episodeSplitParse"))
            .await?;

        let split_episodes = match parse_split_response(&response) {
            Ok(items) if !items.is_empty() => items,
            Ok(_) => {
                last_error = Some(AppError::invalid_params(
                    "episode split returned empty episodes",
                ));
                continue;
            }
            Err(err) => {
                last_error = Some(err);
                continue;
            }
        };

        let _ = task
            .report_progress(80, Some("progress.stage.episodeSplitMatch"))
            .await?;

        match resolve_episode_chunks(&content, &split_episodes) {
            Ok(items) if !items.is_empty() => {
                resolved_episodes = Some(items);
                break;
            }
            Ok(_) => {
                last_error = Some(AppError::invalid_params(
                    "episode split returned empty episodes",
                ));
            }
            Err(err) => {
                last_error = Some(err);
            }
        }
    }

    let episodes = resolved_episodes.ok_or_else(|| {
        last_error.unwrap_or_else(|| AppError::invalid_params("episode split failed"))
    })?;

    let _ = task
        .report_progress(96, Some("progress.stage.episodeSplitDone"))
        .await?;

    Ok(json!({
        "success": true,
        "episodes": episodes,
        "model": analysis_model,
    }))
}
