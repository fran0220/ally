use serde_json::{Value, json};
use waoowaoo_core::errors::AppError;
use waoowaoo_core::prompt_i18n::{PromptIds, PromptVariables};

use crate::task_context::TaskContext;

use super::shared;

fn infer_asset_type(task: &TaskContext) -> Result<&'static str, AppError> {
    match task.task_type.as_str() {
        "asset_hub_ai_design_character" => Ok("character"),
        "asset_hub_ai_design_location" => Ok("location"),
        _ => Err(AppError::invalid_params(format!(
            "unsupported asset_hub_ai_design task_type: {}",
            task.task_type
        ))),
    }
}

pub async fn handle_for_asset_type(
    task: &TaskContext,
    asset_type: &str,
) -> Result<Value, AppError> {
    let payload = &task.payload;
    let user_instruction = shared::read_string(payload, "userInstruction")
        .ok_or_else(|| AppError::invalid_params("userInstruction is required"))?;
    let analysis_model = shared::resolve_analysis_model(task, payload).await?;

    let _ = task
        .report_progress(25, Some("asset_hub_ai_design_prepare"))
        .await?;

    let prompt_id = match asset_type {
        "character" => PromptIds::NP_CHARACTER_CREATE,
        "location" => PromptIds::NP_LOCATION_CREATE,
        _ => {
            return Err(AppError::invalid_params(format!(
                "unsupported asset type for ai design: {asset_type}",
            )));
        }
    };

    let mut prompt_variables = PromptVariables::new();
    prompt_variables.insert("user_input".to_string(), user_instruction);
    let prompt = shared::render_prompt_template(payload, prompt_id, &prompt_variables)?;

    let response = shared::chat(task, &analysis_model, &prompt).await?;
    let parsed = shared::parse_json_object_response(&response)?;
    let generated_prompt = parsed
        .get("prompt")
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .ok_or_else(|| AppError::invalid_params("ai design response missing prompt"))?;

    let _ = task
        .report_progress(96, Some("asset_hub_ai_design_done"))
        .await?;

    Ok(json!({
        "success": true,
        "prompt": generated_prompt,
        "assetType": asset_type,
        "model": analysis_model,
    }))
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    let asset_type = infer_asset_type(task)?;
    handle_for_asset_type(task, asset_type).await
}
