use std::collections::HashMap;

use once_cell::sync::Lazy;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use super::{
    money::decimal_from_f64,
    types::{BillingApiType, DeductRequest},
};

#[derive(Debug, Clone, Copy)]
struct TaskBillingDef {
    api_type: &'static str,
    model_keys: &'static [&'static str],
    quantity_keys: &'static [&'static str],
    unit: &'static str,
    default_quantity: f64,
    default_model: Option<&'static str>,
}

const IMAGE_DEF: TaskBillingDef = TaskBillingDef {
    api_type: "image",
    model_keys: &["imageModel", "modelId", "model"],
    quantity_keys: &["candidateCount", "count"],
    unit: "image",
    default_quantity: 1.0,
    default_model: None,
};

const VIDEO_DEF: TaskBillingDef = TaskBillingDef {
    api_type: "video",
    model_keys: &["videoModel", "modelId", "model", "flModel"],
    quantity_keys: &["duration"],
    unit: "second",
    default_quantity: 5.0,
    default_model: None,
};

const VOICE_LINE_DEF: TaskBillingDef = TaskBillingDef {
    api_type: "voice",
    model_keys: &[],
    quantity_keys: &["maxSeconds"],
    unit: "second",
    default_quantity: 5.0,
    default_model: Some("fal::fal-ai/index-tts-2/text-to-speech"),
};

const VOICE_DESIGN_DEF: TaskBillingDef = TaskBillingDef {
    api_type: "voice-design",
    model_keys: &[],
    quantity_keys: &[],
    unit: "call",
    default_quantity: 1.0,
    default_model: Some("qwen::qwen"),
};

const LIP_SYNC_DEF: TaskBillingDef = TaskBillingDef {
    api_type: "lip-sync",
    model_keys: &["lipSyncModel", "model"],
    quantity_keys: &[],
    unit: "call",
    default_quantity: 1.0,
    default_model: Some("fal::fal-ai/kling-video/lipsync/audio-to-video"),
};

const TEXT_DEF: TaskBillingDef = TaskBillingDef {
    api_type: "text",
    model_keys: &["analysisModel", "model"],
    quantity_keys: &["maxInputTokens"],
    unit: "token",
    default_quantity: 4200.0,
    default_model: None,
};

static BILLING_DEFS: Lazy<HashMap<&'static str, TaskBillingDef>> = Lazy::new(|| {
    let mut defs = HashMap::new();

    for task_type in [
        "image_panel",
        "image_character",
        "image_location",
        "modify_asset_image",
        "regenerate_group",
        "asset_hub_image",
        "asset_hub_modify",
        "panel_variant",
    ] {
        defs.insert(task_type, IMAGE_DEF);
    }

    defs.insert("video_panel", VIDEO_DEF);
    defs.insert("voice_line", VOICE_LINE_DEF);
    defs.insert("voice_design", VOICE_DESIGN_DEF);
    defs.insert("asset_hub_voice_design", VOICE_DESIGN_DEF);
    defs.insert("lip_sync", LIP_SYNC_DEF);

    for task_type in [
        "regenerate_storyboard_text",
        "insert_panel",
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
    ] {
        defs.insert(task_type, TEXT_DEF);
    }

    defs
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

fn to_non_negative(value: Option<f64>, fallback: f64) -> f64 {
    let resolved = value.unwrap_or(fallback);
    if !resolved.is_finite() {
        return fallback;
    }
    resolved.max(0.0)
}

fn to_positive_count(value: Option<f64>, fallback: f64) -> f64 {
    let resolved = value.unwrap_or(fallback);
    if !resolved.is_finite() {
        return fallback;
    }
    resolved.max(1.0).floor()
}

fn to_decimal(value: f64) -> Option<Decimal> {
    decimal_from_f64(value).map(|item| item.round_dp(6))
}

fn image_resolution(payload: &Value) -> Option<String> {
    if let Some(options) = read_generation_options(payload)
        && let Some(resolution) = value_as_string(options.get("resolution"))
    {
        return Some(resolution);
    }

    first_string(payload, &["resolution"])
}

