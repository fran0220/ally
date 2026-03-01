use serde_json::{Value, json};
use waoowaoo_core::errors::AppError;
use waoowaoo_core::prompt_i18n::{PromptIds, PromptVariables};

use crate::task_context::TaskContext;

use super::shared;

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    let payload = &task.payload;
    let modify_instruction = shared::read_string(payload, "modifyInstruction")
        .ok_or_else(|| AppError::invalid_params("modifyInstruction is required"))?;
    let current_description = shared::read_string(payload, "currentDescription")
        .ok_or_else(|| AppError::invalid_params("currentDescription is required"))?;
    let analysis_model = shared::resolve_analysis_model(task, payload).await?;

    let _ = task
        .report_progress(25, Some("asset_hub_ai_modify_prepare"))
        .await?;

    let (target_type, target_id, prompt_variables, prompt_id) = match task.task_type.as_str() {
        "asset_hub_ai_modify_location" => {
            let target_id = shared::read_string(payload, "locationId")
                .ok_or_else(|| AppError::invalid_params("locationId is required"))?;
            let location_name =
                shared::read_string(payload, "locationName").unwrap_or_else(|| "场景".to_string());

            let mut prompt_variables = PromptVariables::new();
            prompt_variables.insert("location_name".to_string(), location_name);
            prompt_variables.insert(
                "location_input".to_string(),
                shared::remove_location_prompt_suffix(&current_description),
            );
            prompt_variables.insert("user_input".to_string(), modify_instruction.clone());

            (
                "location",
                target_id,
                prompt_variables,
                PromptIds::NP_LOCATION_MODIFY,
            )
        }
        "asset_hub_ai_modify_character" => {
            let target_id = shared::read_string(payload, "characterId")
                .ok_or_else(|| AppError::invalid_params("characterId is required"))?;

            let mut prompt_variables = PromptVariables::new();
            prompt_variables.insert(
                "character_input".to_string(),
                shared::remove_character_prompt_suffix(&current_description),
            );
            prompt_variables.insert("user_input".to_string(), modify_instruction.clone());

            (
                "character",
                target_id,
                prompt_variables,
                PromptIds::NP_CHARACTER_MODIFY,
            )
        }
        _ => {
            return Err(AppError::invalid_params(format!(
                "unsupported asset_hub_ai_modify task_type: {}",
                task.task_type
            )));
        }
    };

    let prompt = shared::render_prompt_template(payload, prompt_id, &prompt_variables)?;

    let response = shared::chat(task, &analysis_model, &prompt).await?;
    let parsed = shared::parse_json_object_response(&response)?;
    let modified_description = parsed
        .get("prompt")
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .ok_or_else(|| AppError::invalid_params("ai modify response missing prompt"))?;

    let modified_description = if target_type == "location" {
        shared::remove_location_prompt_suffix(&modified_description)
    } else {
        modified_description
    };

    let _ = task
        .report_progress(96, Some("asset_hub_ai_modify_done"))
        .await?;

    Ok(json!({
        "success": true,
        "targetType": target_type,
        "targetId": target_id,
        "modifiedDescription": modified_description,
        "model": analysis_model,
    }))
}
