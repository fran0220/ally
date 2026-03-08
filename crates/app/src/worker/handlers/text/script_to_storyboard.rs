use serde_json::{Value, json};
use sqlx::FromRow;
use waoowaoo_core::errors::AppError;

use crate::{consumer::WorkerTask, runtime, task_context::TaskContext};

use super::{screenplay_convert, shared, voice_analyze};

const MAX_STORYBOARD_ATTEMPTS: usize = 2;

#[derive(Debug, FromRow)]
struct EpisodeRow {
    id: String,
    #[sqlx(rename = "novelPromotionProjectId")]
    novel_promotion_project_id: String,
}

#[derive(Debug, FromRow)]
struct ClipRow {
    id: String,
    content: String,
    screenplay: Option<String>,
}

fn with_episode_payload(task: &WorkerTask) -> WorkerTask {
    let mut next = task.clone();
    if shared::read_string(&next.payload, "episodeId").is_some() {
        return next;
    }
    let Some(episode_id) = task.episode_id.clone() else {
        return next;
    };

    if let Some(object) = next.payload.as_object_mut() {
        object.insert("episodeId".to_string(), Value::String(episode_id));
    } else {
        next.payload = json!({ "episodeId": episode_id });
    }
    next
}

fn extract_panels(response: &str) -> Result<Vec<Value>, AppError> {
    if let Ok(object) = shared::parse_json_object_response(response) {
        if let Some(items) = object.get("finalPanels").and_then(Value::as_array) {
            return Ok(items.clone());
        }
        if let Some(items) = object.get("panels").and_then(Value::as_array) {
            return Ok(items.clone());
        }
    }
    shared::parse_json_array_response(response)
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    shared::ensure_novel_project(task).await?;
    let mysql = runtime::mysql()?;
    let payload = &task.payload;
    let locale = shared::resolve_prompt_locale(payload);

    let task_with_episode = task.with_task(with_episode_payload(task));
    let episode_id = shared::read_episode_id(&task_with_episode)
        .ok_or_else(|| AppError::invalid_params("episodeId is required"))?;
    let novel_project = shared::get_novel_project(task).await?;
    let analysis_model = shared::resolve_analysis_model(task, payload).await?;
    let run_id = shared::create_stream_run_id(task, "script_to_storyboard");

    let _ = task
        .report_progress(10, Some("progress.stage.scriptToStoryboardPrepare"))
        .await?;

    let episode = sqlx::query_as::<_, EpisodeRow>(
        "SELECT id, novelPromotionProjectId FROM novel_promotion_episodes WHERE id = ? LIMIT 1",
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

    let _ = task
        .report_progress(35, Some("progress.stage.scriptToStoryboardScreenplay"))
        .await?;
    let screenplay_result = screenplay_convert::handle(&task_with_episode).await?;

    let clips = sqlx::query_as::<_, ClipRow>(
        "SELECT id, content, screenplay FROM novel_promotion_clips WHERE episodeId = ? ORDER BY createdAt ASC",
    )
    .bind(&episode.id)
    .fetch_all(mysql)
    .await?;
    if clips.is_empty() {
        return Err(AppError::invalid_params(
            "script_to_storyboard requires clips",
        ));
    }

    let _ = task
        .report_progress(45, Some("progress.stage.scriptToStoryboardGenerating"))
        .await?;

    sqlx::query("DELETE FROM novel_promotion_storyboards WHERE episodeId = ?")
        .bind(&episode.id)
        .execute(mysql)
        .await?;

    let mut storyboard_count = 0usize;
    let mut panel_count = 0usize;
    let total_clips = i32::try_from(clips.len())
        .map_err(|err| AppError::internal(format!("clips count overflow: {err}")))?;
    for (clip_index, clip) in clips.iter().enumerate() {
        let clip_index_i32 = i32::try_from(clip_index)
            .map_err(|err| AppError::internal(format!("clip index overflow: {err}")))?;
        let loop_progress = 45 + ((clip_index_i32 * 40) / total_clips.max(1));
        let _ = task
            .report_progress(
                loop_progress,
                Some("progress.stage.scriptToStoryboardGenerating"),
            )
            .await?;

        let clip_source = clip
            .screenplay
            .as_deref()
            .map(str::trim)
            .map(|item| item.to_string())
            .filter(|item| !item.is_empty())
            .unwrap_or_else(|| clip.content.clone());

        let prompt = format!(
            "Generate storyboard panels for this clip.\nClip source:\n{}\n\nReturn JSON array only. Each panel item should include: panel_number, shot_type, camera_move, description, video_prompt, location, characters, source_text, duration.",
            clip_source,
        );
        let mut panels = Vec::new();
        let mut step_error: Option<AppError> = None;
        for attempt in 1..=MAX_STORYBOARD_ATTEMPTS {
            let step_title = format!(
                "{} {}/{}",
                shared::l(locale, "分镜生成", "Storyboard Generation"),
                clip_index + 1,
                clips.len()
            );
            let meta = shared::LlmStepMeta {
                run_id: Some(run_id.clone()),
                stream_run_id: Some(run_id.clone()),
                step_id: Some(if attempt == 1 {
                    format!("storyboard_clip_{}", clip.id)
                } else {
                    format!("storyboard_clip_{}_retry_{}", clip.id, attempt)
                }),
                step_title: Some(step_title),
                step_attempt: Some(attempt as i32),
                step_index: Some((clip_index + 1) as i32),
                step_total: Some(clips.len() as i32),
            };

            let response = match shared::chat_with_step(task, &analysis_model, &prompt, &meta).await
            {
                Ok(value) => value,
                Err(err) => {
                    step_error = Some(err);
                    continue;
                }
            };
            match extract_panels(&response) {
                Ok(items) if !items.is_empty() => {
                    panels = items;
                    break;
                }
                Ok(_) => {
                    step_error = Some(AppError::invalid_params(format!(
                        "script_to_storyboard returned no panels for clip {}",
                        clip.id
                    )));
                }
                Err(err) => {
                    step_error = Some(err);
                }
            }
        }
        if panels.is_empty() {
            return Err(step_error.unwrap_or_else(|| {
                AppError::invalid_params(format!(
                    "script_to_storyboard returned no panels for clip {}",
                    clip.id
                ))
            }));
        }

        let storyboard_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO novel_promotion_storyboards (id, episodeId, clipId, panelCount, createdAt, updatedAt) VALUES (?, ?, ?, ?, NOW(3), NOW(3))",
        )
        .bind(&storyboard_id)
        .bind(&episode.id)
        .bind(&clip.id)
        .bind(i32::try_from(panels.len()).map_err(|err| {
            AppError::internal(format!("panel count overflow: {err}"))
        })?)
        .execute(mysql)
        .await?;
        storyboard_count += 1;

        for (index, panel) in panels.iter().enumerate() {
            let panel_number = panel
                .get("panel_number")
                .and_then(Value::as_i64)
                .and_then(|value| i32::try_from(value).ok())
                .unwrap_or((index + 1) as i32);
            let shot_type = panel
                .get("shot_type")
                .and_then(Value::as_str)
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty());
            let camera_move = panel
                .get("camera_move")
                .and_then(Value::as_str)
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty());
            let description = panel
                .get("description")
                .and_then(Value::as_str)
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty());
            let video_prompt = panel
                .get("video_prompt")
                .and_then(Value::as_str)
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .or_else(|| description.clone());
            let location = panel
                .get("location")
                .and_then(Value::as_str)
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty());
            let characters = panel.get("characters").map(|item| item.to_string());
            let source_text = panel
                .get("source_text")
                .and_then(Value::as_str)
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty());
            let duration = panel.get("duration").and_then(Value::as_f64);

            sqlx::query(
                "INSERT INTO novel_promotion_panels (id, storyboardId, panelIndex, panelNumber, shotType, cameraMove, description, videoPrompt, location, characters, srtSegment, duration, createdAt, updatedAt) VALUES (UUID(), ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
            )
            .bind(&storyboard_id)
            .bind(index as i32)
            .bind(panel_number)
            .bind(shot_type)
            .bind(camera_move)
            .bind(description)
            .bind(video_prompt)
            .bind(location)
            .bind(characters)
            .bind(source_text)
            .bind(duration)
            .execute(mysql)
            .await?;
            panel_count += 1;
        }
    }

    let _ = task
        .report_progress(92, Some("progress.stage.scriptToStoryboardVoice"))
        .await?;
    let voice_result = voice_analyze::handle(&task_with_episode).await?;

    let _ = task
        .report_progress(96, Some("progress.stage.scriptToStoryboardDone"))
        .await?;

    Ok(json!({
        "success": true,
        "screenplay": screenplay_result,
        "voice": voice_result,
        "storyboardCount": storyboard_count,
        "panelCount": panel_count,
        "model": analysis_model,
    }))
}