fn canonical_api_type(raw: &str) -> String {
    BillingApiType::parse(raw)
        .map(BillingApiType::as_str)
        .unwrap_or(raw)
        .to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BillingParams {
    pub api_type: String,
    pub model: String,
    pub quantity: Decimal,
    pub unit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

pub fn is_billable_task_type(task_type: &str) -> bool {
    BILLING_DEFS.contains_key(task_type.trim())
}

pub fn extract_billing_params(task_type: &str, payload: &Value) -> Option<BillingParams> {
    let def = BILLING_DEFS.get(task_type.trim())?;

    let model =
        first_string(payload, def.model_keys).or_else(|| def.default_model.map(str::to_string))?;
    let model = model.trim().to_string();
    if model.is_empty() {
        return None;
    }

    let mut unit = def.unit.to_string();
    let mut metadata: Option<Value> = None;

    let quantity = if def.api_type == "text" {
        let input_tokens =
            to_non_negative(first_number(payload, &["maxInputTokens"]), 3_000.0).floor();
        let output_tokens =
            to_non_negative(first_number(payload, &["maxOutputTokens"]), 1_200.0).floor();
        let total = input_tokens + output_tokens;

        metadata = Some(json!({
            "inputTokens": input_tokens,
            "outputTokens": output_tokens,
        }));
        total
    } else {
        let raw = if def.quantity_keys.is_empty() {
            None
        } else {
            first_number(payload, def.quantity_keys)
        };

        if def.unit == "call" {
            to_positive_count(raw, def.default_quantity)
        } else if def.unit == "second" {
            to_positive_count(raw, def.default_quantity)
        } else {
            to_positive_count(raw, def.default_quantity)
        }
    };

    if def.api_type == "image"
        && let Some(resolution) = image_resolution(payload)
    {
        unit = format!("image:{resolution}");

        let mut metadata_map = metadata
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        metadata_map.insert("resolution".to_string(), Value::String(resolution));
        metadata = Some(Value::Object(metadata_map));
    }

    let quantity = to_decimal(quantity)?;
    if quantity <= Decimal::ZERO {
        return None;
    }

    Some(BillingParams {
        api_type: canonical_api_type(def.api_type),
        model,
        quantity,
        unit,
        metadata,
    })
}

pub fn build_deduct_request(
    task_id: &str,
    user_id: &str,
    project_id: &str,
    episode_id: Option<&str>,
    task_type: &str,
    payload: &Value,
) -> Option<DeductRequest> {
    let params = extract_billing_params(task_type, payload)?;

    Some(DeductRequest {
        task_id: task_id.trim().to_string(),
        user_id: user_id.trim().to_string(),
        project_id: project_id.trim().to_string(),
        episode_id: episode_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        api_type: params.api_type,
        model: params.model,
        action: task_type.trim().to_string(),
        quantity: params.quantity,
        unit: params.unit,
        metadata: params.metadata,
    })
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
    fn extract_text_billing_with_default_token_breakdown() {
        let payload = json!({
            "analysisModel": "openai-compatible::claude-sonnet-4-6"
        });

        let params = extract_billing_params("analyze_novel", &payload)
            .expect("text billing params should exist");

        assert_eq!(params.api_type, "text");
        assert_eq!(params.unit, "token");
        assert_eq!(params.quantity, Decimal::new(4200, 0));
        assert_eq!(
            params.metadata,
            Some(json!({
                "inputTokens": 3000.0,
                "outputTokens": 1200.0,
            }))
        );
    }

    #[test]
    fn extract_image_billing_includes_resolution_unit() {
        let payload = json!({
            "imageModel": "fal::banana-2",
            "candidateCount": 2,
            "generationOptions": {
                "resolution": "2K"
            }
        });

        let params = extract_billing_params("image_panel", &payload)
            .expect("image billing params should exist");

        assert_eq!(params.api_type, "image");
        assert_eq!(params.unit, "image:2K");
        assert_eq!(params.quantity, Decimal::new(2, 0));
    }
}
