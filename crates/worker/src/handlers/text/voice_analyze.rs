use std::collections::HashMap;

use serde_json::{Value, json};
use sqlx::FromRow;
use waoowaoo_core::errors::AppError;
use waoowaoo_core::prompt_i18n::{PromptIds, PromptVariables};

use crate::{runtime, task_context::TaskContext};

use super::shared;

const MAX_VOICE_ANALYZE_ATTEMPTS: usize = 2;

#[derive(Debug, FromRow)]
struct EpisodeRow {
    id: String,
    #[sqlx(rename = "novelPromotionProjectId")]
    novel_promotion_project_id: String,
    #[sqlx(rename = "novelText")]
    novel_text: Option<String>,
}

#[derive(Debug, FromRow)]
struct StoryboardRow {
    id: String,
    #[sqlx(rename = "clipId")]
    clip_id: String,
}

#[derive(Debug, FromRow)]
struct PanelRow {
    id: String,
    #[sqlx(rename = "storyboardId")]
    storyboard_id: String,
    #[sqlx(rename = "panelIndex")]
    panel_index: i32,
    #[sqlx(rename = "shotType")]
    shot_type: Option<String>,
    #[sqlx(rename = "cameraMove")]
    camera_move: Option<String>,
    description: Option<String>,
    #[sqlx(rename = "videoPrompt")]
    video_prompt: Option<String>,
    location: Option<String>,
    characters: Option<String>,
}

#[derive(Debug, FromRow)]
struct CharacterIntroRow {
    name: String,
    introduction: Option<String>,
}

fn extract_voice_lines(payload: &Value) -> Vec<Value> {
    payload
        .get("voice_lines")
        .or_else(|| payload.get("voiceLines"))
        .and_then(Value::as_array)
        .cloned()
        .or_else(|| payload.as_array().cloned())
        .unwrap_or_default()
}

fn read_required_line_index(line: &Value, index: usize) -> Result<i32, AppError> {
    let line_index = line
        .get("lineIndex")
        .or_else(|| line.get("line_index"))
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .ok_or_else(|| {
            AppError::invalid_params(format!(
                "voice line {} is missing valid lineIndex",
                index + 1
            ))
        })?;
    if line_index <= 0 {
        return Err(AppError::invalid_params(format!(
            "voice line {} has invalid lineIndex",
            index + 1
        )));
    }
    Ok(line_index)
}

fn read_required_string(line: &Value, key: &str, index: usize) -> Result<String, AppError> {
    line.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| item.to_string())
        .ok_or_else(|| {
            AppError::invalid_params(format!("voice line {} is missing valid {key}", index + 1))
        })
}

