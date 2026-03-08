use serde_json::{Value, json};
use sqlx::FromRow;
use waoowaoo_core::errors::AppError;
use waoowaoo_core::prompt_i18n::{PromptIds, PromptVariables};

use crate::{runtime, task_context::TaskContext};

use super::shared;

const MAX_SPLIT_BOUNDARY_ATTEMPTS: usize = 2;
const CLIP_BOUNDARY_SUFFIX: &str = "\n\n[Boundary Constraints]\n1. The \"start\" and \"end\" anchors must come from the original text and be locatable.\n2. Allow punctuation/whitespace differences, but do not rewrite key entities or events.\n3. If anchors cannot be located reliably, return [] directly.";

#[derive(Debug, FromRow)]
struct EpisodeRow {
    id: String,
    #[sqlx(rename = "novelPromotionProjectId")]
    novel_promotion_project_id: String,
    #[sqlx(rename = "novelText")]
    novel_text: Option<String>,
}

#[derive(Debug, FromRow)]
struct CharacterRow {
    name: String,
    introduction: Option<String>,
}

#[derive(Debug, FromRow)]
struct LocationRow {
    name: String,
}

#[derive(Debug, Clone)]
struct ResolvedClip {
    start_text: String,
    end_text: String,
    summary: String,
    location: Option<String>,
    characters_json: Option<String>,
    content: String,
}

