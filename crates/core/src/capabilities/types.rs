use std::{collections::HashMap, fmt};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api_config::UnifiedModelType;

pub fn provider_key(provider_id: &str) -> String {
    provider_id
        .split(':')
        .next()
        .unwrap_or(provider_id)
        .trim()
        .to_ascii_lowercase()
}

pub fn compose_model_key(provider: &str, model_id: &str) -> Option<String> {
    let provider = provider.trim();
    let model_id = model_id.trim();
    if provider.is_empty() || model_id.is_empty() {
        return None;
    }
    Some(format!("{provider}::{model_id}"))
}

pub const fn model_type_key(model_type: UnifiedModelType) -> &'static str {
    match model_type {
        UnifiedModelType::Llm => "llm",
        UnifiedModelType::Image => "image",
        UnifiedModelType::Video => "video",
        UnifiedModelType::Audio => "audio",
        UnifiedModelType::Lipsync => "lipsync",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CapabilityValue {
    String(String),
    Number(i64),
    Bool(bool),
}

impl fmt::Display for CapabilityValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(value) => write!(f, "{value}"),
            Self::Number(value) => write!(f, "{value}"),
            Self::Bool(value) => write!(f, "{value}"),
        }
    }
}

pub type CapabilitySelections = HashMap<String, HashMap<String, CapabilityValue>>;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilityFieldI18n {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option_label_keys: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LlmCapabilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort_options: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_i18n: Option<HashMap<String, CapabilityFieldI18n>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageCapabilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution_options: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_i18n: Option<HashMap<String, CapabilityFieldI18n>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VideoCapabilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_mode_options: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generate_audio_options: Option<Vec<bool>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_options: Option<Vec<i64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fps_options: Option<Vec<i64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution_options: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub firstlastframe: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_generate_audio: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_i18n: Option<HashMap<String, CapabilityFieldI18n>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioCapabilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_options: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_options: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_i18n: Option<HashMap<String, CapabilityFieldI18n>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LipSyncCapabilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode_options: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_i18n: Option<HashMap<String, CapabilityFieldI18n>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelCapabilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm: Option<LlmCapabilities>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageCapabilities>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video: Option<VideoCapabilities>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<AudioCapabilities>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lipsync: Option<LipSyncCapabilities>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VideoPricingTier {
    pub when: HashMap<String, CapabilityValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityValidationCode {
    CapabilityShapeInvalid,
    CapabilityNamespaceInvalid,
    CapabilityFieldInvalid,
    CapabilityValueNotAllowed,
}

impl CapabilityValidationCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CapabilityShapeInvalid => "CAPABILITY_SHAPE_INVALID",
            Self::CapabilityNamespaceInvalid => "CAPABILITY_NAMESPACE_INVALID",
            Self::CapabilityFieldInvalid => "CAPABILITY_FIELD_INVALID",
            Self::CapabilityValueNotAllowed => "CAPABILITY_VALUE_NOT_ALLOWED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityValidationIssue {
    pub code: CapabilityValidationCode,
    pub field: String,
    pub message: String,
    pub allowed_values: Option<Vec<CapabilityValue>>,
}

fn push_issue(
    issues: &mut Vec<CapabilityValidationIssue>,
    code: CapabilityValidationCode,
    field: impl Into<String>,
    message: impl Into<String>,
    allowed_values: Option<Vec<CapabilityValue>>,
) {
    issues.push(CapabilityValidationIssue {
        code,
        field: field.into(),
        message: message.into(),
        allowed_values,
    });
}

fn is_non_empty_text(value: &str) -> bool {
    !value.trim().is_empty()
}

fn to_capability_values_from_string(options: Option<&Vec<String>>) -> Option<Vec<CapabilityValue>> {
    options.map(|items| {
        items
            .iter()
            .map(|item| CapabilityValue::String(item.clone()))
            .collect::<Vec<_>>()
    })
}

fn to_capability_values_from_number(options: Option<&Vec<i64>>) -> Option<Vec<CapabilityValue>> {
    options.map(|items| {
        items
            .iter()
            .map(|item| CapabilityValue::Number(*item))
            .collect::<Vec<_>>()
    })
}

fn to_capability_values_from_bool(options: Option<&Vec<bool>>) -> Option<Vec<CapabilityValue>> {
    options.map(|items| {
        items
            .iter()
            .map(|item| CapabilityValue::Bool(*item))
            .collect::<Vec<_>>()
    })
}

fn validate_field_i18n(
    issues: &mut Vec<CapabilityValidationIssue>,
    namespace: &str,
    field_i18n: Option<&HashMap<String, CapabilityFieldI18n>>,
    allowed_fields: &HashMap<String, Option<Vec<CapabilityValue>>>,
) {
    let Some(map) = field_i18n else {
        return;
    };

    for (field, config) in map {
        if !allowed_fields.contains_key(field) {
            push_issue(
                issues,
                CapabilityValidationCode::CapabilityFieldInvalid,
                format!("capabilities.{namespace}.fieldI18n.{field}"),
                format!("unknown i18n field: {field}"),
                None,
            );
            continue;
        }

        if let Some(label_key) = config.label_key.as_deref()
            && !is_non_empty_text(label_key)
        {
            push_issue(
                issues,
                CapabilityValidationCode::CapabilityFieldInvalid,
                format!("capabilities.{namespace}.fieldI18n.{field}.labelKey"),
                "labelKey must be a non-empty string",
                None,
            );
        }
        if let Some(unit_key) = config.unit_key.as_deref()
            && !is_non_empty_text(unit_key)
        {
            push_issue(
                issues,
                CapabilityValidationCode::CapabilityFieldInvalid,
                format!("capabilities.{namespace}.fieldI18n.{field}.unitKey"),
                "unitKey must be a non-empty string",
                None,
            );
        }

        if let Some(option_label_keys) = config.option_label_keys.as_ref() {
            let allowed_option_values = allowed_fields
                .get(field)
                .cloned()
                .unwrap_or(None)
                .unwrap_or_default();
            let allowed_option_text = allowed_option_values
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>();

            for (option_key, option_label) in option_label_keys {
                if !is_non_empty_text(option_label) {
                    push_issue(
                        issues,
                        CapabilityValidationCode::CapabilityFieldInvalid,
                        format!(
                            "capabilities.{namespace}.fieldI18n.{field}.optionLabelKeys.{option_key}"
                        ),
                        "option label must be a non-empty string",
                        None,
                    );
                }

                if !allowed_option_text.is_empty() && !allowed_option_text.contains(option_key) {
                    push_issue(
                        issues,
                        CapabilityValidationCode::CapabilityValueNotAllowed,
                        format!(
                            "capabilities.{namespace}.fieldI18n.{field}.optionLabelKeys.{option_key}"
                        ),
                        format!("option key {option_key} is not defined in {field} options"),
                        Some(allowed_option_values.clone()),
                    );
                }
            }
        }
    }
}

pub fn validate_model_capabilities(
    model_type: UnifiedModelType,
    raw: Option<&Value>,
) -> Vec<CapabilityValidationIssue> {
    const LLM_ALLOWED_FIELDS: &[&str] = &["reasoningEffortOptions", "fieldI18n"];
    const IMAGE_ALLOWED_FIELDS: &[&str] = &["resolutionOptions", "fieldI18n"];
    const VIDEO_ALLOWED_FIELDS: &[&str] = &[
        "generationModeOptions",
        "generateAudioOptions",
        "durationOptions",
        "fpsOptions",
        "resolutionOptions",
        "firstlastframe",
        "supportGenerateAudio",
        "fieldI18n",
    ];
    const AUDIO_ALLOWED_FIELDS: &[&str] = &["voiceOptions", "rateOptions", "fieldI18n"];
    const LIPSYNC_ALLOWED_FIELDS: &[&str] = &["modeOptions", "fieldI18n"];

    let mut issues = Vec::new();
    let Some(raw) = raw else {
        return issues;
    };

    let Some(raw_object) = raw.as_object() else {
        push_issue(
            &mut issues,
            CapabilityValidationCode::CapabilityShapeInvalid,
            "capabilities",
            "capabilities must be an object",
            None,
        );
        return issues;
    };

    let expected_namespace = model_type_key(model_type);
    for (namespace, value) in raw_object {
        let allowed_fields = match namespace.as_str() {
            "llm" => Some(LLM_ALLOWED_FIELDS),
            "image" => Some(IMAGE_ALLOWED_FIELDS),
            "video" => Some(VIDEO_ALLOWED_FIELDS),
            "audio" => Some(AUDIO_ALLOWED_FIELDS),
            "lipsync" => Some(LIPSYNC_ALLOWED_FIELDS),
            _ => None,
        };

        let Some(allowed_fields) = allowed_fields else {
            push_issue(
                &mut issues,
                CapabilityValidationCode::CapabilityNamespaceInvalid,
                format!("capabilities.{namespace}"),
                format!("Unknown capabilities namespace: {namespace}"),
                None,
            );
            continue;
        };

        if namespace != expected_namespace {
            push_issue(
                &mut issues,
                CapabilityValidationCode::CapabilityNamespaceInvalid,
                format!("capabilities.{namespace}"),
                format!("Namespace {namespace} is not allowed for model type {expected_namespace}"),
                Some(vec![CapabilityValue::String(
                    expected_namespace.to_string(),
                )]),
            );
        }

        let Some(namespace_object) = value.as_object() else {
            push_issue(
                &mut issues,
                CapabilityValidationCode::CapabilityShapeInvalid,
                format!("capabilities.{namespace}"),
                format!("capabilities.{namespace} must be an object"),
                None,
            );
            continue;
        };

        for field in namespace_object.keys() {
            if allowed_fields.contains(&field.as_str()) {
                continue;
            }
            let message = if field == "i18n" {
                "Use fieldI18n instead of i18n".to_string()
            } else {
                format!("Unknown capability field: {field}")
            };
            push_issue(
                &mut issues,
                CapabilityValidationCode::CapabilityFieldInvalid,
                format!("capabilities.{namespace}.{field}"),
                message,
                None,
            );
        }
    }

    let parsed = match serde_json::from_value::<ModelCapabilities>(raw.clone()) {
        Ok(value) => value,
        Err(error) => {
            if !issues.is_empty() {
                return issues;
            }
            push_issue(
                &mut issues,
                CapabilityValidationCode::CapabilityShapeInvalid,
                "capabilities",
                format!("capabilities payload is invalid: {error}"),
                None,
            );
            return issues;
        }
    };

    if let Some(llm) = parsed.llm.as_ref() {
        if matches!(llm.reasoning_effort_options.as_ref(), Some(values) if values.is_empty()) {
            push_issue(
                &mut issues,
                CapabilityValidationCode::CapabilityFieldInvalid,
                "capabilities.llm.reasoningEffortOptions",
                "reasoningEffortOptions must not be empty",
                None,
            );
        }

        let mut allowed = HashMap::new();
        allowed.insert(
            "reasoningEffort".to_string(),
            to_capability_values_from_string(llm.reasoning_effort_options.as_ref()),
        );
        validate_field_i18n(&mut issues, "llm", llm.field_i18n.as_ref(), &allowed);
    }

    if let Some(image) = parsed.image.as_ref() {
        if matches!(image.resolution_options.as_ref(), Some(values) if values.is_empty()) {
            push_issue(
                &mut issues,
                CapabilityValidationCode::CapabilityFieldInvalid,
                "capabilities.image.resolutionOptions",
                "resolutionOptions must not be empty",
                None,
            );
        }

        let mut allowed = HashMap::new();
        allowed.insert(
            "resolution".to_string(),
            to_capability_values_from_string(image.resolution_options.as_ref()),
        );
        validate_field_i18n(&mut issues, "image", image.field_i18n.as_ref(), &allowed);
    }

    if let Some(video) = parsed.video.as_ref() {
        if matches!(video.generation_mode_options.as_ref(), Some(values) if values.is_empty()) {
            push_issue(
                &mut issues,
                CapabilityValidationCode::CapabilityFieldInvalid,
                "capabilities.video.generationModeOptions",
                "generationModeOptions must not be empty",
                None,
            );
        }
        if matches!(video.generate_audio_options.as_ref(), Some(values) if values.is_empty()) {
            push_issue(
                &mut issues,
                CapabilityValidationCode::CapabilityFieldInvalid,
                "capabilities.video.generateAudioOptions",
                "generateAudioOptions must not be empty",
                None,
            );
        }
        if matches!(video.duration_options.as_ref(), Some(values) if values.is_empty()) {
            push_issue(
                &mut issues,
                CapabilityValidationCode::CapabilityFieldInvalid,
                "capabilities.video.durationOptions",
                "durationOptions must not be empty",
                None,
            );
        }
        if matches!(video.fps_options.as_ref(), Some(values) if values.is_empty()) {
            push_issue(
                &mut issues,
                CapabilityValidationCode::CapabilityFieldInvalid,
                "capabilities.video.fpsOptions",
                "fpsOptions must not be empty",
                None,
            );
        }
        if matches!(video.resolution_options.as_ref(), Some(values) if values.is_empty()) {
            push_issue(
                &mut issues,
                CapabilityValidationCode::CapabilityFieldInvalid,
                "capabilities.video.resolutionOptions",
                "resolutionOptions must not be empty",
                None,
            );
        }

        let mut allowed = HashMap::new();
        allowed.insert(
            "generationMode".to_string(),
            to_capability_values_from_string(video.generation_mode_options.as_ref()),
        );
        allowed.insert(
            "generateAudio".to_string(),
            to_capability_values_from_bool(video.generate_audio_options.as_ref()),
        );
        allowed.insert(
            "duration".to_string(),
            to_capability_values_from_number(video.duration_options.as_ref()),
        );
        allowed.insert(
            "fps".to_string(),
            to_capability_values_from_number(video.fps_options.as_ref()),
        );
        allowed.insert(
            "resolution".to_string(),
            to_capability_values_from_string(video.resolution_options.as_ref()),
        );
        validate_field_i18n(&mut issues, "video", video.field_i18n.as_ref(), &allowed);
    }

    if let Some(audio) = parsed.audio.as_ref() {
        if matches!(audio.voice_options.as_ref(), Some(values) if values.is_empty()) {
            push_issue(
                &mut issues,
                CapabilityValidationCode::CapabilityFieldInvalid,
                "capabilities.audio.voiceOptions",
                "voiceOptions must not be empty",
                None,
            );
        }
        if matches!(audio.rate_options.as_ref(), Some(values) if values.is_empty()) {
            push_issue(
                &mut issues,
                CapabilityValidationCode::CapabilityFieldInvalid,
                "capabilities.audio.rateOptions",
                "rateOptions must not be empty",
                None,
            );
        }

        let mut allowed = HashMap::new();
        allowed.insert(
            "voice".to_string(),
            to_capability_values_from_string(audio.voice_options.as_ref()),
        );
        allowed.insert(
            "rate".to_string(),
            to_capability_values_from_string(audio.rate_options.as_ref()),
        );
        validate_field_i18n(&mut issues, "audio", audio.field_i18n.as_ref(), &allowed);
    }

    if let Some(lipsync) = parsed.lipsync.as_ref() {
        if matches!(lipsync.mode_options.as_ref(), Some(values) if values.is_empty()) {
            push_issue(
                &mut issues,
                CapabilityValidationCode::CapabilityFieldInvalid,
                "capabilities.lipsync.modeOptions",
                "modeOptions must not be empty",
                None,
            );
        }

        let mut allowed = HashMap::new();
        allowed.insert(
            "mode".to_string(),
            to_capability_values_from_string(lipsync.mode_options.as_ref()),
        );
        validate_field_i18n(
            &mut issues,
            "lipsync",
            lipsync.field_i18n.as_ref(),
            &allowed,
        );
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validate_model_capabilities_does_not_require_expected_namespace() {
        let issues = validate_model_capabilities(UnifiedModelType::Video, Some(&json!({})));
        assert!(issues.is_empty());
    }

    #[test]
    fn validate_model_capabilities_reports_unknown_namespace() {
        let issues =
            validate_model_capabilities(UnifiedModelType::Video, Some(&json!({ "foo": {} })));
        assert!(issues.iter().any(|issue| {
            issue.code == CapabilityValidationCode::CapabilityNamespaceInvalid
                && issue.field == "capabilities.foo"
        }));
    }

    #[test]
    fn validate_model_capabilities_reports_legacy_i18n_field() {
        let issues = validate_model_capabilities(
            UnifiedModelType::Video,
            Some(&json!({ "video": { "i18n": {} } })),
        );
        assert!(issues.iter().any(|issue| {
            issue.code == CapabilityValidationCode::CapabilityFieldInvalid
                && issue.field == "capabilities.video.i18n"
                && issue.message == "Use fieldI18n instead of i18n"
        }));
    }
}
