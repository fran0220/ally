use serde_json::{Value, json};
use sqlx::FromRow;
use waoowaoo_core::{errors::AppError, generators::ImageGenerateOptions};

use crate::{runtime, task_context::TaskContext};

use super::shared;

#[derive(Debug, FromRow, Clone)]
struct AppearanceContextRow {
    id: String,
    #[sqlx(rename = "characterId")]
    character_id: String,
    #[sqlx(rename = "appearanceIndex")]
    appearance_index: i32,
    descriptions: Option<String>,
    description: Option<String>,
    #[sqlx(rename = "imageUrls")]
    image_urls: Option<String>,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
    #[sqlx(rename = "selectedIndex")]
    selected_index: Option<i32>,
    #[sqlx(rename = "characterName")]
    character_name: String,
    #[sqlx(rename = "projectId")]
    project_id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
}

#[derive(Debug, FromRow)]
struct PrimaryAppearanceRow {
    #[sqlx(rename = "imageUrls")]
    image_urls: Option<String>,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
    #[sqlx(rename = "selectedIndex")]
    selected_index: Option<i32>,
}

fn pick_selected_image_key(primary: &PrimaryAppearanceRow) -> Option<String> {
    let image_urls = shared::parse_image_urls(primary.image_urls.as_deref());
    if let Some(selected_index) = primary.selected_index
        && selected_index >= 0
        && let Some(value) = image_urls.get(selected_index as usize)
        && !value.trim().is_empty()
    {
        return Some(value.trim().to_string());
    }
    image_urls
        .into_iter()
        .find(|item| !item.trim().is_empty())
        .or_else(|| primary.image_url.clone())
}

