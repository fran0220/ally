use serde_json::{Value, json};
use sqlx::FromRow;
use waoowaoo_core::errors::AppError;
use waoowaoo_core::prompt_i18n::{PromptIds, PromptVariables};

use crate::{runtime, task_context::TaskContext};

use super::shared;

#[derive(Debug, FromRow)]
struct LocationRow {
    name: String,
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    shared::ensure_novel_project(task).await?;
    let mysql = runtime::mysql()?;
    let payload = &task.payload;
    let location_id = shared::read_string(payload, "locationId")
        .ok_or_else(|| AppError::invalid_params("locationId is required"))?;
    let image_index = shared::read_i32(payload, "imageIndex").unwrap_or(0).max(0);
    let current_description = shared::read_string(payload, "currentDescription")
        .ok_or_else(|| AppError::invalid_params("currentDescription is required"))?;
    let modify_instruction = shared::read_string(payload, "modifyInstruction")
        .ok_or_else(|| AppError::invalid_params("modifyInstruction is required"))?;
    let novel_project = shared::get_novel_project(task).await?;

    let location = sqlx::query_as::<_, LocationRow>(
        "SELECT name FROM novel_promotion_locations WHERE id = ? AND novelPromotionProjectId = ? LIMIT 1",
    )
    .bind(&location_id)
    .bind(&novel_project.id)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("location not found"))?;

    let analysis_model = shared::resolve_analysis_model(task, payload).await?;

    let _ = task
        .report_progress(22, Some("ai_modify_location_prepare"))
        .await?;

    let mut prompt_variables = PromptVariables::new();
    prompt_variables.insert("location_name".to_string(), location.name.clone());
    prompt_variables.insert(
        "location_input".to_string(),
        shared::remove_location_prompt_suffix(&current_description),
    );
    prompt_variables.insert("user_input".to_string(), modify_instruction.clone());
    let prompt =
        shared::render_prompt_template(payload, PromptIds::NP_LOCATION_MODIFY, &prompt_variables)?;

    let response = shared::chat(task, &analysis_model, &prompt).await?;
    let parsed = shared::parse_json_object_response(&response)?;
    let modified_description = parsed
        .get("prompt")
        .and_then(Value::as_str)
        .map(shared::remove_location_prompt_suffix)
        .filter(|item| !item.is_empty())
        .ok_or_else(|| AppError::invalid_params("ai modify location response missing prompt"))?;

    let updated = sqlx::query(
        "UPDATE location_images SET description = ?, updatedAt = NOW(3) WHERE locationId = ? AND imageIndex = ?",
    )
    .bind(&modified_description)
    .bind(&location_id)
    .bind(image_index)
    .execute(mysql)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(AppError::not_found("location image not found"));
    }

    let _ = task
        .report_progress(96, Some("ai_modify_location_done"))
        .await?;

    Ok(json!({
        "success": true,
        "locationId": location_id,
        "imageIndex": image_index,
        "prompt": prompt,
        "modifiedDescription": modified_description,
        "model": analysis_model,
    }))
}