fn read_required_emotion_strength(line: &Value, index: usize) -> Result<f64, AppError> {
    let value = line
        .get("emotionStrength")
        .or_else(|| line.get("emotion_strength"))
        .and_then(Value::as_f64)
        .ok_or_else(|| {
            AppError::invalid_params(format!(
                "voice line {} is missing valid emotionStrength",
                index + 1
            ))
        })?;
    Ok(value.clamp(0.1, 1.0))
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    shared::ensure_novel_project(task).await?;
    let mysql = runtime::mysql()?;
    let payload = &task.payload;

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
    let novel_text = episode
        .novel_text
        .as_deref()
        .map(str::trim)
        .map(|item| item.to_string())
        .filter(|item| !item.is_empty())
        .ok_or_else(|| AppError::invalid_params("No novel text to analyze"))?;

    let storyboards = sqlx::query_as::<_, StoryboardRow>(
        "SELECT id, clipId FROM novel_promotion_storyboards WHERE episodeId = ? ORDER BY createdAt ASC",
    )
    .bind(&episode.id)
    .fetch_all(mysql)
    .await?;

    let mut panel_id_by_storyboard_panel = HashMap::new();
    let mut storyboard_payload = Vec::with_capacity(storyboards.len());
    for storyboard in &storyboards {
        let panels = sqlx::query_as::<_, PanelRow>(
            "SELECT id, storyboardId, panelIndex, shotType, cameraMove, description, videoPrompt, location, characters FROM novel_promotion_panels WHERE storyboardId = ? ORDER BY panelIndex ASC",
        )
        .bind(&storyboard.id)
        .fetch_all(mysql)
        .await?;

        for panel in &panels {
            panel_id_by_storyboard_panel.insert(
                format!("{}:{}", storyboard.id, panel.panel_index),
                panel.id.clone(),
            );
        }

        storyboard_payload.push(json!({
            "id": storyboard.id,
            "clipId": storyboard.clip_id,
            "panels": panels
                .into_iter()
                .map(|panel| {
                    json!({
                        "id": panel.id,
                        "storyboardId": panel.storyboard_id,
                        "panelIndex": panel.panel_index,
                        "shotType": panel.shot_type,
                        "cameraMove": panel.camera_move,
                        "description": panel.description,
                        "videoPrompt": panel.video_prompt,
                        "location": panel.location,
                        "characters": panel.characters,
                    })
                })
                .collect::<Vec<_>>(),
        }));
    }
    if panel_id_by_storyboard_panel.is_empty() {
        return Err(AppError::invalid_params(
            "No storyboard panels found for voice matching",
        ));
    }

    let character_rows = sqlx::query_as::<_, CharacterIntroRow>(
        "SELECT name, introduction FROM novel_promotion_characters WHERE novelPromotionProjectId = ? ORDER BY createdAt ASC",
    )
    .bind(&novel_project.id)
    .fetch_all(mysql)
    .await?;
    let characters_lib_name = character_rows
        .iter()
        .map(|item| item.name.as_str())
        .collect::<Vec<_>>()
        .join("、");
    let characters_introduction = shared::build_characters_introduction(
        &character_rows
            .iter()
            .map(|item| (item.name.clone(), item.introduction.clone()))
            .collect::<Vec<_>>(),
    );

    let mut prompt_variables = PromptVariables::new();
    prompt_variables.insert("input".to_string(), novel_text);
    prompt_variables.insert(
        "characters_lib_name".to_string(),
        if characters_lib_name.is_empty() {
            "无".to_string()
        } else {
            characters_lib_name
        },
    );
    prompt_variables.insert(
        "characters_introduction".to_string(),
        characters_introduction,
    );
    prompt_variables.insert(
        "storyboard_json".to_string(),
        serde_json::to_string_pretty(&storyboard_payload).map_err(|err| {
            AppError::internal(format!("failed to encode storyboard json: {err}"))
        })?,
    );
    let prompt =
        shared::render_prompt_template(payload, PromptIds::NP_VOICE_ANALYSIS, &prompt_variables)?;

    let _ = task
        .report_progress(20, Some("voice_analyze_prepare"))
        .await?;

    let run_id = shared::create_stream_run_id(task, "voice_analyze");
    let mut voice_lines = Vec::new();
    let mut last_error: Option<AppError> = None;
    for attempt in 1..=MAX_VOICE_ANALYZE_ATTEMPTS {
        let step_meta = shared::LlmStepMeta {
            run_id: Some(run_id.clone()),
            stream_run_id: Some(run_id.clone()),
            step_id: Some(if attempt == 1 {
                "voice_analyze".to_string()
            } else {
                format!("voice_analyze_retry_{attempt}")
            }),
            step_title: Some("台词分析".to_string()),
            step_attempt: Some(attempt as i32),
            step_index: Some(1),
            step_total: Some(1),
        };

        let response =
            match shared::chat_with_step(task, &analysis_model, &prompt, &step_meta).await {
                Ok(value) => value,
                Err(err) => {
                    last_error = Some(err);
                    continue;
                }
            };
        let parsed = if let Ok(object) = shared::parse_json_object_response(&response) {
            object
        } else {
            Value::Array(shared::parse_json_array_response(&response)?)
        };
        let candidate_lines = extract_voice_lines(&parsed);
        if candidate_lines.is_empty() {
            last_error = Some(AppError::invalid_params(
                "voice_analyze returned empty lines",
            ));
            continue;
        }

        // Validate shape before mutating DB so malformed attempts can retry.
        let mut validation_failed = false;
        for (index, line) in candidate_lines.iter().enumerate() {
            if read_required_line_index(line, index).is_err()
                || read_required_string(line, "speaker", index).is_err()
                || read_required_string(line, "content", index).is_err()
                || read_required_emotion_strength(line, index).is_err()
            {
                validation_failed = true;
                break;
            }
        }
        if validation_failed {
            last_error = Some(AppError::invalid_params(
                "voice_analyze returned malformed voice lines",
            ));
            continue;
        }

        voice_lines = candidate_lines;
        break;
    }

    if voice_lines.is_empty() {
        return Err(last_error
            .unwrap_or_else(|| AppError::invalid_params("voice_analyze returned empty lines")));
    }

    let _ = task
        .report_progress(82, Some("voice_analyze_persist"))
        .await?;

    sqlx::query("DELETE FROM novel_promotion_voice_lines WHERE episodeId = ?")
        .bind(&episode.id)
        .execute(mysql)
        .await?;

    let mut speaker_stats: HashMap<String, i32> = HashMap::new();
    let mut matched_count = 0i32;

    for (index, line) in voice_lines.iter().enumerate() {
        let line_index = read_required_line_index(line, index)?;
        let speaker = read_required_string(line, "speaker", index)?;
        let content = read_required_string(line, "content", index)?;
        let emotion_strength = read_required_emotion_strength(line, index)?;

        let mut matched_panel_id: Option<String> = None;
        let mut matched_storyboard_id: Option<String> = None;
        let mut matched_panel_index: Option<i32> = None;
        if let Some(matched_panel) = line.get("matchedPanel") {
            if !matched_panel.is_null() {
                let matched_panel = matched_panel.as_object().ok_or_else(|| {
                    AppError::invalid_params(format!(
                        "voice line {} has invalid matchedPanel",
                        index + 1
                    ))
                })?;
                let storyboard_id = matched_panel
                    .get("storyboardId")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .ok_or_else(|| {
                        AppError::invalid_params(format!(
                            "voice line {} has invalid matchedPanel",
                            index + 1
                        ))
                    })?;
                let panel_index = matched_panel
                    .get("panelIndex")
                    .and_then(Value::as_i64)
                    .and_then(|value| i32::try_from(value).ok())
                    .filter(|value| *value >= 0)
                    .ok_or_else(|| {
                        AppError::invalid_params(format!(
                            "voice line {} has invalid matchedPanel",
                            index + 1
                        ))
                    })?;
                let key = format!("{}:{}", storyboard_id, panel_index);
                let panel_id =
                    panel_id_by_storyboard_panel
                        .get(&key)
                        .cloned()
                        .ok_or_else(|| {
                            AppError::invalid_params(format!(
                                "voice line {} references non-existent panel {}",
                                index + 1,
                                key
                            ))
                        })?;
                matched_panel_id = Some(panel_id);
                matched_storyboard_id = Some(storyboard_id.to_string());
                matched_panel_index = Some(panel_index);
                matched_count += 1;
            }
        }

        sqlx::query(
            "INSERT INTO novel_promotion_voice_lines (id, episodeId, lineIndex, speaker, content, emotionStrength, matchedPanelId, matchedStoryboardId, matchedPanelIndex, createdAt, updatedAt) VALUES (UUID(), ?, ?, ?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
        )
        .bind(&episode.id)
        .bind(line_index)
        .bind(&speaker)
        .bind(&content)
        .bind(emotion_strength)
        .bind(matched_panel_id)
        .bind(matched_storyboard_id)
        .bind(matched_panel_index)
        .execute(mysql)
        .await?;

        *speaker_stats.entry(speaker).or_insert(0) += 1;
    }

    let _ = task
        .report_progress(96, Some("voice_analyze_persist_done"))
        .await?;

    Ok(json!({
        "episodeId": episode.id,
        "count": voice_lines.len(),
        "matchedCount": matched_count,
        "speakerStats": speaker_stats,
        "model": analysis_model,
    }))
}