fn read_optional_text(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn extract_clips(response: &str) -> Result<Vec<Value>, AppError> {
    if let Ok(object) = shared::parse_json_object_response(response)
        && let Some(items) = object.get("clips").and_then(Value::as_array)
    {
        return Ok(items.clone());
    }
    shared::parse_json_array_response(response)
}

fn resolve_clips_by_boundary(
    content: &str,
    clips: &[Value],
) -> Result<Vec<ResolvedClip>, AppError> {
    let mut resolved = Vec::with_capacity(clips.len());
    let mut search_from = 0usize;

    for (index, item) in clips.iter().enumerate() {
        let start_text = read_optional_text(item.get("start"))
            .or_else(|| read_optional_text(item.get("startText")))
            .ok_or_else(|| {
                AppError::invalid_params(format!(
                    "split_clips boundary matching failed at clip_{}: missing start",
                    index + 1
                ))
            })?;
        let end_text = read_optional_text(item.get("end"))
            .or_else(|| read_optional_text(item.get("endText")))
            .ok_or_else(|| {
                AppError::invalid_params(format!(
                    "split_clips boundary matching failed at clip_{}: missing end",
                    index + 1
                ))
            })?;

        let Some((start_index, end_index)) =
            shared::match_text_boundary(content, &start_text, &end_text, search_from)
        else {
            return Err(AppError::invalid_params(format!(
                "split_clips boundary matching failed at clip_{}: start=\"{}\" end=\"{}\"",
                index + 1,
                start_text,
                end_text
            )));
        };

        let matched_content = content
            .get(start_index..end_index)
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(|item| item.to_string())
            .ok_or_else(|| {
                AppError::invalid_params(format!(
                    "split_clips boundary matching failed at clip_{}: empty content",
                    index + 1
                ))
            })?;

        let summary = read_optional_text(item.get("summary")).unwrap_or_else(|| "clip".to_string());
        let location = read_optional_text(item.get("location"));
        let characters_json = item.get("characters").map(Value::to_string);

        resolved.push(ResolvedClip {
            start_text,
            end_text,
            summary,
            location,
            characters_json,
            content: matched_content,
        });
        search_from = end_index;
    }

    Ok(resolved)
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    shared::ensure_novel_project(task).await?;
    let mysql = runtime::mysql()?;
    let payload = &task.payload;
    let locale = shared::resolve_prompt_locale(payload);
    let list_separator = shared::l(locale, "、", ", ");
    let none_text = shared::l(locale, "无", "None");

    let episode_id = shared::read_episode_id(task)
        .ok_or_else(|| AppError::invalid_params("episodeId is required"))?;
    let novel_project = shared::get_novel_project(task).await?;
    let analysis_model = shared::resolve_analysis_model(task, payload).await?;

    let episode = sqlx::query_as::<_, EpisodeRow>(
        "SELECT id, novelPromotionProjectId, novelText FROM novel_promotion_episodes WHERE id = ? LIMIT 1",
    )
    .bind(&episode_id)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("episode not found"))?;
    if episode.novel_promotion_project_id != novel_project.id {
        return Err(AppError::invalid_params(
            "episode does not belong to current project",
        ));
    }

    let content_to_process = episode
        .novel_text
        .as_deref()
        .map(str::trim)
        .map(|item| item.to_string())
        .filter(|item| !item.is_empty())
        .ok_or_else(|| AppError::invalid_params("episode novel text is empty"))?;

    let characters = sqlx::query_as::<_, CharacterRow>(
        "SELECT name, introduction FROM novel_promotion_characters WHERE novelPromotionProjectId = ? ORDER BY createdAt ASC",
    )
    .bind(&novel_project.id)
    .fetch_all(mysql)
    .await?;
    let locations = sqlx::query_as::<_, LocationRow>(
        "SELECT name FROM novel_promotion_locations WHERE novelPromotionProjectId = ? ORDER BY createdAt ASC",
    )
    .bind(&novel_project.id)
    .fetch_all(mysql)
    .await?;

    let locations_lib_name = if locations.is_empty() {
        none_text.to_string()
    } else {
        locations
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>()
            .join(list_separator)
    };
    let characters_lib_name = if characters.is_empty() {
        none_text.to_string()
    } else {
        characters
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>()
            .join(list_separator)
    };
    let characters_introduction = shared::build_characters_introduction(
        &characters
            .iter()
            .map(|item| (item.name.clone(), item.introduction.clone()))
            .collect::<Vec<_>>(),
        locale,
    );

    let mut prompt_variables = PromptVariables::new();
    prompt_variables.insert("input".to_string(), content_to_process.clone());
    prompt_variables.insert("locations_lib_name".to_string(), locations_lib_name);
    prompt_variables.insert("characters_lib_name".to_string(), characters_lib_name);
    prompt_variables.insert(
        "characters_introduction".to_string(),
        characters_introduction,
    );
    let prompt_base =
        shared::render_prompt_template(payload, PromptIds::NP_AGENT_CLIP, &prompt_variables)?;
    let prompt = format!("{prompt_base}{CLIP_BOUNDARY_SUFFIX}");

    let _ = task
        .report_progress(20, Some("progress.stage.clipsBuildPrepare"))
        .await?;

    let run_id = shared::create_stream_run_id(task, "clips_build");
    let mut resolved_clips = Vec::new();
    let mut last_error: Option<AppError> = None;

    for attempt in 1..=MAX_SPLIT_BOUNDARY_ATTEMPTS {
        let meta = shared::LlmStepMeta {
            run_id: Some(run_id.clone()),
            stream_run_id: Some(run_id.clone()),
            step_id: Some(if attempt == 1 {
                "split_clips".to_string()
            } else {
                format!("split_clips_retry_{attempt}")
            }),
            step_title: Some(shared::l(locale, "片段切分", "Clip Segmentation").to_string()),
            step_attempt: Some(attempt as i32),
            step_index: Some(1),
            step_total: Some(1),
        };

        let response = shared::chat_with_step(task, &analysis_model, &prompt, &meta).await?;
        let clips = match extract_clips(&response) {
            Ok(items) if !items.is_empty() => items,
            Ok(_) => {
                last_error = Some(AppError::invalid_params("clips_build produced no clips"));
                continue;
            }
            Err(err) => {
                last_error = Some(err);
                continue;
            }
        };

        match resolve_clips_by_boundary(&content_to_process, &clips) {
            Ok(items) if !items.is_empty() => {
                resolved_clips = items;
                break;
            }
            Ok(_) => {
                last_error = Some(AppError::invalid_params("clips_build produced no clips"));
            }
            Err(err) => {
                last_error = Some(err);
            }
        }
    }

    if resolved_clips.is_empty() {
        return Err(last_error
            .unwrap_or_else(|| AppError::invalid_params("split_clips boundary matching failed")));
    }

    let _ = task
        .report_progress(75, Some("progress.stage.clipsBuildPersist"))
        .await?;

    sqlx::query("DELETE FROM novel_promotion_clips WHERE episodeId = ?")
        .bind(&episode.id)
        .execute(mysql)
        .await?;

    let mut created_count = 0usize;
    for clip in &resolved_clips {
        sqlx::query(
            "INSERT INTO novel_promotion_clips (id, episodeId, startText, endText, summary, location, characters, content, createdAt, updatedAt) VALUES (UUID(), ?, ?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
        )
        .bind(&episode.id)
        .bind(Some(clip.start_text.clone()))
        .bind(Some(clip.end_text.clone()))
        .bind(clip.summary.clone())
        .bind(clip.location.clone())
        .bind(clip.characters_json.clone())
        .bind(clip.content.clone())
        .execute(mysql)
        .await?;

        created_count += 1;
    }

    let _ = task
        .report_progress(96, Some("progress.stage.clipsBuildDone"))
        .await?;

    Ok(json!({
        "episodeId": episode.id,
        "count": created_count,
        "model": analysis_model,
    }))
}
