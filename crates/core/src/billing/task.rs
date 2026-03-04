use std::collections::HashSet;

use once_cell::sync::Lazy;
use rust_decimal::Decimal;
use serde_json::{Map, Value, json};
use sqlx::MySqlPool;

use crate::errors::AppError;

use super::{
    ledger::{
        ConfirmChargeInput, FreezeBalanceOptions, freeze_balance, get_balance,
        insufficient_balance_error, record_shadow_usage, rollback_freeze,
    },
    money::{decimal_from_f64, decimal_to_f64},
    pricing::quote_task_cost,
    types::{BillingApiType, BillingMode, BillingStatus, TaskBillingInfo, UsageUnit},
};

pub const BUILTIN_PRICING_VERSION: &str = "2026-02-19";

static BILLABLE_TASK_TYPES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "image_panel",
        "image_character",
        "image_location",
        "video_panel",
        "lip_sync",
        "voice_line",
        "voice_design",
        "asset_hub_voice_design",
        "regenerate_storyboard_text",
        "insert_panel",
        "panel_variant",
        "modify_asset_image",
        "regenerate_group",
        "asset_hub_image",
        "asset_hub_modify",
        "analyze_novel",
        "story_to_script_run",
        "script_to_storyboard_run",
        "clips_build",
        "screenplay_convert",
        "voice_analyze",
        "analyze_global",
        "ai_modify_appearance",
        "ai_modify_location",
        "ai_modify_shot_prompt",
        "analyze_shot_variants",
        "ai_create_character",
        "ai_create_location",
        "reference_to_character",
        "character_profile_confirm",
        "character_profile_batch_confirm",
        "episode_split_llm",
        "asset_hub_ai_design_character",
        "asset_hub_ai_design_location",
        "asset_hub_ai_modify_character",
        "asset_hub_ai_modify_location",
        "asset_hub_reference_to_character",
    ])
});

fn value_as_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn value_as_number(value: Option<&Value>) -> Option<f64> {
    let value = value?;
    if let Some(number) = value.as_f64() {
        return Some(number);
    }
    value
        .as_str()
        .and_then(|raw| raw.trim().parse::<f64>().ok())
}

fn first_string(payload: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value_as_string(payload.get(*key)))
}

fn first_number(payload: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter()
        .find_map(|key| value_as_number(payload.get(*key)))
}

fn read_generation_options(payload: &Value) -> Option<&Map<String, Value>> {
    payload.get("generationOptions").and_then(Value::as_object)
}

fn to_positive_count(value: Option<f64>, fallback: f64) -> f64 {
    let resolved = value.unwrap_or(fallback);
    if !resolved.is_finite() {
        return fallback;
    }
    resolved.max(1.0).floor()
}

fn to_non_negative(value: Option<f64>, fallback: f64) -> f64 {
    let resolved = value.unwrap_or(fallback);
    if !resolved.is_finite() {
        return fallback;
    }
    resolved.max(0.0)
}

fn parse_json_object(raw: Option<&Value>) -> Option<Map<String, Value>> {
    let Value::Object(map) = raw? else {
        return None;
    };
    Some(map.clone())
}

fn normalize_metadata(metadata: Option<Map<String, Value>>) -> Option<Value> {
    metadata.filter(|map| !map.is_empty()).map(Value::Object)
}

fn default_text_billing(task_type: &str, payload: &Value) -> Option<TaskBillingInfo> {
    let model = first_string(payload, &["analysisModel", "model"])?;
    let input_tokens = to_non_negative(first_number(payload, &["maxInputTokens"]), 3_000.0).floor();
    let output_tokens =
        to_non_negative(first_number(payload, &["maxOutputTokens"]), 1_200.0).floor();
    let quantity = input_tokens + output_tokens;

    let mut info = TaskBillingInfo::billable(
        task_type.to_string(),
        BillingApiType::Text,
        model,
        quantity,
        UsageUnit::Token,
    );
    info.metadata = normalize_metadata(Some(Map::from_iter([
        ("inputTokens".to_string(), Value::from(input_tokens)),
        ("outputTokens".to_string(), Value::from(output_tokens)),
    ])));
    Some(info)
}

