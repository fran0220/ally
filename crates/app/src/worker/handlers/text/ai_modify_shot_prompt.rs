use serde_json::{Value, json};
use waoowaoo_core::errors::AppError;
use waoowaoo_core::prompt_i18n::{PromptIds, PromptLocale, PromptVariables};

use crate::task_context::TaskContext;

use super::shared;

fn collect_asset_descriptions(payload: &Value, locale: PromptLocale) -> Option<String> {
    let assets = payload.get("referencedAssets")?.as_array()?;
    let descriptions = assets
        .iter()
        .filter_map(|asset| {
            let record = asset.as_object()?;
            let name = record
                .get("name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .unwrap_or("");
            let description = record
                .get("description")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .unwrap_or("");

            if name.is_empty() && description.is_empty() {
                None
            } else {
                Some(format!("{name}({description})"))
            }
        })
        .collect::<Vec<_>>();

    if descriptions.is_empty() {
        None
    } else {
        Some(descriptions.join(shared::l(locale, "，", ", ")))
    }
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    let payload = &task.payload;
    let locale = shared::resolve_prompt_locale(payload);
    let current_prompt = shared::read_string(payload, "currentPrompt")
        .ok_or_else(|| AppError::invalid_params("currentPrompt is required"))?;
    let current_video_prompt =
        shared::read_string(payload, "currentVideoPrompt").unwrap_or_default();
    let modify_instruction = shared::read_string(payload, "modifyInstruction")
        .ok_or_else(|| AppError::invalid_params("modifyInstruction is required"))?;
    let analysis_model = shared::resolve_analysis_model(task, payload).await?;
    let referenced_assets = payload
        .get("referencedAssets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let _ = task
        .report_progress(22, Some("ai_modify_shot_prompt_prepare"))
        .await?;

    let user_input = match collect_asset_descriptions(payload, locale) {
        Some(asset_descriptions) => {
            format!(
                "{modify_instruction}\n\n{} {asset_descriptions}",
                shared::l(locale, "引用的资产描述：", "Referenced asset descriptions:")
            )
        }
        None => modify_instruction,
    };

    let mut prompt_variables = PromptVariables::new();
    prompt_variables.insert("prompt_input".to_string(), current_prompt);
    prompt_variables.insert(
        "video_prompt_input".to_string(),
        if current_video_prompt.is_empty() {
            shared::l(locale, "无", "None").to_string()
        } else {
            current_video_prompt
        },
    );
    prompt_variables.insert("user_input".to_string(), user_input);
    let prompt = shared::render_prompt_template(
        payload,
        PromptIds::NP_IMAGE_PROMPT_MODIFY,
        &prompt_variables,
    )?;

    let response = shared::chat(task, &analysis_model, &prompt).await?;
    let parsed = shared::parse_json_object_response(&response)?;

    let image_prompt = parsed
        .get("image_prompt")
        .or_else(|| parsed.get("prompt"))
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .ok_or_else(|| {
            AppError::invalid_params("ai modify shot prompt response missing image prompt")
        })?;
    let video_prompt = parsed
        .get("video_prompt")
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .unwrap_or_default();

    let _ = task
        .report_progress(96, Some("ai_modify_shot_prompt_done"))
        .await?;

    Ok(json!({
        "success": true,
        "modifiedImagePrompt": image_prompt,
        "modifiedVideoPrompt": video_prompt,
        "referencedAssets": referenced_assets,
        "model": analysis_model,
    }))
}
