use serde_json::{Value, json};
use sqlx::FromRow;
use waoowaoo_core::{errors::AppError, generators::ImageGenerateOptions};

use crate::{runtime, task_context::TaskContext};

use super::shared;

#[derive(Debug, FromRow)]
struct PanelVariantRow {
    id: String,
    #[sqlx(rename = "storyboardId")]
    storyboard_id: String,
    description: Option<String>,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
    #[sqlx(rename = "projectId")]
    project_id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
}

fn read_variant_prompt(payload: &Value, source_description: Option<&str>) -> String {
    let variant = payload.get("variant").and_then(Value::as_object);

    variant
        .and_then(|item| item.get("description"))
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            variant
                .and_then(|item| item.get("video_prompt"))
                .and_then(Value::as_str)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .or_else(|| {
            source_description
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| "Generate a cinematic panel variant".to_string())
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    let payload = &task.payload;
    let mysql = runtime::mysql()?;
    let new_panel_id = shared::read_string(payload, "newPanelId")
        .or_else(|| {
            let value = task.target_id.trim();
            if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        })
        .ok_or_else(|| AppError::invalid_params("newPanelId is required"))?;
    let source_panel_id = shared::read_string(payload, "sourcePanelId")
        .ok_or_else(|| AppError::invalid_params("sourcePanelId is required"))?;

    let new_panel = sqlx::query_as::<_, PanelVariantRow>(
        "SELECT panel.id, panel.storyboardId, panel.description, panel.imageUrl, np.projectId, p.userId FROM novel_promotion_panels panel INNER JOIN novel_promotion_storyboards sb ON sb.id = panel.storyboardId INNER JOIN novel_promotion_clips c ON c.id = sb.clipId INNER JOIN novel_promotion_episodes ep ON ep.id = c.episodeId INNER JOIN novel_promotion_projects np ON np.id = ep.novelPromotionProjectId INNER JOIN projects p ON p.id = np.projectId WHERE panel.id = ? LIMIT 1",
    )
    .bind(&new_panel_id)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("new panel not found"))?;

    let source_panel = sqlx::query_as::<_, PanelVariantRow>(
        "SELECT panel.id, panel.storyboardId, panel.description, panel.imageUrl, np.projectId, p.userId FROM novel_promotion_panels panel INNER JOIN novel_promotion_storyboards sb ON sb.id = panel.storyboardId INNER JOIN novel_promotion_clips c ON c.id = sb.clipId INNER JOIN novel_promotion_episodes ep ON ep.id = c.episodeId INNER JOIN novel_promotion_projects np ON np.id = ep.novelPromotionProjectId INNER JOIN projects p ON p.id = np.projectId WHERE panel.id = ? LIMIT 1",
    )
    .bind(&source_panel_id)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("source panel not found"))?;

    let project_models =
        shared::get_project_models(&new_panel.project_id, &new_panel.user_id).await?;
    let storyboard_model = shared::read_string(payload, "imageModel")
        .or(project_models.storyboard_model.clone())
        .ok_or_else(|| AppError::invalid_params("storyboard image model is not configured"))?;

    let resolution = payload
        .get("generationOptions")
        .and_then(|item| item.get("resolution"))
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .or(project_models.image_resolution.clone());

    let mut references = Vec::new();
    if let Some(source_url) = shared::to_fetchable_url(source_panel.image_url.as_deref()) {
        references.push(source_url);
    }
    let references = shared::normalize_reference_urls(&references).await?;

    let mut prompt = read_variant_prompt(payload, source_panel.description.as_deref());
    let locale = shared::read_locale_tag(payload);
    let style_prompt =
        shared::resolve_art_style_prompt(project_models.art_style.as_deref(), locale)
            .unwrap_or_else(|| shared::default_image_style_prompt(locale).to_string());
    prompt.push_str(". Visual style: ");
    prompt.push_str(&style_prompt);

    let image_key = shared::generate_image_to_storage(
        &storyboard_model,
        &prompt,
        ImageGenerateOptions {
            reference_images: references,
            aspect_ratio: Some(project_models.video_ratio.clone()),
            resolution,
            output_format: Some("png".to_string()),
            quality: None,
        },
        "panel-variant",
        &new_panel.id,
    )
    .await?;

    sqlx::query("UPDATE novel_promotion_panels SET imageUrl = ?, updatedAt = NOW(3) WHERE id = ?")
        .bind(&image_key)
        .bind(&new_panel.id)
        .execute(mysql)
        .await?;

    Ok(json!({
        "panelId": new_panel.id,
        "storyboardId": new_panel.storyboard_id,
        "imageUrl": image_key,
    }))
}