fn default_image_billing(task_type: &str, payload: &Value) -> Option<TaskBillingInfo> {
    let model = first_string(payload, &["imageModel", "modelId", "model"])?;
    let quantity = to_positive_count(first_number(payload, &["candidateCount", "count"]), 1.0);

    let mut metadata = Map::new();
    if let Some(options) = read_generation_options(payload)
        && let Some(resolution) = value_as_string(options.get("resolution"))
    {
        metadata.insert("resolution".to_string(), Value::String(resolution));
    } else if let Some(resolution) = first_string(payload, &["resolution"]) {
        metadata.insert("resolution".to_string(), Value::String(resolution));
    }

    let mut info = TaskBillingInfo::billable(
        task_type.to_string(),
        BillingApiType::Image,
        model,
        quantity,
        UsageUnit::Image,
    );
    info.metadata = normalize_metadata(Some(metadata));
    Some(info)
}

fn default_video_billing(task_type: &str, payload: &Value) -> Option<TaskBillingInfo> {
    let model = first_string(payload, &["videoModel", "modelId", "model", "flModel"])?;
    let quantity = to_positive_count(first_number(payload, &["count"]), 1.0);

    let mut metadata = Map::new();
    if let Some(options) = read_generation_options(payload) {
        if let Some(resolution) = value_as_string(options.get("resolution")) {
            metadata.insert("resolution".to_string(), Value::String(resolution));
        }
        if let Some(duration) = value_as_number(options.get("duration")) {
            metadata.insert("duration".to_string(), Value::from(duration));
        }
        if let Some(generate_audio) = options.get("generateAudio").and_then(Value::as_bool) {
            metadata.insert("generateAudio".to_string(), Value::Bool(generate_audio));
        }
    }
    if let Some(duration) = first_number(payload, &["duration"]) {
        metadata.insert("duration".to_string(), Value::from(duration));
    }
    if let Some(mode) = first_string(payload, &["generationMode"]) {
        metadata.insert("generationMode".to_string(), Value::String(mode));
    } else if parse_json_object(payload.get("firstLastFrame")).is_some() {
        metadata.insert(
            "generationMode".to_string(),
            Value::String("firstlastframe".to_string()),
        );
    } else {
        metadata.insert(
            "generationMode".to_string(),
            Value::String("normal".to_string()),
        );
    }

    let mut info = TaskBillingInfo::billable(
        task_type.to_string(),
        BillingApiType::Video,
        model,
        quantity,
        UsageUnit::Video,
    );
    info.metadata = normalize_metadata(Some(metadata));
    Some(info)
}

fn default_voice_billing(task_type: &str, payload: &Value) -> TaskBillingInfo {
    let seconds = to_positive_count(first_number(payload, &["maxSeconds"]), 5.0);
    let mut info = TaskBillingInfo::billable(
        task_type.to_string(),
        BillingApiType::Voice,
        "fal::fal-ai/index-tts-2/text-to-speech".to_string(),
        seconds,
        UsageUnit::Second,
    );
    info.metadata = normalize_metadata(Some(Map::from_iter([(
        "maxSeconds".to_string(),
        Value::from(seconds),
    )])));
    info
}

fn default_voice_design_billing(task_type: &str) -> TaskBillingInfo {
    TaskBillingInfo::billable(
        task_type.to_string(),
        BillingApiType::VoiceDesign,
        "qwen::qwen".to_string(),
        1.0,
        UsageUnit::Call,
    )
}

fn default_lip_sync_billing(task_type: &str, payload: &Value) -> TaskBillingInfo {
    let model = first_string(payload, &["lipSyncModel", "model"])
        .unwrap_or_else(|| "fal::fal-ai/kling-video/lipsync/audio-to-video".to_string());
    TaskBillingInfo::billable(
        task_type.to_string(),
        BillingApiType::LipSync,
        model,
        1.0,
        UsageUnit::Call,
    )
}

fn should_use_image_default(task_type: &str) -> bool {
    matches!(
        task_type,
        "image_panel"
            | "image_character"
            | "image_location"
            | "modify_asset_image"
            | "regenerate_group"
            | "asset_hub_image"
            | "asset_hub_modify"
            | "panel_variant"
    )
}

