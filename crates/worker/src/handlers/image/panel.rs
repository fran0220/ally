use serde_json::{Value, json};
use sqlx::FromRow;
use waoowaoo_core::{errors::AppError, generators::ImageGenerateOptions};

use crate::{runtime, task_context::TaskContext};

use super::shared;

#[derive(Debug, FromRow)]
struct PanelRow {
    id: String,
    #[sqlx(rename = "panelIndex")]
    panel_index: i32,
    description: Option<String>,
    #[sqlx(rename = "videoPrompt")]
    video_prompt: Option<String>,
    location: Option<String>,
    characters: Option<String>,
    #[sqlx(rename = "srtSegment")]
    srt_segment: Option<String>,
    #[sqlx(rename = "sketchImageUrl")]
    sketch_image_url: Option<String>,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
}

#[derive(Debug, FromRow)]
struct PanelProjectContextRow {
    #[sqlx(rename = "projectId")]
    project_id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
    #[sqlx(rename = "novelPromotionProjectId")]
    novel_promotion_project_id: String,
}

#[derive(Debug, FromRow)]
struct CharacterAppearanceImageRow {
    #[sqlx(rename = "imageUrls")]
    image_urls: Option<String>,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
    #[sqlx(rename = "selectedIndex")]
    selected_index: Option<i32>,
}

#[derive(Debug, FromRow)]
struct LocationImageRow {
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
}

fn build_panel_prompt(panel: &PanelRow, art_style: Option<&str>) -> String {
    let mut prompt = panel
        .video_prompt
        .as_deref()
        .or(panel.description.as_deref())
        .or(panel.srt_segment.as_deref())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            format!(
                "Generate storyboard panel {} for scene composition",
                panel.panel_index
            )
        });

    if let Some(style) = art_style
        && !style.trim().is_empty()
    {
        prompt.push_str(". Visual style: ");
        prompt.push_str(style.trim());
    }

    prompt
}

fn pick_selected_image_key(row: &CharacterAppearanceImageRow) -> Option<String> {
    let image_urls = shared::parse_image_urls(row.image_urls.as_deref());
    if let Some(selected_index) = row.selected_index
        && selected_index >= 0
    {
        let index = selected_index as usize;
        if let Some(value) = image_urls.get(index)
            && !value.trim().is_empty()
        {
            return Some(value.trim().to_string());
        }
    }

    image_urls
        .into_iter()
        .find(|item| !item.trim().is_empty())
        .or_else(|| row.image_url.clone())
}

async fn collect_character_reference_images(
    context: &PanelProjectContextRow,
    panel: &PanelRow,
) -> Result<Vec<String>, AppError> {
    let mysql = runtime::mysql()?;
    let mut refs = Vec::new();

    for character_ref in shared::parse_panel_character_refs(panel.characters.as_deref()) {
        let query = if character_ref.appearance.is_some() {
            sqlx::query_as::<_, CharacterAppearanceImageRow>(
                "SELECT ca.imageUrls, ca.imageUrl, ca.selectedIndex FROM character_appearances ca INNER JOIN novel_promotion_characters c ON c.id = ca.characterId WHERE c.novelPromotionProjectId = ? AND LOWER(c.name) = LOWER(?) AND LOWER(COALESCE(ca.changeReason, '')) = LOWER(?) ORDER BY ca.appearanceIndex ASC LIMIT 1",
            )
            .bind(&context.novel_promotion_project_id)
            .bind(&character_ref.name)
            .bind(character_ref.appearance.unwrap_or_default())
            .fetch_optional(mysql)
            .await?
        } else {
            sqlx::query_as::<_, CharacterAppearanceImageRow>(
                "SELECT ca.imageUrls, ca.imageUrl, ca.selectedIndex FROM character_appearances ca INNER JOIN novel_promotion_characters c ON c.id = ca.characterId WHERE c.novelPromotionProjectId = ? AND LOWER(c.name) = LOWER(?) ORDER BY ca.appearanceIndex ASC LIMIT 1",
            )
            .bind(&context.novel_promotion_project_id)
            .bind(&character_ref.name)
            .fetch_optional(mysql)
            .await?
        };

        if let Some(appearance_row) = query
            && let Some(image_key) = pick_selected_image_key(&appearance_row)
            && let Some(url) = shared::to_fetchable_url(Some(&image_key))
        {
            refs.push(url);
        }
    }

    Ok(refs)
}

