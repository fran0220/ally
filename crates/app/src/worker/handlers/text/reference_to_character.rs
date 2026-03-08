use serde_json::{Value, json};
use waoowaoo_core::errors::AppError;
use waoowaoo_core::generators::ImageGenerateOptions;
use waoowaoo_core::media;
use waoowaoo_core::prompt_i18n::{PromptIds, PromptVariables};

use crate::{handlers::image::shared as image_shared, runtime, task_context::TaskContext};

use super::shared;

fn parse_reference_images(payload: &Value) -> Vec<String> {
    let mut refs = payload
        .get("referenceImageUrls")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if refs.is_empty()
        && let Some(single) = shared::read_string(payload, "referenceImageUrl")
    {
        refs.push(single);
    }

    refs.truncate(5);
    refs
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    let mysql = runtime::mysql()?;
    let payload = &task.payload;
    let locale = shared::resolve_prompt_locale(payload);
    let is_asset_hub = task.task_type == "asset_hub_reference_to_character";
    let references = parse_reference_images(payload);
    if references.is_empty() {
        return Err(AppError::invalid_params(
            "referenceImageUrl or referenceImageUrls is required",
        ));
    }

    let _ = task
        .report_progress(15, Some("reference_to_character_prepare"))
        .await?;

    let extract_only = shared::read_bool(payload, "extractOnly");
    let normalized_references = image_shared::normalize_reference_urls(&references).await?;
    let analysis_prompt = shared::render_prompt_template(
        payload,
        PromptIds::CHARACTER_IMAGE_TO_DESCRIPTION,
        &PromptVariables::new(),
    )?;

    if extract_only {
        let _ = task
            .report_progress(45, Some("reference_to_character_extract"))
            .await?;

        let analysis_model = shared::resolve_analysis_model(task, payload).await?;
        let description = shared::vision_chat(
            task,
            &analysis_model,
            &analysis_prompt,
            &normalized_references,
        )
        .await?;

        let _ = task
            .report_progress(96, Some("reference_to_character_extract_done"))
            .await?;

        return Ok(json!({
            "success": true,
            "description": description,
            "extractOnly": true,
            "model": analysis_model,
        }));
    }

    let user_models = image_shared::get_user_models(&task.user_id).await?;
    let image_model = shared::read_string(payload, "imageModel")
        .or(user_models.character_model)
        .ok_or_else(|| AppError::invalid_params("character image model is not configured"))?;

    let custom_description = shared::read_string(payload, "customDescription");
    let character_name = shared::read_string(payload, "characterName").unwrap_or_else(|| {
        shared::l(locale, "新角色 - 初始形象", "New Character - Initial Look").to_string()
    });
    let art_style = shared::read_string(payload, "artStyle");
    let mut prompt = if let Some(description) = custom_description.clone() {
        description
    } else {
        shared::render_prompt_template(
            payload,
            PromptIds::CHARACTER_REFERENCE_TO_SHEET,
            &PromptVariables::new(),
        )?
    };
    prompt = shared::add_character_prompt_suffix_for_locale(&prompt, locale);
    if let Some(style) = art_style {
        prompt = format!("{prompt}{}{style}", shared::l(locale, "，", ", "));
    }

    let _ = task
        .report_progress(35, Some("reference_to_character_generate"))
        .await?;

    let mut generated_keys = Vec::new();
    let mut last_generate_error: Option<AppError> = None;
    let use_reference_images = custom_description.is_none();
    let key_prefix = if is_asset_hub {
        "ref-char".to_string()
    } else {
        format!("proj-ref-char-{}", task.project_id)
    };
    for index in 0..3 {
        let result = image_shared::generate_image_to_storage(
            &image_model,
            &prompt,
            ImageGenerateOptions {
                reference_images: if use_reference_images {
                    normalized_references.clone()
                } else {
                    Vec::new()
                },
                aspect_ratio: Some("3:2".to_string()),
                resolution: None,
                output_format: Some("png".to_string()),
                quality: None,
            },
            &key_prefix,
            &format!("{}-{index}", uuid::Uuid::new_v4()),
        )
        .await;

        match result {
            Ok(key) => generated_keys.push(key),
            Err(err) => last_generate_error = Some(err),
        }
    }

    if generated_keys.is_empty() {
        return Err(last_generate_error.unwrap_or_else(|| {
            AppError::internal("reference_to_character failed to generate images")
        }));
    }

    let analysis_description = match shared::resolve_optional_analysis_model(task, payload).await? {
        Some(model) => {
            let completion =
                shared::vision_chat(task, &model, &analysis_prompt, &normalized_references).await?;
            Some(completion)
        }
        None => None,
    };

    let is_background_job = shared::read_bool(payload, "isBackgroundJob");
    if is_background_job {
        let _character_id = shared::read_string(payload, "characterId").ok_or_else(|| {
            AppError::invalid_params("characterId is required for background job")
        })?;
        let appearance_id = shared::read_string(payload, "appearanceId").ok_or_else(|| {
            AppError::invalid_params("appearanceId is required for background job")
        })?;

        if is_asset_hub {
            let updated = sqlx::query(
                "UPDATE global_character_appearances SET imageUrl = ?, imageUrls = ?, description = COALESCE(?, description), updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(generated_keys.first().cloned())
            .bind(serde_json::to_string(&generated_keys).map_err(|err| {
                AppError::internal(format!("failed to encode global imageUrls: {err}"))
            })?)
            .bind(analysis_description)
            .bind(&appearance_id)
            .execute(mysql)
            .await?;
            if updated.rows_affected() == 0 {
                return Err(AppError::not_found("global character appearance not found"));
            }
        } else {
            let updated = sqlx::query(
                "UPDATE character_appearances SET imageUrl = ?, imageUrls = ?, description = COALESCE(?, description), updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(generated_keys.first().cloned())
            .bind(serde_json::to_string(&generated_keys).map_err(|err| {
                AppError::internal(format!("failed to encode project imageUrls: {err}"))
            })?)
            .bind(analysis_description)
            .bind(&appearance_id)
            .execute(mysql)
            .await?;
            if updated.rows_affected() == 0 {
                return Err(AppError::not_found("character appearance not found"));
            }
        }

        let _ = task
            .report_progress(96, Some("reference_to_character_done"))
            .await?;

        return Ok(json!({
            "success": true,
            "updated": true,
        }));
    }

    let _ = task
        .report_progress(96, Some("reference_to_character_done"))
        .await?;

    let first_key = generated_keys.first().cloned();
    let image_url = media::to_public_media_url(first_key.as_deref()).or(first_key.clone());

    Ok(json!({
        "success": true,
        "characterName": character_name,
        "imageUrl": image_url,
        "cosKey": first_key,
        "cosKeys": generated_keys,
        "description": analysis_description,
        "model": image_model,
    }))
}
