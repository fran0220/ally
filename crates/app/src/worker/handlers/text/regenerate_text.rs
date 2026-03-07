use serde_json::{Value, json};
use sqlx::FromRow;
use waoowaoo_core::errors::AppError;

use crate::{runtime, task_context::TaskContext};

use super::shared;

#[derive(Debug, FromRow)]
struct StoryboardContextRow {
    id: String,
    #[sqlx(rename = "clipId")]
    clip_id: String,
    #[sqlx(rename = "clipContent")]
    clip_content: Option<String>,
    #[sqlx(rename = "clipScreenplay")]
    clip_screenplay: Option<String>,
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

    let storyboard_id = shared::read_string(payload, "storyboardId")
        .or_else(|| {
            if !task.target_id.trim().is_empty() {
                Some(task.target_id.clone())
            } else {
                None
            }
        })
        .ok_or_else(|| AppError::invalid_params("storyboardId is required"))?;
    let analysis_model = shared::resolve_analysis_model(task, payload).await?;

    let _ = task
        .report_progress(20, Some("regenerate_storyboard_prepare"))
        .await?;

    let context = sqlx::query_as::<_, StoryboardContextRow>(
        "SELECT sb.id, sb.clipId, clip.content AS clipContent, clip.screenplay AS clipScreenplay FROM novel_promotion_storyboards sb INNER JOIN novel_promotion_clips clip ON clip.id = sb.clipId WHERE sb.id = ? LIMIT 1",
    )
    .bind(&storyboard_id)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("storyboard not found"))?;

    let source_text = context
        .clip_screenplay
        .as_deref()
        .map(str::trim)
        .map(|item| item.to_string())
        .filter(|item| !item.is_empty())
        .or_else(|| {
            context
                .clip_content
                .as_deref()
                .map(str::trim)
                .map(|item| item.to_string())
                .filter(|item| !item.is_empty())
        })
        .ok_or_else(|| AppError::invalid_params("clip content is empty"))?;

    let prompt = format!(
        "Generate storyboard panel text breakdown for the following clip source.\nSource:\n{}\n\nReturn JSON array only. Each panel item should include: panel_number, shot_type, camera_move, description, video_prompt, location, characters, source_text, duration.",
        source_text,
    );
    let response = shared::chat(task, &analysis_model, &prompt).await?;
    let panels = extract_panels(&response)?;
    if panels.is_empty() {
        return Err(AppError::invalid_params(
            "regenerate_storyboard_text returned empty panels",
        ));
    }

    let _ = task
        .report_progress(85, Some("regenerate_storyboard_persist"))
        .await?;

    sqlx::query("DELETE FROM novel_promotion_panels WHERE storyboardId = ?")
        .bind(&context.id)
        .execute(mysql)
        .await?;

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
        let scene_type = panel
            .get("scene_type")
            .and_then(Value::as_str)
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty());

        sqlx::query(
            "INSERT INTO novel_promotion_panels (id, storyboardId, panelIndex, panelNumber, shotType, cameraMove, description, videoPrompt, location, characters, srtSegment, duration, sceneType, createdAt, updatedAt) VALUES (UUID(), ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
        )
        .bind(&context.id)
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
        .bind(scene_type)
        .execute(mysql)
        .await?;
    }

    sqlx::query(
        "UPDATE novel_promotion_storyboards SET panelCount = ?, updatedAt = NOW(3) WHERE id = ?",
    )
    .bind(
        i32::try_from(panels.len())
            .map_err(|err| AppError::internal(format!("panel count overflow: {err}")))?,
    )
    .bind(&context.id)
    .execute(mysql)
    .await?;

    Ok(json!({
        "storyboardId": context.id,
        "clipId": context.clip_id,
        "panelCount": panels.len(),
        "model": analysis_model,
    }))
}
