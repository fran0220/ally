use serde_json::{Value, json};
use sqlx::FromRow;
use waoowaoo_core::{errors::AppError, generators::ImageGenerateOptions};

use crate::{runtime, task_context::TaskContext};

use super::shared;

#[derive(Debug, FromRow, Clone)]
struct LocationImageContextRow {
    id: String,
    #[sqlx(rename = "locationId")]
    location_id: String,
    #[sqlx(rename = "imageIndex")]
    image_index: i32,
    description: Option<String>,
    #[sqlx(rename = "locationName")]
    location_name: String,
    #[sqlx(rename = "projectId")]
    project_id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
}

async fn load_location_images(
    task: &TaskContext,
) -> Result<Vec<LocationImageContextRow>, AppError> {
    let payload = &task.payload;
    let mysql = runtime::mysql()?;
    let target_id = shared::read_string(payload, "targetId")
        .or_else(|| shared::read_string(payload, "locationImageId"))
        .or_else(|| shared::read_string(payload, "id"))
        .or_else(|| shared::read_string(payload, "locationId"))
        .or_else(|| {
            let value = task.target_id.trim();
            if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        })
        .ok_or_else(|| AppError::invalid_params("location id is required"))?;

    let single_image = sqlx::query_as::<_, LocationImageContextRow>(
        "SELECT li.id, li.locationId, li.imageIndex, li.description, l.name AS locationName, np.projectId, p.userId FROM location_images li INNER JOIN novel_promotion_locations l ON l.id = li.locationId INNER JOIN novel_promotion_projects np ON np.id = l.novelPromotionProjectId INNER JOIN projects p ON p.id = np.projectId WHERE li.id = ? LIMIT 1",
    )
    .bind(&target_id)
    .fetch_optional(mysql)
    .await?;

    if let Some(location_image) = single_image {
        if shared::read_i32(payload, "imageIndex").is_some() {
            return Ok(vec![location_image]);
        }

        let rows = sqlx::query_as::<_, LocationImageContextRow>(
            "SELECT li.id, li.locationId, li.imageIndex, li.description, l.name AS locationName, np.projectId, p.userId FROM location_images li INNER JOIN novel_promotion_locations l ON l.id = li.locationId INNER JOIN novel_promotion_projects np ON np.id = l.novelPromotionProjectId INNER JOIN projects p ON p.id = np.projectId WHERE li.locationId = ? ORDER BY li.imageIndex ASC",
        )
        .bind(&location_image.location_id)
        .fetch_all(mysql)
        .await?;
        return Ok(rows);
    }

    let location_id = shared::read_string(payload, "locationId")
        .or_else(|| shared::read_string(payload, "id"))
        .unwrap_or(target_id);

    let mut rows = sqlx::query_as::<_, LocationImageContextRow>(
        "SELECT li.id, li.locationId, li.imageIndex, li.description, l.name AS locationName, np.projectId, p.userId FROM location_images li INNER JOIN novel_promotion_locations l ON l.id = li.locationId INNER JOIN novel_promotion_projects np ON np.id = l.novelPromotionProjectId INNER JOIN projects p ON p.id = np.projectId WHERE li.locationId = ? ORDER BY li.imageIndex ASC",
    )
    .bind(&location_id)
    .fetch_all(mysql)
    .await?;

    if let Some(image_index) = shared::read_i32(payload, "imageIndex") {
        rows.retain(|item| item.image_index == image_index);
    }

    if rows.is_empty() {
        return Err(AppError::not_found("location image not found"));
    }

    Ok(rows)
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    let payload = &task.payload;
    let mysql = runtime::mysql()?;
    let images = load_location_images(task).await?;
    let context = images
        .first()
        .cloned()
        .ok_or_else(|| AppError::not_found("location image not found"))?;

    let project_models = shared::get_project_models(&context.project_id, &context.user_id).await?;
    let location_model = shared::read_string(payload, "imageModel")
        .or(project_models.location_model.clone())
        .ok_or_else(|| AppError::invalid_params("location image model is not configured"))?;

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

    let mut location_ids = Vec::new();
    for image in &images {
        if !location_ids.contains(&image.location_id) {
            location_ids.push(image.location_id.clone());
        }
    }

    let total_images = images.len();
    let image_count = total_images.max(1) as i32;
    for (loop_index, image) in images.into_iter().enumerate() {
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
        if prompt.is_empty() {
            prompt = format!("Location concept for {}", image.location_name);
        }

        if let Some(style) = style_prompt.as_deref() {
            prompt.push_str(", ");
            prompt.push_str(style);
        }

        let progress = 20 + ((loop_index as i32) * 55 / image_count);
        let _ = task
            .report_progress(progress, Some("generate_location_image"))
            .await?;

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
            "location",
            &image.id,
        )
        .await?;

        sqlx::query("UPDATE location_images SET imageUrl = ?, updatedAt = NOW(3) WHERE id = ?")
            .bind(image_key)
            .bind(&image.id)
            .execute(mysql)
            .await?;
    }

    Ok(json!({
        "updated": total_images,
        "locationIds": location_ids,
    }))
}
