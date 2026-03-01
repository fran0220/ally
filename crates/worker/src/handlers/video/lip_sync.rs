use serde_json::{Value, json};
use sqlx::FromRow;
use waoowaoo_core::errors::AppError;
use waoowaoo_core::generators;
use waoowaoo_core::media;

use crate::{handlers::image::shared, runtime, task_context::TaskContext};

#[derive(Debug, FromRow)]
struct PanelLipSyncRow {
    id: String,
    #[sqlx(rename = "videoUrl")]
    video_url: Option<String>,
}

#[derive(Debug, FromRow)]
struct VoiceLineRow {
    id: String,
    #[sqlx(rename = "audioUrl")]
    audio_url: Option<String>,
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    let mysql = runtime::mysql()?;
    let payload = &task.payload;

    let panel = if task.target_type == "NovelPromotionPanel" && !task.target_id.trim().is_empty() {
        sqlx::query_as::<_, PanelLipSyncRow>(
            "SELECT id, videoUrl FROM novel_promotion_panels WHERE id = ? LIMIT 1",
        )
        .bind(&task.target_id)
        .fetch_optional(mysql)
        .await?
        .ok_or_else(|| AppError::not_found("panel not found"))?
    } else if let Some(panel_id) =
        shared::read_string(payload, "panelId").or_else(|| shared::read_string(payload, "targetId"))
    {
        sqlx::query_as::<_, PanelLipSyncRow>(
            "SELECT id, videoUrl FROM novel_promotion_panels WHERE id = ? LIMIT 1",
        )
        .bind(panel_id)
        .fetch_optional(mysql)
        .await?
        .ok_or_else(|| AppError::not_found("panel not found"))?
    } else {
        let storyboard_id = shared::read_string(payload, "storyboardId")
            .ok_or_else(|| AppError::invalid_params("storyboardId is required"))?;
        let panel_index = shared::read_i32(payload, "panelIndex")
            .ok_or_else(|| AppError::invalid_params("panelIndex is required"))?;

        sqlx::query_as::<_, PanelLipSyncRow>(
            "SELECT id, videoUrl FROM novel_promotion_panels WHERE storyboardId = ? AND panelIndex = ? LIMIT 1",
        )
        .bind(storyboard_id)
        .bind(panel_index)
        .fetch_optional(mysql)
        .await?
        .ok_or_else(|| AppError::not_found("panel not found"))?
    };

    let video_source = shared::to_fetchable_url(panel.video_url.as_deref())
        .ok_or_else(|| AppError::invalid_params("panel base videoUrl is missing"))?;

    let voice_line_id = shared::read_string(payload, "voiceLineId")
        .ok_or_else(|| AppError::invalid_params("voiceLineId is required"))?;
    let voice_line = sqlx::query_as::<_, VoiceLineRow>(
        "SELECT id, audioUrl FROM novel_promotion_voice_lines WHERE id = ? LIMIT 1",
    )
    .bind(&voice_line_id)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("voice line not found"))?;
    let audio_source = shared::to_fetchable_url(voice_line.audio_url.as_deref())
        .ok_or_else(|| AppError::invalid_params("voice line audioUrl is missing"))?;

    let lip_sync_model = shared::read_string(payload, "lipSyncModel")
        .ok_or_else(|| AppError::invalid_params("lipSyncModel is required"))?;

    let _ = task.report_progress(25, Some("submit_lip_sync")).await?;

    let lip_sync_source =
        generators::generate_lip_sync(mysql, &lip_sync_model, &video_source, &audio_source).await?;

    let _ = task.report_progress(93, Some("persist_lip_sync")).await?;

    let storage_key =
        media::upload_source_to_storage(&lip_sync_source, "lip-sync", &panel.id).await?;

    sqlx::query(
        "UPDATE novel_promotion_panels SET lipSyncVideoUrl = ?, lipSyncTaskId = NULL, updatedAt = NOW(3) WHERE id = ?",
    )
    .bind(&storage_key)
    .bind(&panel.id)
    .execute(mysql)
    .await?;

    Ok(json!({
        "panelId": panel.id,
        "voiceLineId": voice_line.id,
        "lipSyncVideoUrl": storage_key,
    }))
}
