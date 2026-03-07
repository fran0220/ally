use serde_json::{Value, json};
use sqlx::FromRow;
use waoowaoo_core::errors::AppError;
use waoowaoo_core::generators::ImageGenerateOptions;

use crate::{runtime, task_context::TaskContext};

use super::shared;

#[derive(Debug, FromRow)]
struct GlobalCharacterAppearanceRow {
    id: String,
    #[sqlx(rename = "appearanceIndex")]
    appearance_index: i32,
    description: Option<String>,
    descriptions: Option<String>,
    #[sqlx(rename = "characterName")]
    character_name: String,
}

#[derive(Debug, FromRow)]
struct GlobalLocationImageRow {
    id: String,
    #[sqlx(rename = "locationId")]
    location_id: String,
    description: Option<String>,
}

fn read_resolution(payload: &Value) -> Option<String> {
    payload
        .get("generationOptions")
        .and_then(|item| item.get("resolution"))
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    let mysql = runtime::mysql()?;
    let payload = &task.payload;

    let asset_type = shared::read_string(payload, "type")
        .ok_or_else(|| AppError::invalid_params("type is required"))?
        .to_lowercase();

    let user_models = shared::get_user_models(&task.user_id).await?;
    let resolution = read_resolution(payload);
    let locale = shared::read_locale_tag(payload);
    let style_prompt = shared::read_string(payload, "artStyle")
        .and_then(|item| shared::resolve_art_style_prompt(Some(item.as_str()), locale));

    match asset_type.as_str() {
        "character" => {
            let character_id = shared::read_string(payload, "id")
                .or_else(|| shared::read_string(payload, "characterId"))
                .ok_or_else(|| AppError::invalid_params("character id is required"))?;
            let appearance_index = shared::read_i32(payload, "appearanceIndex").unwrap_or(0);

            let appearances = sqlx::query_as::<_, GlobalCharacterAppearanceRow>(
                "SELECT ca.id, ca.appearanceIndex, ca.description, ca.descriptions, c.name AS characterName FROM global_character_appearances ca INNER JOIN global_characters c ON c.id = ca.characterId WHERE c.id = ? AND c.userId = ? ORDER BY ca.appearanceIndex ASC",
            )
            .bind(&character_id)
            .bind(&task.user_id)
            .fetch_all(mysql)
            .await?;

            if appearances.is_empty() {
                return Err(AppError::not_found("global character appearance not found"));
            }

            let appearance = appearances
                .iter()
                .find(|item| item.appearance_index == appearance_index)
                .or_else(|| appearances.first())
                .ok_or_else(|| AppError::not_found("global character appearance not found"))?;

            let character_model = shared::read_string(payload, "imageModel")
                .or(user_models.character_model.clone())
                .ok_or_else(|| {
                    AppError::invalid_params("character image model is not configured")
                })?;

            let mut descriptions = shared::parse_string_array(appearance.descriptions.as_deref());
            if descriptions.is_empty()
                && let Some(value) = appearance
                    .description
                    .as_deref()
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
            {
                descriptions.push(value.to_string());
            }
            if descriptions.is_empty() {
                descriptions.push(format!(
                    "Character portrait for {}",
                    appearance.character_name
                ));
            }
            descriptions.truncate(descriptions.len().clamp(1, 3));

            let mut generated = Vec::with_capacity(descriptions.len());
            for (index, description) in descriptions.iter().enumerate() {
                let prompt_seed = if description.trim().is_empty() {
                    format!("Character portrait for {}", appearance.character_name)
                } else {
                    description.trim().to_string()
                };
                let mut prompt = shared::add_character_prompt_suffix(&prompt_seed);
                if let Some(style) = style_prompt.as_deref() {
                    prompt.push_str(", ");
                    prompt.push_str(style);
                }

                let image_key = shared::generate_image_to_storage(
                    &character_model,
                    &prompt,
                    ImageGenerateOptions {
                        reference_images: Vec::new(),
                        aspect_ratio: Some("3:2".to_string()),
                        resolution: resolution.clone(),
                        output_format: Some("png".to_string()),
                        quality: None,
                    },
                    "global-character",
                    &format!("{}-{index}", appearance.id),
                )
                .await?;
                generated.push(image_key);
            }

            sqlx::query(
                "UPDATE global_character_appearances SET imageUrls = ?, imageUrl = ?, selectedIndex = NULL, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(serde_json::to_string(&generated).map_err(|err| {
                AppError::internal(format!("failed to encode global character imageUrls: {err}"))
            })?)
            .bind(generated.first().cloned())
            .bind(&appearance.id)
            .execute(mysql)
            .await?;

            Ok(json!({
                "type": "character",
                "appearanceId": appearance.id,
                "imageCount": generated.iter().filter(|value| !value.trim().is_empty()).count(),
            }))
        }
        "location" => {
            let location_id = shared::read_string(payload, "id")
                .or_else(|| shared::read_string(payload, "locationId"))
                .ok_or_else(|| AppError::invalid_params("location id is required"))?;

            let images = sqlx::query_as::<_, GlobalLocationImageRow>(
                "SELECT li.id, li.locationId, li.description FROM global_location_images li INNER JOIN global_locations l ON l.id = li.locationId WHERE l.id = ? AND l.userId = ? ORDER BY li.imageIndex ASC",
            )
            .bind(&location_id)
            .bind(&task.user_id)
            .fetch_all(mysql)
            .await?;

            if images.is_empty() {
                return Err(AppError::not_found("global location image not found"));
            }

            let location_model = shared::read_string(payload, "imageModel")
                .or(user_models.location_model.clone())
                .ok_or_else(|| {
                    AppError::invalid_params("location image model is not configured")
                })?;

            for image in &images {
                let Some(description) = image
                    .description
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| value.to_string())
                else {
                    continue;
                };

                let mut prompt = shared::add_location_prompt_suffix(&description);
                if let Some(style) = style_prompt.as_deref() {
                    prompt.push_str(", ");
                    prompt.push_str(style);
                }

                let image_key = shared::generate_image_to_storage(
                    &location_model,
                    &prompt,
                    ImageGenerateOptions {
                        reference_images: Vec::new(),
                        aspect_ratio: Some("1:1".to_string()),
                        resolution: resolution.clone(),
                        output_format: Some("png".to_string()),
                        quality: None,
                    },
                    "global-location",
                    &image.id,
                )
                .await?;

                sqlx::query(
                    "UPDATE global_location_images SET imageUrl = ?, updatedAt = NOW(3) WHERE id = ?",
                )
                .bind(image_key)
                .bind(&image.id)
                .execute(mysql)
                .await?;
            }

            Ok(json!({
                "type": "location",
                "locationId": images[0].location_id,
                "imageCount": images.len(),
            }))
        }
        _ => Err(AppError::invalid_params(format!(
            "unsupported asset_hub_image type: {asset_type}"
        ))),
    }
}
