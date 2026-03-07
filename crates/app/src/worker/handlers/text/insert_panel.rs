use serde_json::{Value, json};
use sqlx::FromRow;
use waoowaoo_core::errors::AppError;

use crate::{runtime, task_context::TaskContext};

use super::shared;

#[derive(Debug, FromRow)]
struct PanelRow {
    id: String,
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
    #[sqlx(rename = "srtSegment")]
    srt_segment: Option<String>,
}

#[derive(Debug, FromRow)]
struct PanelIndexRow {
    id: String,
    #[sqlx(rename = "panelIndex")]
    panel_index: i32,
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
    let insert_after_panel_id = shared::read_string(payload, "insertAfterPanelId")
        .ok_or_else(|| AppError::invalid_params("insertAfterPanelId is required"))?;
    let user_input = shared::read_string(payload, "userInput")
        .or_else(|| shared::read_string(payload, "prompt"))
        .ok_or_else(|| AppError::invalid_params("userInput is required"))?;
    let analysis_model = shared::resolve_analysis_model(task, payload).await?;

    let prev_panel = sqlx::query_as::<_, PanelRow>(
        "SELECT id, panelIndex, shotType, cameraMove, description, videoPrompt, location, characters, srtSegment FROM novel_promotion_panels WHERE id = ? AND storyboardId = ? LIMIT 1",
    )
    .bind(&insert_after_panel_id)
    .bind(&storyboard_id)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("insert_after panel not found"))?;

    let next_panel = sqlx::query_as::<_, PanelRow>(
        "SELECT id, panelIndex, shotType, cameraMove, description, videoPrompt, location, characters, srtSegment FROM novel_promotion_panels WHERE storyboardId = ? AND panelIndex = ? LIMIT 1",
    )
    .bind(&storyboard_id)
    .bind(prev_panel.panel_index + 1)
    .fetch_optional(mysql)
    .await?;

    let prompt = format!(
        "Insert one storyboard panel between two neighboring panels.\nUser request: {}\n\nPrevious panel:\n{}\n\nNext panel:\n{}\n\nReturn JSON object only with fields: shot_type, camera_move, description, video_prompt, location, characters, source_text, duration.",
        user_input,
        serde_json::to_string_pretty(&json!({
            "shot_type": prev_panel.shot_type,
            "camera_move": prev_panel.camera_move,
            "description": prev_panel.description,
            "video_prompt": prev_panel.video_prompt,
            "location": prev_panel.location,
            "characters": prev_panel.characters,
            "source_text": prev_panel.srt_segment,
        }))
        .map_err(|err| AppError::internal(format!("failed to encode previous panel: {err}")))?,
        serde_json::to_string_pretty(&next_panel.as_ref().map(|panel| json!({
            "shot_type": panel.shot_type,
            "camera_move": panel.camera_move,
            "description": panel.description,
            "video_prompt": panel.video_prompt,
            "location": panel.location,
            "characters": panel.characters,
            "source_text": panel.srt_segment,
        })))
        .map_err(|err| AppError::internal(format!("failed to encode next panel: {err}")))?,
    );

    let _ = task
        .report_progress(40, Some("insert_panel_generate_text"))
        .await?;

    let response = shared::chat(task, &analysis_model, &prompt).await?;
    let generated = shared::parse_json_object_response(&response)?;

    let _ = task
        .report_progress(80, Some("insert_panel_persist"))
        .await?;

    let new_panel_id = uuid::Uuid::new_v4().to_string();
    let mut tx = mysql.begin().await?;

    let affected = sqlx::query_as::<_, PanelIndexRow>(
        "SELECT id, panelIndex FROM novel_promotion_panels WHERE storyboardId = ? AND panelIndex > ? ORDER BY panelIndex DESC",
    )
    .bind(&storyboard_id)
    .bind(prev_panel.panel_index)
    .fetch_all(&mut *tx)
    .await?;
    for panel in &affected {
        sqlx::query(
            "UPDATE novel_promotion_panels SET panelIndex = ?, updatedAt = NOW(3) WHERE id = ?",
        )
        .bind(panel.panel_index + 1)
        .bind(&panel.id)
        .execute(&mut *tx)
        .await?;
    }

    let shot_type = generated
        .get("shot_type")
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .or(prev_panel.shot_type.clone());
    let camera_move = generated
        .get("camera_move")
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .or(prev_panel.camera_move.clone());
    let description = generated
        .get("description")
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .or_else(|| Some(user_input.clone()));
    let video_prompt = generated
        .get("video_prompt")
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .or_else(|| description.clone());
    let location = generated
        .get("location")
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .or(prev_panel.location.clone());
    let characters = generated
        .get("characters")
        .map(|item| item.to_string())
        .or(prev_panel.characters.clone());
    let source_text = generated
        .get("source_text")
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .or(prev_panel.srt_segment.clone());
    let duration = generated.get("duration").and_then(Value::as_f64);

    sqlx::query(
        "INSERT INTO novel_promotion_panels (id, storyboardId, panelIndex, panelNumber, shotType, cameraMove, description, videoPrompt, location, characters, srtSegment, duration, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
    )
    .bind(&new_panel_id)
    .bind(&storyboard_id)
    .bind(prev_panel.panel_index + 1)
    .bind(prev_panel.panel_index + 2)
    .bind(shot_type)
    .bind(camera_move)
    .bind(description)
    .bind(video_prompt)
    .bind(location)
    .bind(characters)
    .bind(source_text)
    .bind(duration)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE novel_promotion_storyboards SET panelCount = panelCount + 1, updatedAt = NOW(3) WHERE id = ?",
    )
    .bind(&storyboard_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(json!({
        "storyboardId": storyboard_id,
        "panelId": new_panel_id,
        "panelIndex": prev_panel.panel_index + 1,
        "insertAfterPanelId": prev_panel.id,
        "model": analysis_model,
    }))
}