fn should_use_text_default(task_type: &str) -> bool {
    matches!(
        task_type,
        "regenerate_storyboard_text"
            | "insert_panel"
            | "analyze_novel"
            | "story_to_script_run"
            | "script_to_storyboard_run"
            | "clips_build"
            | "screenplay_convert"
            | "voice_analyze"
            | "analyze_global"
            | "ai_modify_appearance"
            | "ai_modify_location"
            | "ai_modify_shot_prompt"
            | "analyze_shot_variants"
            | "ai_create_character"
            | "ai_create_location"
            | "reference_to_character"
            | "character_profile_confirm"
            | "character_profile_batch_confirm"
            | "episode_split_llm"
            | "asset_hub_ai_design_character"
            | "asset_hub_ai_design_location"
            | "asset_hub_ai_modify_character"
            | "asset_hub_ai_modify_location"
            | "asset_hub_reference_to_character"
    )
}

fn resolve_quote(info: &TaskBillingInfo) -> Result<Decimal, AppError> {
    if let Some(max_cost) = info.max_frozen_cost
        && max_cost.is_finite()
        && max_cost > 0.0
    {
        return decimal_from_f64(max_cost)
            .ok_or_else(|| AppError::invalid_params("invalid billing maxFrozenCost"));
    }
    quote_task_cost(info)
}

fn merge_metadata(base: Option<Value>, extras: Value) -> Option<Value> {
    let mut merged = match base {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    };
    if let Value::Object(extra_map) = extras {
        for (key, value) in extra_map {
            merged.insert(key, value);
        }
    }
    normalize_metadata(Some(merged))
}

fn infer_task_billing_info(task_type: &str, payload: &Value) -> Option<TaskBillingInfo> {
    if should_use_image_default(task_type) {
        return default_image_billing(task_type, payload);
    }

    match task_type {
        "video_panel" => default_video_billing(task_type, payload),
        "lip_sync" => Some(default_lip_sync_billing(task_type, payload)),
        "voice_line" => Some(default_voice_billing(task_type, payload)),
        "voice_design" | "asset_hub_voice_design" => Some(default_voice_design_billing(task_type)),
        _ if should_use_text_default(task_type) => default_text_billing(task_type, payload),
        _ => None,
    }
}

pub fn is_billable_task_type(task_type: &str) -> bool {
    BILLABLE_TASK_TYPES.contains(task_type.trim())
}

pub fn build_default_task_billing_info(
    task_type: &str,
    payload: &Value,
) -> Option<TaskBillingInfo> {
    let task_type = task_type.trim();
    if !is_billable_task_type(task_type) {
        return None;
    }
    infer_task_billing_info(task_type, payload)
}

pub fn parse_task_billing_info(raw: Option<Value>) -> Result<Option<TaskBillingInfo>, AppError> {
    let Some(raw) = raw else {
        return Ok(None);
    };

    serde_json::from_value::<TaskBillingInfo>(raw)
        .map(Some)
        .map_err(|error| {
            AppError::invalid_params(format!("invalid task billingInfo json: {error}"))
        })
}

pub fn serialize_task_billing_info(
    info: Option<&TaskBillingInfo>,
) -> Result<Option<Value>, AppError> {
    let Some(info) = info else {
        return Ok(None);
    };

    serde_json::to_value(info).map(Some).map_err(|error| {
        AppError::internal(format!("failed to serialize task billing info: {error}"))
    })
}

