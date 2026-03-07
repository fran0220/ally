use serde_json::{Value, json};
use sqlx::FromRow;
use waoowaoo_core::{errors::AppError, generators::ImageGenerateOptions};

use crate::{runtime, task_context::TaskContext};

use super::shared;

#[derive(Debug, FromRow)]
struct CharacterModifyRow {
    id: String,
    #[sqlx(rename = "imageUrls")]
    image_urls: Option<String>,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
    #[sqlx(rename = "selectedIndex")]
    selected_index: Option<i32>,
    #[sqlx(rename = "projectId")]
    project_id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
}

#[derive(Debug, FromRow)]
struct LocationModifyRow {
    id: String,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
    #[sqlx(rename = "projectId")]
    project_id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
}

#[derive(Debug, FromRow)]
struct PanelModifyRow {
    id: String,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
    #[sqlx(rename = "projectId")]
    project_id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
}

fn pick_character_source_key(row: &CharacterModifyRow, image_index: usize) -> Option<String> {
    let image_urls = shared::parse_image_urls(row.image_urls.as_deref());
    image_urls
        .get(image_index)
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .or_else(|| {
            image_urls
                .into_iter()
                .find(|value| !value.trim().is_empty())
                .or_else(|| row.image_url.clone())
        })
}

async fn handle_character_modify(task: &TaskContext) -> Result<Value, AppError> {
    let payload = &task.payload;
    let mysql = runtime::mysql()?;
    let appearance_id = shared::read_string(payload, "appearanceId")
        .or_else(|| shared::read_string(payload, "targetId"))
        .or_else(|| {
            let value = task.target_id.trim();
            if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        })
        .ok_or_else(|| AppError::invalid_params("appearanceId is required"))?;

    let row = sqlx::query_as::<_, CharacterModifyRow>(
        "SELECT ca.id, ca.imageUrls, ca.imageUrl, ca.selectedIndex, np.projectId, p.userId FROM character_appearances ca INNER JOIN novel_promotion_characters c ON c.id = ca.characterId INNER JOIN novel_promotion_projects np ON np.id = c.novelPromotionProjectId INNER JOIN projects p ON p.id = np.projectId WHERE ca.id = ? LIMIT 1",
    )
    .bind(&appearance_id)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("character appearance not found"))?;

    let project_models = shared::get_project_models(&row.project_id, &row.user_id).await?;
    let edit_model = shared::read_string(payload, "imageModel")
        .or(project_models.edit_model.clone())
        .ok_or_else(|| AppError::invalid_params("edit image model is not configured"))?;

    let image_index = shared::read_usize(payload, "imageIndex")
        .or_else(|| {
            row.selected_index
                .and_then(|value| (value >= 0).then_some(value as usize))
        })
        .unwrap_or(0);

    let source_key = pick_character_source_key(&row, image_index)
        .ok_or_else(|| AppError::invalid_params("character image source is missing"))?;

    let mut references = vec![
        shared::to_fetchable_url(Some(&source_key))
            .ok_or_else(|| AppError::invalid_params("invalid character image source"))?,
    ];
    references.extend(shared::collect_extra_image_urls(payload));
    let references = shared::normalize_reference_urls(&references).await?;

    let modify_prompt = shared::read_string(payload, "modifyPrompt")
        .ok_or_else(|| AppError::invalid_params("modifyPrompt is required"))?;
    let resolution = payload
        .get("generationOptions")
        .and_then(|item| item.get("resolution"))
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .or(project_models.image_resolution.clone());

    let image_key = shared::generate_image_to_storage(
        &edit_model,
        &format!("Modify character image: {modify_prompt}"),
        ImageGenerateOptions {
            reference_images: references,
            aspect_ratio: Some("3:2".to_string()),
            resolution,
            output_format: Some("png".to_string()),
            quality: None,
        },
        "character-modify",
        &format!("{}-{image_index}", row.id),
    )
    .await?;

    let mut image_urls = shared::parse_image_urls(row.image_urls.as_deref());
    if image_urls.len() <= image_index {
        image_urls.resize(image_index + 1, String::new());
    }
    image_urls[image_index] = image_key.clone();

    let should_update_main = if let Some(selected_index) = row.selected_index {
        selected_index >= 0 && selected_index as usize == image_index
    } else {
        image_index == 0 || image_urls.len() == 1
    };

    sqlx::query(
        "UPDATE character_appearances SET previousImageUrl = imageUrl, previousImageUrls = imageUrls, previousDescription = description, imageUrls = ?, imageUrl = ?, updatedAt = NOW(3) WHERE id = ?",
    )
    .bind(serde_json::to_string(&image_urls).map_err(|err| {
        AppError::internal(format!("failed to encode character imageUrls: {err}"))
    })?)
    .bind(if should_update_main {
        Some(image_key.clone())
    } else {
        row.image_url.clone()
    })
    .bind(&row.id)
    .execute(mysql)
    .await?;

    Ok(json!({
        "type": "character",
        "appearanceId": row.id,
        "imageIndex": image_index,
        "imageUrl": image_key,
    }))
}

