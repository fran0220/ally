use serde_json::{Value, json};
use sqlx::FromRow;
use waoowaoo_core::api_config::UnifiedModelType;
use waoowaoo_core::capabilities::resolve_builtin_capabilities_by_model_key;
use waoowaoo_core::errors::AppError;
use waoowaoo_core::generators::{self, VideoGenerateOptions};
use waoowaoo_core::media;

use crate::{handlers::image::shared, runtime, task_context::TaskContext};

#[derive(Debug, FromRow)]
struct PanelVideoRow {
    id: String,
    description: Option<String>,
    #[sqlx(rename = "videoPrompt")]
    video_prompt: Option<String>,
    #[sqlx(rename = "firstLastFramePrompt")]
    first_last_frame_prompt: Option<String>,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
}

fn read_i32(value: &Value) -> Option<i32> {
    value
        .as_i64()
        .or_else(|| {
            value
                .as_str()
                .and_then(|raw| raw.trim().parse::<i64>().ok())
        })
        .and_then(|parsed| i32::try_from(parsed).ok())
}

#[derive(Debug, FromRow)]
struct PanelProjectContextRow {
    #[sqlx(rename = "projectId")]
    project_id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
}

#[derive(Debug, FromRow)]
struct PanelImageRow {
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
}

fn read_u32(payload: &Value, key: &str) -> Option<u32> {
    payload.get(key).and_then(|value| {
        value
            .as_u64()
            .or_else(|| {
                value
                    .as_str()
                    .and_then(|raw| raw.trim().parse::<u64>().ok())
            })
            .and_then(|parsed| u32::try_from(parsed).ok())
    })
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    let mysql = runtime::mysql()?;
    let payload = &task.payload;

    let panel = if task.target_type == "NovelPromotionPanel" && !task.target_id.trim().is_empty() {
        sqlx::query_as::<_, PanelVideoRow>(
            "SELECT id, description, videoPrompt, firstLastFramePrompt, imageUrl FROM novel_promotion_panels WHERE id = ? LIMIT 1",
        )
        .bind(&task.target_id)
        .fetch_optional(mysql)
        .await?
        .ok_or_else(|| AppError::not_found("panel not found"))?
    } else if let Some(panel_id) =
        shared::read_string(payload, "panelId").or_else(|| shared::read_string(payload, "targetId"))
    {
        sqlx::query_as::<_, PanelVideoRow>(
            "SELECT id, description, videoPrompt, firstLastFramePrompt, imageUrl FROM novel_promotion_panels WHERE id = ? LIMIT 1",
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

        sqlx::query_as::<_, PanelVideoRow>(
            "SELECT id, description, videoPrompt, firstLastFramePrompt, imageUrl FROM novel_promotion_panels WHERE storyboardId = ? AND panelIndex = ? LIMIT 1",
        )
        .bind(storyboard_id)
        .bind(panel_index)
        .fetch_optional(mysql)
        .await?
        .ok_or_else(|| AppError::not_found("panel not found"))?
    };

    let context = sqlx::query_as::<_, PanelProjectContextRow>(
        "SELECT np.projectId, p.userId FROM novel_promotion_panels panel INNER JOIN novel_promotion_storyboards sb ON sb.id = panel.storyboardId INNER JOIN novel_promotion_clips c ON c.id = sb.clipId INNER JOIN novel_promotion_episodes ep ON ep.id = c.episodeId INNER JOIN novel_promotion_projects np ON np.id = ep.novelPromotionProjectId INNER JOIN projects p ON p.id = np.projectId WHERE panel.id = ? LIMIT 1",
    )
    .bind(&panel.id)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("panel project context not found"))?;

    let project_models = shared::get_project_models(&context.project_id, &context.user_id).await?;

    let first_last_frame = payload.get("firstLastFrame").and_then(Value::as_object);
    let mut video_model = shared::read_string(payload, "videoModel").ok_or_else(|| {
        AppError::invalid_params("VIDEO_MODEL_REQUIRED: payload.videoModel is required")
    })?;
    if let Some(object) = first_last_frame
        && let Some(fl_model) = object
            .get("flModel")
            .and_then(Value::as_str)
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
    {
        video_model = fl_model;
    }

    if first_last_frame.is_some() {
        let capabilities =
            resolve_builtin_capabilities_by_model_key(UnifiedModelType::Video, &video_model)?;
        let supported = capabilities
            .and_then(|item| item.video)
            .and_then(|item| item.firstlastframe)
            .unwrap_or(false);
        if !supported {
            return Err(AppError::invalid_params(format!(
                "VIDEO_FIRSTLASTFRAME_MODEL_UNSUPPORTED: {video_model}"
            )));
        }
    }

    let _ = task
        .report_progress(10, Some("generate_panel_video"))
        .await?;

    let source_image = shared::to_fetchable_url(panel.image_url.as_deref())
        .ok_or_else(|| AppError::invalid_params("panel imageUrl is missing"))?;

    let prompt = first_last_frame
        .and_then(|item| item.get("customPrompt"))
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .or_else(|| shared::read_string(payload, "customPrompt"))
        .or_else(|| {
            panel
                .first_last_frame_prompt
                .as_deref()
                .map(str::trim)
                .map(|item| item.to_string())
                .filter(|item| !item.is_empty())
        })
        .or_else(|| {
            panel
                .video_prompt
                .as_deref()
                .map(str::trim)
                .map(|item| item.to_string())
                .filter(|item| !item.is_empty())
        })
        .or_else(|| {
            panel
                .description
                .as_deref()
                .map(str::trim)
                .map(|item| item.to_string())
                .filter(|item| !item.is_empty())
        })
        .ok_or_else(|| AppError::invalid_params("panel video prompt is missing"))?;

    let generation_options = payload.get("generationOptions");
    let duration = generation_options
        .and_then(|item| read_u32(item, "duration"))
        .or_else(|| read_u32(payload, "duration"));
    let resolution = generation_options
        .and_then(|item| item.get("resolution"))
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty());
    let aspect_ratio = generation_options
        .and_then(|item| item.get("aspectRatio"))
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .or_else(|| Some(project_models.video_ratio.clone()));
    let generate_audio = generation_options
        .and_then(|item| item.get("generateAudio"))
        .and_then(Value::as_bool);

    let mut last_frame_image_source: Option<String> = None;
    if let Some(object) = first_last_frame {
        let storyboard_id = object
            .get("lastFrameStoryboardId")
            .and_then(Value::as_str)
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty());
        let panel_index = object.get("lastFramePanelIndex").and_then(read_i32);

        if let (Some(last_storyboard_id), Some(last_panel_index)) = (storyboard_id, panel_index) {
            let last_panel = sqlx::query_as::<_, PanelImageRow>(
                "SELECT imageUrl FROM novel_promotion_panels WHERE storyboardId = ? AND panelIndex = ? LIMIT 1",
            )
            .bind(last_storyboard_id)
            .bind(last_panel_index)
            .fetch_optional(mysql)
            .await?;

            last_frame_image_source = last_panel
                .and_then(|item| item.image_url)
                .and_then(|item| shared::to_fetchable_url(Some(&item)));
        }
    }

    let generation_mode = if first_last_frame.is_some() {
        "firstlastframe"
    } else {
        "normal"
    };

    let video_source = generators::generate_video(
        mysql,
        &video_model,
        &source_image,
        VideoGenerateOptions {
            prompt: Some(prompt),
            duration,
            resolution,
            aspect_ratio,
            generation_mode: Some(generation_mode.to_string()),
            generate_audio,
            last_frame_image_source,
        },
    )
    .await?;

    let storage_key =
        media::upload_source_to_storage(&video_source, "panel-video", &panel.id).await?;

    sqlx::query(
        "UPDATE novel_promotion_panels SET videoUrl = ?, videoGenerationMode = ?, updatedAt = NOW(3) WHERE id = ?",
    )
    .bind(&storage_key)
    .bind(generation_mode)
    .bind(&panel.id)
    .execute(mysql)
    .await?;

    Ok(json!({
        "panelId": panel.id,
        "videoUrl": storage_key,
    }))
}