pub async fn prepare_task_billing(
    pool: &MySqlPool,
    mode: BillingMode,
    task_id: &str,
    user_id: &str,
    project_id: &str,
    task_type: &str,
    payload: &Value,
) -> Result<Option<TaskBillingInfo>, AppError> {
    if !is_billable_task_type(task_type) {
        return Ok(None);
    }

    let mut info = if let Some(raw) = payload.get("billingInfo").cloned() {
        serde_json::from_value::<TaskBillingInfo>(raw).map_err(|error| {
            AppError::invalid_params(format!("invalid billingInfo payload: {error}"))
        })?
    } else {
        infer_task_billing_info(task_type, payload).ok_or_else(|| {
            AppError::invalid_params(format!(
                "missing server-generated billingInfo for billable task type: {task_type}"
            ))
        })?
    };

    if !info.billable {
        info.status = Some(BillingStatus::Skipped);
        return Ok(Some(info));
    }

    info.source.get_or_insert_with(|| "task".to_string());
    info.task_type.get_or_insert_with(|| task_type.to_string());
    info.action.get_or_insert_with(|| task_type.to_string());
    info.pricing_version
        .get_or_insert_with(|| BUILTIN_PRICING_VERSION.to_string());
    info.billing_key.get_or_insert_with(|| task_id.to_string());
    info.mode_snapshot = Some(mode);

    let quoted_cost = match resolve_quote(&info) {
        Ok(value) => value,
        Err(error) if mode != BillingMode::Enforce => Decimal::ZERO,
        Err(error) => return Err(error),
    };
    let quoted_cost_number = decimal_to_f64(quoted_cost);
    info.max_frozen_cost = Some(quoted_cost_number);

    match mode {
        BillingMode::Off => {
            info.status = Some(BillingStatus::Skipped);
            Ok(Some(info))
        }
        BillingMode::Shadow => {
            info.status = Some(if quoted_cost > Decimal::ZERO {
                BillingStatus::Quoted
            } else {
                BillingStatus::Skipped
            });
            Ok(Some(info))
        }
        BillingMode::Enforce => {
            if quoted_cost <= Decimal::ZERO {
                return Err(AppError::invalid_params(
                    "billing quote must be positive in ENFORCE mode",
                ));
            }

            let freeze_metadata = merge_metadata(
                info.metadata.clone(),
                json!({
                    "taskId": task_id,
                    "taskType": info.task_type,
                    "projectId": project_id,
                    "action": info.action,
                    "apiType": info.api_type.map(|item| item.as_str()),
                    "model": info.model,
                    "quantity": info.quantity,
                    "unit": info.unit.map(|item| item.as_str()),
                    "billingKey": info.billing_key,
                    "pricingVersion": info.pricing_version,
                    "quotedCost": quoted_cost_number,
                }),
            );

            let freeze_id = freeze_balance(
                pool,
                user_id,
                quoted_cost_number,
                Some(FreezeBalanceOptions {
                    source: Some("task".to_string()),
                    task_id: Some(task_id.to_string()),
                    request_id: None,
                    idempotency_key: info.billing_key.clone(),
                    metadata: freeze_metadata,
                }),
            )
            .await?;

            let Some(freeze_id) = freeze_id else {
                let balance = get_balance(pool, user_id).await?;
                return Err(insufficient_balance_error(quoted_cost, balance.balance));
            };

            info.freeze_id = Some(freeze_id);
            info.status = Some(BillingStatus::Frozen);
            Ok(Some(info))
        }
    }
}