async fn resolve_location_modify_row(task: &TaskContext) -> Result<LocationModifyRow, AppError> {
    let payload = &task.payload;
    let mysql = runtime::mysql()?;

    if let Some(location_image_id) = shared::read_string(payload, "locationImageId")
        .or_else(|| shared::read_string(payload, "targetId"))
        .or_else(|| {
            let value = task.target_id.trim();
            if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        })
    {
        let row = sqlx::query_as::<_, LocationModifyRow>(
            "SELECT li.id, li.imageUrl, np.projectId, p.userId FROM location_images li INNER JOIN novel_promotion_locations l ON l.id = li.locationId INNER JOIN novel_promotion_projects np ON np.id = l.novelPromotionProjectId INNER JOIN projects p ON p.id = np.projectId WHERE li.id = ? LIMIT 1",
        )
        .bind(&location_image_id)
        .fetch_optional(mysql)
        .await?;
        if let Some(item) = row {
            return Ok(item);
        }
    }

    let location_id = shared::read_string(payload, "locationId")
        .or_else(|| shared::read_string(payload, "id"))
        .ok_or_else(|| AppError::invalid_params("locationId is required"))?;
    let image_index = shared::read_i32(payload, "imageIndex").unwrap_or(0);

    sqlx::query_as::<_, LocationModifyRow>(
        "SELECT li.id, li.imageUrl, np.projectId, p.userId FROM location_images li INNER JOIN novel_promotion_locations l ON l.id = li.locationId INNER JOIN novel_promotion_projects np ON np.id = l.novelPromotionProjectId INNER JOIN projects p ON p.id = np.projectId WHERE li.locationId = ? AND li.imageIndex = ? LIMIT 1",
    )
    .bind(location_id)
    .bind(image_index)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("location image not found"))
}

async fn handle_location_modify(task: &TaskContext) -> Result<Value, AppError> {
    let payload = &task.payload;
    let mysql = runtime::mysql()?;
    let row = resolve_location_modify_row(task).await?;

    let project_models = shared::get_project_models(&row.project_id, &row.user_id).await?;
    let edit_model = shared::read_string(payload, "imageModel")
        .or(project_models.edit_model.clone())
        .ok_or_else(|| AppError::invalid_params("edit image model is not configured"))?;

    let source_url = shared::to_fetchable_url(row.image_url.as_deref())
        .ok_or_else(|| AppError::invalid_params("location image source is missing"))?;
    let mut references = vec![source_url];
    references.extend(shared::collect_extra_image_urls(payload));
    let references = shared::normalize_reference_urls(&references).await?;

    let modify_prompt = shared::read_string(payload, "modifyPrompt")
        .ok_or_else(|| AppError::invalid_params("modifyPrompt is required"))?;
    let resolution = payload
        .get("generationOptions")
        .and_then(|item| item.get("resolution"))
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .or(project_models.image_resolution.clone());

    let image_key = shared::generate_image_to_storage(
        &edit_model,
        &format!("Modify location image: {modify_prompt}"),
        ImageGenerateOptions {
            reference_images: references,
            aspect_ratio: Some("1:1".to_string()),
            resolution,
            output_format: Some("png".to_string()),
            quality: None,
        },
        "location-modify",
        &row.id,
    )
    .await?;

    sqlx::query(
        "UPDATE location_images SET previousImageUrl = imageUrl, imageUrl = ?, updatedAt = NOW(3) WHERE id = ?",
    )
    .bind(&image_key)
    .bind(&row.id)
    .execute(mysql)
    .await?;

    Ok(json!({
        "type": "location",
        "locationImageId": row.id,
        "imageUrl": image_key,
    }))
}

