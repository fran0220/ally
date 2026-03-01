use serde_json::{Value, json};
use sqlx::FromRow;
use waoowaoo_core::errors::AppError;
use waoowaoo_core::generators::ImageGenerateOptions;

use crate::{runtime, task_context::TaskContext};

use super::shared;

#[derive(Debug, FromRow)]
struct GlobalCharacterModifyRow {
    id: String,
    #[sqlx(rename = "imageUrls")]
    image_urls: Option<String>,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
    #[sqlx(rename = "selectedIndex")]
    selected_index: Option<i32>,
}

#[derive(Debug, FromRow)]
struct GlobalLocationModifyRow {
    id: String,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
}

fn read_resolution(payload: &Value) -> Option<String> {
    payload
        .get("generationOptions")
        .and_then(|item| item.get("resolution"))
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn pick_character_source_key(row: &GlobalCharacterModifyRow, image_index: usize) -> Option<String> {
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

async fn load_character_row(task: &TaskContext) -> Result<GlobalCharacterModifyRow, AppError> {
    let mysql = runtime::mysql()?;
    let payload = &task.payload;

    if let Some(appearance_id) = shared::read_string(payload, "appearanceId")
        .or_else(|| shared::read_string(payload, "targetId"))
    {
        let row = sqlx::query_as::<_, GlobalCharacterModifyRow>(
            "SELECT ca.id, ca.imageUrls, ca.imageUrl, ca.selectedIndex FROM global_character_appearances ca INNER JOIN global_characters c ON c.id = ca.characterId WHERE ca.id = ? AND c.userId = ? LIMIT 1",
        )
        .bind(appearance_id)
        .bind(&task.user_id)
        .fetch_optional(mysql)
        .await?;
        if let Some(item) = row {
            return Ok(item);
        }
    }

    let character_id = shared::read_string(payload, "id")
        .or_else(|| shared::read_string(payload, "characterId"))
        .ok_or_else(|| AppError::invalid_params("character id is required"))?;
    let appearance_index = shared::read_i32(payload, "appearanceIndex").unwrap_or(0);

    sqlx::query_as::<_, GlobalCharacterModifyRow>(
        "SELECT ca.id, ca.imageUrls, ca.imageUrl, ca.selectedIndex FROM global_character_appearances ca INNER JOIN global_characters c ON c.id = ca.characterId WHERE c.id = ? AND c.userId = ? AND ca.appearanceIndex = ? LIMIT 1",
    )
    .bind(character_id)
    .bind(&task.user_id)
    .bind(appearance_index)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("global character appearance not found"))
}

async fn load_location_row(task: &TaskContext) -> Result<GlobalLocationModifyRow, AppError> {
    let mysql = runtime::mysql()?;
    let payload = &task.payload;

    if let Some(location_image_id) = shared::read_string(payload, "locationImageId")
        .or_else(|| shared::read_string(payload, "targetId"))
    {
        let row = sqlx::query_as::<_, GlobalLocationModifyRow>(
            "SELECT li.id, li.imageUrl FROM global_location_images li INNER JOIN global_locations l ON l.id = li.locationId WHERE li.id = ? AND l.userId = ? LIMIT 1",
        )
        .bind(location_image_id)
        .bind(&task.user_id)
        .fetch_optional(mysql)
        .await?;
        if let Some(item) = row {
            return Ok(item);
        }
    }

    let location_id = shared::read_string(payload, "id")
        .or_else(|| shared::read_string(payload, "locationId"))
        .ok_or_else(|| AppError::invalid_params("location id is required"))?;
    let image_index = shared::read_i32(payload, "imageIndex").unwrap_or(0);

    sqlx::query_as::<_, GlobalLocationModifyRow>(
        "SELECT li.id, li.imageUrl FROM global_location_images li INNER JOIN global_locations l ON l.id = li.locationId WHERE l.id = ? AND l.userId = ? AND li.imageIndex = ? LIMIT 1",
    )
    .bind(location_id)
    .bind(&task.user_id)
    .bind(image_index)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("global location image not found"))
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    let mysql = runtime::mysql()?;
    let payload = &task.payload;
    let user_models = shared::get_user_models(&task.user_id).await?;
    let edit_model = shared::read_string(payload, "imageModel")
        .or(user_models.edit_model.clone())
        .ok_or_else(|| AppError::invalid_params("edit image model is not configured"))?;
    let resolution = read_resolution(payload);

    let asset_type = shared::read_string(payload, "type")
        .ok_or_else(|| AppError::invalid_params("type is required"))?
        .to_lowercase();

    match asset_type.as_str() {
        "character" => {
            let row = load_character_row(task).await?;
            let image_index = shared::read_usize(payload, "imageIndex")
                .or_else(|| {
                    row.selected_index
                        .and_then(|value| (value >= 0).then_some(value as usize))
                })
                .unwrap_or(0);

            let source_key = pick_character_source_key(&row, image_index).ok_or_else(|| {
                AppError::invalid_params("global character source image is missing")
            })?;

            let source_url = shared::to_fetchable_url(Some(&source_key))
                .ok_or_else(|| AppError::invalid_params("invalid global character source image"))?;
            let mut references = vec![source_url];
            references.extend(shared::collect_extra_image_urls(payload));
            let references = shared::normalize_reference_urls(&references).await?;

            let modify_prompt = shared::read_string(payload, "modifyPrompt")
                .ok_or_else(|| AppError::invalid_params("modifyPrompt is required"))?;

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
                "global-character-modify",
                &format!("{}-{image_index}", row.id),
            )
            .await?;

            let mut image_urls = shared::parse_image_urls(row.image_urls.as_deref());
            if image_urls.len() <= image_index {
                image_urls.resize(image_index + 1, String::new());
            }
            image_urls[image_index] = image_key.clone();

            let should_update_main = row
                .selected_index
                .map(|value| value >= 0 && value as usize == image_index)
                .unwrap_or(true);

            sqlx::query(
                "UPDATE global_character_appearances SET previousImageUrl = imageUrl, previousImageUrls = imageUrls, imageUrls = ?, imageUrl = ?, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(serde_json::to_string(&image_urls).map_err(|err| {
                AppError::internal(format!("failed to encode global character imageUrls: {err}"))
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
                "imageUrl": image_key,
            }))
        }
        "location" => {
            let row = load_location_row(task).await?;
            let source_url =
                shared::to_fetchable_url(row.image_url.as_deref()).ok_or_else(|| {
                    AppError::invalid_params("global location source image is missing")
                })?;
            let mut references = vec![source_url];
            references.extend(shared::collect_extra_image_urls(payload));
            let references = shared::normalize_reference_urls(&references).await?;

            let modify_prompt = shared::read_string(payload, "modifyPrompt")
                .ok_or_else(|| AppError::invalid_params("modifyPrompt is required"))?;

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
                "global-location-modify",
                &row.id,
            )
            .await?;

            sqlx::query(
                "UPDATE global_location_images SET previousImageUrl = imageUrl, imageUrl = ?, updatedAt = NOW(3) WHERE id = ?",
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
        _ => Err(AppError::invalid_params(format!(
            "unsupported asset_hub_modify type: {asset_type}"
        ))),
    }
}