pub async fn settle_task_billing(
    pool: &MySqlPool,
    task_id: &str,
    user_id: &str,
    project_id: &str,
    episode_id: Option<&str>,
    billing_info: Option<&TaskBillingInfo>,
) -> Result<Option<TaskBillingInfo>, AppError> {
    let Some(mut info) = billing_info.cloned() else {
        return Ok(None);
    };
    if !info.billable {
        return Ok(Some(info));
    }

    let mode = info.mode_snapshot.unwrap_or(BillingMode::Off);
    let no_charge_status = if info.status == Some(BillingStatus::Skipped) {
        BillingStatus::Skipped
    } else {
        BillingStatus::Settled
    };

    let quoted_cost = match resolve_quote(&info) {
        Ok(value) => value,
        Err(error) if mode == BillingMode::Shadow => Decimal::ZERO,
        Err(error) => return Err(error),
    };
    let quoted_cost_number = decimal_to_f64(quoted_cost);

    match mode {
        BillingMode::Off => {
            info.status = Some(no_charge_status);
            info.charged_cost = Some(0.0);
            Ok(Some(info))
        }
        BillingMode::Shadow => {
            if quoted_cost > Decimal::ZERO {
                let action = info.action.clone().unwrap_or_else(|| {
                    info.task_type.clone().unwrap_or_else(|| "task".to_string())
                });
                let api_type = info
                    .api_type
                    .ok_or_else(|| AppError::invalid_params("billing info apiType is required"))?;
                let model = info
                    .model
                    .clone()
                    .ok_or_else(|| AppError::invalid_params("billing info model is required"))?;
                let quantity = info.quantity.unwrap_or(0.0).max(0.0);
                let unit = info
                    .unit
                    .ok_or_else(|| AppError::invalid_params("billing info unit is required"))?;

                let metadata = merge_metadata(
                    info.metadata.clone(),
                    json!({
                        "mode": "SHADOW",
                        "taskId": task_id,
                        "quotedCost": quoted_cost_number,
                    }),
                );

                record_shadow_usage(
                    pool,
                    user_id,
                    &ConfirmChargeInput {
                        project_id: project_id.to_string(),
                        action,
                        api_type,
                        model,
                        quantity,
                        unit,
                        metadata,
                        episode_id: episode_id.map(|value| value.to_string()),
                        task_type: info.task_type.clone(),
                        charged_amount: Some(quoted_cost_number),
                    },
                )
                .await?;
            }

            info.status = Some(no_charge_status);
            info.charged_cost = Some(0.0);
            Ok(Some(info))
        }
        BillingMode::Enforce => {
            let freeze_id = info
                .freeze_id
                .clone()
                .ok_or_else(|| AppError::invalid_params("missing freezeId for enforce billing"))?;
            if quoted_cost <= Decimal::ZERO {
                return Err(AppError::invalid_params(
                    "charged amount must be positive for enforce billing",
                ));
            }

            let action = info
                .action
                .clone()
                .unwrap_or_else(|| info.task_type.clone().unwrap_or_else(|| "task".to_string()));
            let api_type = info
                .api_type
                .ok_or_else(|| AppError::invalid_params("billing info apiType is required"))?;
            let model = info
                .model
                .clone()
                .ok_or_else(|| AppError::invalid_params("billing info model is required"))?;
            let quantity = info.quantity.unwrap_or(0.0).max(0.0);
            let unit = info
                .unit
                .ok_or_else(|| AppError::invalid_params("billing info unit is required"))?;
            let metadata = merge_metadata(
                info.metadata.clone(),
                json!({
                    "mode": "ENFORCE",
                    "taskId": task_id,
                    "quotedCost": quoted_cost_number,
                    "pricingVersion": info.pricing_version,
                    "billingKey": info.billing_key,
                }),
            );

            super::ledger::confirm_charge_with_record(
                pool,
                &freeze_id,
                &ConfirmChargeInput {
                    project_id: project_id.to_string(),
                    action,
                    api_type,
                    model,
                    quantity,
                    unit,
                    metadata,
                    episode_id: episode_id.map(|value| value.to_string()),
                    task_type: info.task_type.clone(),
                    charged_amount: Some(quoted_cost_number),
                },
            )
            .await?;

            info.status = Some(BillingStatus::Settled);
            info.charged_cost = Some(quoted_cost_number);
            Ok(Some(info))
        }
    }
}

pub async fn rollback_task_billing(
    pool: &MySqlPool,
    billing_info: Option<&TaskBillingInfo>,
) -> Result<Option<TaskBillingInfo>, AppError> {
    let Some(mut info) = billing_info.cloned() else {
        return Ok(None);
    };
    if !info.billable {
        return Ok(Some(info));
    }

    if info.mode_snapshot != Some(BillingMode::Enforce) {
        return Ok(Some(info));
    }

    let Some(freeze_id) = info.freeze_id.clone() else {
        return Ok(Some(info));
    };

    rollback_freeze(pool, &freeze_id).await?;
    info.status = Some(BillingStatus::RolledBack);
    Ok(Some(info))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn billable_task_types_match_expected_keys() {
        assert!(is_billable_task_type("image_panel"));
        assert!(is_billable_task_type("voice_design"));
        assert!(!is_billable_task_type("unknown_task"));
    }

    #[test]
    fn default_image_billing_extracts_model_and_resolution() {
        let payload = json!({
            "imageModel": "fal::banana-2",
            "candidateCount": 3,
            "generationOptions": {
                "resolution": "2K"
            }
        });

        let info = build_default_task_billing_info("image_panel", &payload)
            .expect("image billing info should be generated");

        assert_eq!(info.model.as_deref(), Some("fal::banana-2"));
        assert_eq!(info.quantity, Some(3.0));
        assert_eq!(info.api_type, Some(BillingApiType::Image));
    }
}
