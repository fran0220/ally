use serde_json::{Map, Value, json};
use sqlx::FromRow;
use waoowaoo_core::errors::AppError;
use waoowaoo_core::prompt_i18n::{PromptIds, PromptVariables};

use crate::{runtime, task_context::TaskContext};

use super::shared;

#[derive(Debug, FromRow)]
struct CharacterRow {
    id: String,
    name: String,
    #[sqlx(rename = "profileData")]
    profile_data: Option<String>,
}

fn read_profile_payload(profile_value: Option<&Value>) -> Option<String> {
    profile_value.and_then(|value| {
        if let Some(raw) = value.as_str() {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        } else if value.is_object() {
            serde_json::to_string(value).ok()
        } else {
            None
        }
    })
}

fn build_character_profile_prompt_payload(
    character_name: &str,
    profile_data: &str,
) -> Result<String, AppError> {
    let parsed_profile = serde_json::from_str::<Value>(profile_data)
        .map_err(|err| AppError::invalid_params(format!("invalid profileData JSON: {err}")))?;
    let profile_object = parsed_profile
        .as_object()
        .cloned()
        .ok_or_else(|| AppError::invalid_params("profileData must be a JSON object"))?;

    let mut character_profile = Map::new();
    character_profile.insert(
        "name".to_string(),
        Value::String(character_name.to_string()),
    );
    for (key, value) in profile_object {
        character_profile.insert(key, value);
    }

    serde_json::to_string_pretty(&vec![Value::Object(character_profile)]).map_err(|err| {
        AppError::internal(format!("failed to encode character profile data: {err}"))
    })
}

fn extract_appearances_from_response(parsed: &Value) -> Vec<Value> {
    parsed
        .get("characters")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|character| character.get("appearances"))
        .and_then(Value::as_array)
        .cloned()
        .or_else(|| parsed.get("appearances").and_then(Value::as_array).cloned())
        .unwrap_or_default()
}

async fn confirm_character(
    task: &TaskContext,
    character_id: &str,
    suppress_progress: bool,
) -> Result<Value, AppError> {
    let mysql = runtime::mysql()?;
    let payload = &task.payload;
    let novel_project = shared::get_novel_project(task).await?;
    let analysis_model = shared::resolve_analysis_model(task, payload).await?;

    if !suppress_progress {
        let _ = task
            .report_progress(20, Some("character_profile_confirm_prepare"))
            .await?;
    }

    let character = sqlx::query_as::<_, CharacterRow>(
        "SELECT id, name, profileData, profileConfirmed FROM novel_promotion_characters WHERE id = ? AND novelPromotionProjectId = ? LIMIT 1",
    )
    .bind(character_id)
    .bind(&novel_project.id)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("character not found"))?;

    let payload_profile_data = read_profile_payload(payload.get("profileData"));
    let profile_data = payload_profile_data
        .clone()
        .or(character.profile_data.clone())
        .ok_or_else(|| AppError::invalid_params("character profileData is required"))?;

    if let Some(requested_profile_data) = payload_profile_data {
        sqlx::query(
            "UPDATE novel_promotion_characters SET profileData = ?, updatedAt = NOW(3) WHERE id = ?",
        )
        .bind(requested_profile_data)
        .bind(&character.id)
        .execute(mysql)
        .await?;
    }

    let character_profiles =
        build_character_profile_prompt_payload(&character.name, &profile_data)?;
    let mut prompt_variables = PromptVariables::new();
    prompt_variables.insert("character_profiles".to_string(), character_profiles);
    let prompt = shared::render_prompt_template(
        payload,
        PromptIds::NP_AGENT_CHARACTER_VISUAL,
        &prompt_variables,
    )?;

    let response = shared::chat(task, &analysis_model, &prompt).await?;
    let parsed = shared::parse_json_object_response(&response)?;
    let appearances = extract_appearances_from_response(&parsed);
    if appearances.is_empty() {
        return Err(AppError::invalid_params(
            "character profile confirm returned empty appearances",
        ));
    }

    if !suppress_progress {
        let _ = task
            .report_progress(78, Some("character_profile_confirm_persist"))
            .await?;
    }

    for (index, appearance) in appearances.iter().enumerate() {
        let change_reason = appearance
            .get("change_reason")
            .or_else(|| appearance.get("changeReason"))
            .and_then(Value::as_str)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "初始形象".to_string());
        let descriptions = appearance
            .get("descriptions")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str())
                    .map(|item| item.trim().to_string())
                    .filter(|item| !item.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let first_description = descriptions.first().cloned().unwrap_or_default();

        sqlx::query(
            "INSERT INTO character_appearances (id, characterId, appearanceIndex, changeReason, description, descriptions, imageUrls, previousImageUrls, createdAt, updatedAt) VALUES (UUID(), ?, ?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
        )
        .bind(&character.id)
        .bind(index as i32)
        .bind(change_reason)
        .bind(first_description)
        .bind(serde_json::to_string(&descriptions).map_err(|err| {
            AppError::internal(format!("failed to encode character descriptions: {err}"))
        })?)
        .bind("[]")
        .bind("[]")
        .execute(mysql)
        .await?;
    }

    sqlx::query(
        "UPDATE novel_promotion_characters SET profileConfirmed = TRUE, updatedAt = NOW(3) WHERE id = ?",
    )
    .bind(&character.id)
    .execute(mysql)
    .await?;

    if !suppress_progress {
        let _ = task
            .report_progress(96, Some("character_profile_confirm_done"))
            .await?;
    }

    Ok(json!({
        "id": character.id,
        "name": character.name,
        "profileData": profile_data,
        "profileConfirmed": true,
        "appearances": appearances,
    }))
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    shared::ensure_novel_project(task).await?;
    let mysql = runtime::mysql()?;
    let payload = &task.payload;
    let novel_project = shared::get_novel_project(task).await?;

    match task.task_type.as_str() {
        "character_profile_batch_confirm" => {
            let characters = sqlx::query_as::<_, CharacterRow>(
                "SELECT id, name, profileData, profileConfirmed FROM novel_promotion_characters WHERE novelPromotionProjectId = ? AND profileConfirmed = FALSE AND profileData IS NOT NULL",
            )
            .bind(&novel_project.id)
            .fetch_all(mysql)
            .await?;

            if characters.is_empty() {
                return Ok(json!({
                    "success": true,
                    "count": 0,
                    "message": "没有待确认的角色",
                }));
            }

            let _ = task
                .report_progress(18, Some("character_profile_batch_prepare"))
                .await?;

            let mut confirmed = 0usize;
            let total = characters.len();
            for (index, character) in characters.iter().enumerate() {
                let progress = 18 + (((index + 1) as i32 * 78) / i32::try_from(total).unwrap_or(1));
                let _ = task
                    .report_progress(progress, Some("character_profile_batch_loop_character"))
                    .await?;

                let _ = confirm_character(task, &character.id, true).await?;
                confirmed += 1;
            }

            let _ = task
                .report_progress(96, Some("character_profile_batch_done"))
                .await?;

            Ok(json!({
                "success": true,
                "count": confirmed,
                "total": total,
            }))
        }
        _ => {
            let character_id = shared::read_string(payload, "characterId")
                .ok_or_else(|| AppError::invalid_params("characterId is required"))?;
            let character = confirm_character(task, &character_id, false).await?;

            Ok(json!({
                "success": true,
                "character": character,
            }))
        }
    }
}
