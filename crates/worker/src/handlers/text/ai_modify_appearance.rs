use serde_json::{Value, json};
use waoowaoo_core::errors::AppError;
use waoowaoo_core::prompt_i18n::{PromptIds, PromptVariables};

use crate::task_context::TaskContext;

use super::shared;

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    let payload = &task.payload;
    let character_id = shared::read_string(payload, "characterId")
        .ok_or_else(|| AppError::invalid_params("characterId is required"))?;
    let appearance_id = shared::read_string(payload, "appearanceId")
        .ok_or_else(|| AppError::invalid_params("appearanceId is required"))?;
    let current_description = shared::read_string(payload, "currentDescription")
        .ok_or_else(|| AppError::invalid_params("currentDescription is required"))?;
    let modify_instruction = shared::read_string(payload, "modifyInstruction")
        .ok_or_else(|| AppError::invalid_params("modifyInstruction is required"))?;
    let analysis_model = shared::resolve_analysis_model(task, payload).await?;

    let _ = task
        .report_progress(22, Some("ai_modify_appearance_prepare"))
        .await?;

    let mut prompt_variables = PromptVariables::new();
    prompt_variables.insert(
        "character_input".to_string(),
        shared::remove_character_prompt_suffix(&current_description),
    );
    prompt_variables.insert("user_input".to_string(), modify_instruction.clone());
    let prompt =
        shared::render_prompt_template(payload, PromptIds::NP_CHARACTER_MODIFY, &prompt_variables)?;

    let response = shared::chat(task, &analysis_model, &prompt).await?;
    let parsed = shared::parse_json_object_response(&response)?;
    let modified_description = parsed
        .get("prompt")
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .ok_or_else(|| AppError::invalid_params("ai modify appearance response missing prompt"))?;

    let _ = task
        .report_progress(96, Some("ai_modify_appearance_done"))
        .await?;

    Ok(json!({
        "success": true,
        "characterId": character_id,
        "appearanceId": appearance_id,
        "modifiedDescription": modified_description,
        "originalPrompt": prompt,
        "rawResponse": response,
        "model": analysis_model,
    }))
}