async fn resolve_appearance(task: &TaskContext) -> Result<AppearanceContextRow, AppError> {
    let payload = &task.payload;
    let mysql = runtime::mysql()?;

    if let Some(appearance_id) = shared::read_string(payload, "appearanceId")
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
        let row = sqlx::query_as::<_, AppearanceContextRow>(
            "SELECT ca.id, ca.characterId, ca.appearanceIndex, ca.descriptions, ca.description, ca.imageUrls, ca.imageUrl, ca.selectedIndex, c.name AS characterName, np.projectId, p.userId FROM character_appearances ca INNER JOIN novel_promotion_characters c ON c.id = ca.characterId INNER JOIN novel_promotion_projects np ON np.id = c.novelPromotionProjectId INNER JOIN projects p ON p.id = np.projectId WHERE ca.id = ? LIMIT 1",
        )
        .bind(&appearance_id)
        .fetch_optional(mysql)
        .await?;
        if let Some(item) = row {
            return Ok(item);
        }
    }

    let character_id = shared::read_string(payload, "id")
        .or_else(|| shared::read_string(payload, "characterId"))
        .or_else(|| {
            let value = task.target_id.trim();
            if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        })
        .ok_or_else(|| AppError::invalid_params("characterId is required"))?;

    sqlx::query_as::<_, AppearanceContextRow>(
        "SELECT ca.id, ca.characterId, ca.appearanceIndex, ca.descriptions, ca.description, ca.imageUrls, ca.imageUrl, ca.selectedIndex, c.name AS characterName, np.projectId, p.userId FROM character_appearances ca INNER JOIN novel_promotion_characters c ON c.id = ca.characterId INNER JOIN novel_promotion_projects np ON np.id = c.novelPromotionProjectId INNER JOIN projects p ON p.id = np.projectId WHERE c.id = ? ORDER BY ca.appearanceIndex ASC LIMIT 1",
    )
    .bind(character_id)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("character appearance not found"))
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    let payload = &task.payload;
    let mysql = runtime::mysql()?;
    let appearance = resolve_appearance(task).await?;

    let project_models =
        shared::get_project_models(&appearance.project_id, &appearance.user_id).await?;
    let character_model = shared::read_string(payload, "imageModel")
        .or(project_models.character_model.clone())
        .ok_or_else(|| AppError::invalid_params("character image model is not configured"))?;

    let mut base_descriptions = shared::parse_string_array(appearance.descriptions.as_deref());
    if base_descriptions.is_empty() {
        base_descriptions.push(appearance.description.clone().unwrap_or_default());
    }

    let indexes = if let Some(index) = shared::read_usize(payload, "imageIndex")
        .or_else(|| shared::read_usize(payload, "descriptionIndex"))
    {
        vec![index]
    } else {
        (0..base_descriptions.len().min(3)).collect::<Vec<_>>()
    };

    let mut references = Vec::new();
    if appearance.appearance_index > 0 {
        let primary = sqlx::query_as::<_, PrimaryAppearanceRow>(
            "SELECT imageUrls, imageUrl, selectedIndex FROM character_appearances WHERE characterId = ? AND appearanceIndex = 0 LIMIT 1",
        )
        .bind(&appearance.character_id)
        .fetch_optional(mysql)
        .await?;

        if let Some(primary) = primary
            && let Some(image_key) = pick_selected_image_key(&primary)
            && let Some(url) = shared::to_fetchable_url(Some(&image_key))
        {
            references.push(url);
        }
    }
    let references = shared::normalize_reference_urls(&references).await?;

    let resolution = payload
        .get("generationOptions")
        .and_then(|item| item.get("resolution"))
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .or(project_models.image_resolution.clone());

    let locale = shared::read_locale_tag(payload);
    let style_prompt =
        shared::resolve_art_style_prompt(project_models.art_style.as_deref(), locale);

    let mut next_image_urls = shared::parse_image_urls(appearance.image_urls.as_deref());
    let indexes_len = indexes.len().max(1) as i32;

    for (loop_index, index) in indexes.into_iter().enumerate() {
        let raw_description = base_descriptions
            .get(index)
            .cloned()
            .or_else(|| base_descriptions.first().cloned())
            .unwrap_or_default();

        let prompt_seed = if raw_description.trim().is_empty() {
            format!("Character portrait for {}", appearance.character_name)
        } else {
            raw_description
        };
        let mut prompt = shared::add_character_prompt_suffix(&prompt_seed);
        if let Some(style) = style_prompt.as_deref() {
            prompt.push_str(", ");
            prompt.push_str(style);
        }

        let progress = 15 + ((loop_index as i32) * 55 / indexes_len);
        let _ = task
            .report_progress(progress, Some("generate_character_image"))
            .await?;

        let cos_key = shared::generate_image_to_storage(
            &character_model,
            &prompt,
            ImageGenerateOptions {
                reference_images: references.clone(),
                aspect_ratio: Some("3:2".to_string()),
                resolution: resolution.clone(),
                output_format: Some("png".to_string()),
                quality: None,
            },
            "character",
            &format!("{}-{index}", appearance.id),
        )
        .await?;

        if next_image_urls.len() <= index {
            next_image_urls.resize(index + 1, String::new());
        }
        next_image_urls[index] = cos_key;
    }

    let main_image = appearance
        .selected_index
        .and_then(|value| {
            if value >= 0 {
                next_image_urls.get(value as usize).cloned()
            } else {
                None
            }
        })
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            next_image_urls
                .iter()
                .find(|value| !value.trim().is_empty())
                .cloned()
        })
        .or(appearance.image_url.clone());

    sqlx::query("UPDATE character_appearances SET imageUrls = ?, imageUrl = ?, updatedAt = NOW(3) WHERE id = ?")
        .bind(serde_json::to_string(&next_image_urls).map_err(|err| {
            AppError::internal(format!("failed to encode character imageUrls: {err}"))
        })?)
        .bind(main_image.clone())
        .bind(&appearance.id)
        .execute(mysql)
        .await?;

    Ok(json!({
        "appearanceId": appearance.id,
        "imageCount": next_image_urls.iter().filter(|value| !value.trim().is_empty()).count(),
        "imageUrl": main_image,
    }))
}