async fn collect_location_reference_image(
    context: &PanelProjectContextRow,
    panel: &PanelRow,
) -> Result<Option<String>, AppError> {
    let Some(location_name) = panel
        .location
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };

    let mysql = runtime::mysql()?;
    let row = sqlx::query_as::<_, LocationImageRow>(
        "SELECT li.imageUrl FROM novel_promotion_locations l INNER JOIN location_images li ON li.locationId = l.id WHERE l.novelPromotionProjectId = ? AND LOWER(l.name) = LOWER(?) ORDER BY li.isSelected DESC, li.imageIndex ASC LIMIT 1",
    )
    .bind(&context.novel_promotion_project_id)
    .bind(location_name)
    .fetch_optional(mysql)
    .await?;

    Ok(row
        .and_then(|item| item.image_url)
        .and_then(|value| shared::to_fetchable_url(Some(&value))))
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    let payload = &task.payload;
    let panel_id = shared::read_string(payload, "panelId")
        .or_else(|| shared::read_string(payload, "targetId"))
        .or_else(|| {
            let value = task.target_id.trim();
            if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        })
        .ok_or_else(|| AppError::invalid_params("panelId is required"))?;

    let mysql = runtime::mysql()?;
    let panel = sqlx::query_as::<_, PanelRow>(
        "SELECT id, panelIndex, description, videoPrompt, location, characters, srtSegment, sketchImageUrl, imageUrl FROM novel_promotion_panels WHERE id = ? LIMIT 1",
    )
    .bind(&panel_id)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("panel not found"))?;

    let context = sqlx::query_as::<_, PanelProjectContextRow>(
        "SELECT np.projectId, p.userId, ep.novelPromotionProjectId FROM novel_promotion_panels panel INNER JOIN novel_promotion_storyboards sb ON sb.id = panel.storyboardId INNER JOIN novel_promotion_clips c ON c.id = sb.clipId INNER JOIN novel_promotion_episodes ep ON ep.id = c.episodeId INNER JOIN novel_promotion_projects np ON np.id = ep.novelPromotionProjectId INNER JOIN projects p ON p.id = np.projectId WHERE panel.id = ? LIMIT 1",
    )
    .bind(&panel.id)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("panel project context not found"))?;

    let project_models = shared::get_project_models(&context.project_id, &context.user_id).await?;
    let storyboard_model = shared::read_string(payload, "imageModel")
        .or(project_models.storyboard_model.clone())
        .ok_or_else(|| AppError::invalid_params("storyboard image model is not configured"))?;

    let candidate_count = shared::clamp_count(
        shared::read_i32(payload, "candidateCount").or_else(|| shared::read_i32(payload, "count")),
        1,
        1,
        4,
    );

    let mut references = Vec::new();
    if let Some(sketch_url) = shared::to_fetchable_url(panel.sketch_image_url.as_deref()) {
        references.push(sketch_url);
    }
    references.extend(collect_character_reference_images(&context, &panel).await?);
    if let Some(location_ref) = collect_location_reference_image(&context, &panel).await? {
        references.push(location_ref);
    }
    references = shared::normalize_reference_urls(&references).await?;

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

    let prompt = shared::read_string(payload, "prompt")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| build_panel_prompt(&panel, style_prompt.as_deref()));

    let mut candidates = Vec::new();
    for index in 0..candidate_count {
        let progress = 18 + (index * 58 / candidate_count.max(1));
        let _ = task
            .report_progress(progress, Some("generate_panel_candidate"))
            .await?;

        let candidate_key = shared::generate_image_to_storage(
            &storyboard_model,
            &prompt,
            ImageGenerateOptions {
                reference_images: references.clone(),
                aspect_ratio: Some(project_models.video_ratio.clone()),
                resolution: resolution.clone(),
                output_format: Some("png".to_string()),
                quality: None,
            },
            "panel-candidate",
            &format!("{}-{index}", panel.id),
        )
        .await?;
        candidates.push(candidate_key);
    }

    let candidate_images_json = if candidate_count > 1 {
        Some(serde_json::to_string(&candidates).map_err(|err| {
            AppError::internal(format!("failed to encode panel candidate images: {err}"))
        })?)
    } else {
        None
    };

    let is_first_generation = panel.image_url.is_none();
    if is_first_generation {
        sqlx::query(
            "UPDATE novel_promotion_panels SET imageUrl = ?, candidateImages = ?, updatedAt = NOW(3) WHERE id = ?",
        )
        .bind(candidates.first().cloned())
        .bind(candidate_images_json)
        .bind(&panel.id)
        .execute(mysql)
        .await?;
    } else {
        sqlx::query(
            "UPDATE novel_promotion_panels SET previousImageUrl = ?, candidateImages = ?, updatedAt = NOW(3) WHERE id = ?",
        )
        .bind(panel.image_url.clone())
        .bind(serde_json::to_string(&candidates).map_err(|err| {
            AppError::internal(format!("failed to encode panel candidate images: {err}"))
        })?)
        .bind(&panel.id)
        .execute(mysql)
        .await?;
    }

    Ok(json!({
        "panelId": panel.id,
        "candidateCount": candidates.len(),
        "imageUrl": if is_first_generation { candidates.first().cloned() } else { None },
    }))
}