async fn resolve_panel_modify_row(task: &TaskContext) -> Result<PanelModifyRow, AppError> {
    let payload = &task.payload;
    let mysql = runtime::mysql()?;
    if let Some(panel_id) = shared::read_string(payload, "panelId")
        .or_else(|| shared::read_string(payload, "targetId"))
        .or_else(|| {
            let value = task.target_id.trim();
            if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        })
    {
        let row = sqlx::query_as::<_, PanelModifyRow>(
            "SELECT panel.id, panel.imageUrl, np.projectId, p.userId FROM novel_promotion_panels panel INNER JOIN novel_promotion_storyboards sb ON sb.id = panel.storyboardId INNER JOIN novel_promotion_clips c ON c.id = sb.clipId INNER JOIN novel_promotion_episodes ep ON ep.id = c.episodeId INNER JOIN novel_promotion_projects np ON np.id = ep.novelPromotionProjectId INNER JOIN projects p ON p.id = np.projectId WHERE panel.id = ? LIMIT 1",
        )
        .bind(&panel_id)
        .fetch_optional(mysql)
        .await?;
        if let Some(item) = row {
            return Ok(item);
        }
    }

    let storyboard_id = shared::read_string(payload, "storyboardId")
        .ok_or_else(|| AppError::invalid_params("storyboardId is required"))?;
    let panel_index = shared::read_i32(payload, "panelIndex")
        .ok_or_else(|| AppError::invalid_params("panelIndex is required"))?;

    sqlx::query_as::<_, PanelModifyRow>(
        "SELECT panel.id, panel.imageUrl, np.projectId, p.userId FROM novel_promotion_panels panel INNER JOIN novel_promotion_storyboards sb ON sb.id = panel.storyboardId INNER JOIN novel_promotion_clips c ON c.id = sb.clipId INNER JOIN novel_promotion_episodes ep ON ep.id = c.episodeId INNER JOIN novel_promotion_projects np ON np.id = ep.novelPromotionProjectId INNER JOIN projects p ON p.id = np.projectId WHERE panel.storyboardId = ? AND panel.panelIndex = ? LIMIT 1",
    )
    .bind(storyboard_id)
    .bind(panel_index)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("storyboard panel not found"))
}

async fn handle_storyboard_modify(task: &TaskContext) -> Result<Value, AppError> {
    let payload = &task.payload;
    let mysql = runtime::mysql()?;
    let row = resolve_panel_modify_row(task).await?;

    let project_models = shared::get_project_models(&row.project_id, &row.user_id).await?;
    let edit_model = shared::read_string(payload, "imageModel")
        .or(project_models.edit_model.clone())
        .ok_or_else(|| AppError::invalid_params("edit image model is not configured"))?;

    let source_url = shared::to_fetchable_url(row.image_url.as_deref())
        .ok_or_else(|| AppError::invalid_params("storyboard image source is missing"))?;
    let mut references = vec![source_url];
    references.extend(shared::collect_extra_image_urls(payload));
    let references = shared::normalize_reference_urls(&references).await?;

    let modify_prompt = shared::read_string(payload, "modifyPrompt")
        .ok_or_else(|| AppError::invalid_params("modifyPrompt is required"))?;
    let resolution = payload
        .get("generationOptions")
        .and_then(|item| item.get("resolution"))
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .or(project_models.image_resolution.clone());

    let image_key = shared::generate_image_to_storage(
        &edit_model,
        &format!("Modify storyboard panel image: {modify_prompt}"),
        ImageGenerateOptions {
            reference_images: references,
            aspect_ratio: Some(project_models.video_ratio.clone()),
            resolution,
            output_format: Some("png".to_string()),
            quality: None,
        },
        "panel-modify",
        &row.id,
    )
    .await?;

    sqlx::query(
        "UPDATE novel_promotion_panels SET previousImageUrl = imageUrl, imageUrl = ?, candidateImages = NULL, updatedAt = NOW(3) WHERE id = ?",
    )
    .bind(&image_key)
    .bind(&row.id)
    .execute(mysql)
    .await?;

    Ok(json!({
        "type": "storyboard",
        "panelId": row.id,
        "imageUrl": image_key,
    }))
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    let payload = &task.payload;
    let modify_type = shared::read_string(payload, "type")
        .ok_or_else(|| AppError::invalid_params("modify type is required"))?;

    match modify_type.as_str() {
        "character" => handle_character_modify(task).await,
        "location" => handle_location_modify(task).await,
        "storyboard" => handle_storyboard_modify(task).await,
        _ => Err(AppError::invalid_params(format!(
            "unsupported modify type: {modify_type}"
        ))),
    }
}
