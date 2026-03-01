use std::collections::HashMap;

use crate::{
    api_config::{UnifiedModelType, parse_model_key_strict},
    errors::AppError,
};

use super::{
    catalog::{find_builtin_capabilities, find_builtin_capability_catalog_entry},
    types::{CapabilitySelections, CapabilityValue, ModelCapabilities},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilitySelectionValidationCode {
    CapabilitySelectionInvalid,
    CapabilityModelUnsupported,
    CapabilityFieldInvalid,
    CapabilityValueNotAllowed,
    CapabilityRequired,
}

impl CapabilitySelectionValidationCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CapabilitySelectionInvalid => "CAPABILITY_SELECTION_INVALID",
            Self::CapabilityModelUnsupported => "CAPABILITY_MODEL_UNSUPPORTED",
            Self::CapabilityFieldInvalid => "CAPABILITY_FIELD_INVALID",
            Self::CapabilityValueNotAllowed => "CAPABILITY_VALUE_NOT_ALLOWED",
            Self::CapabilityRequired => "CAPABILITY_REQUIRED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilitySelectionValidationIssue {
    pub code: CapabilitySelectionValidationCode,
    pub field: String,
    pub message: String,
    pub allowed_values: Option<Vec<CapabilityValue>>,
}

#[derive(Debug, Clone)]
pub struct CapabilityModelContext {
    pub model_type: UnifiedModelType,
    pub capabilities: Option<ModelCapabilities>,
}

#[derive(Debug, Clone)]
pub struct ResolveGenerationOptionsInput {
    pub model_type: UnifiedModelType,
    pub model_key: String,
    pub capabilities: Option<ModelCapabilities>,
    pub capability_defaults: Option<CapabilitySelections>,
    pub capability_overrides: Option<CapabilitySelections>,
    pub runtime_selections: Option<HashMap<String, CapabilityValue>>,
    pub require_all_fields: bool,
}

fn read_selection_for_model(
    selections: Option<&CapabilitySelections>,
    model_key: &str,
) -> Option<HashMap<String, CapabilityValue>> {
    let selected = selections.and_then(|records| records.get(model_key))?;
    let mut output = HashMap::new();

    for (field, value) in selected {
        if field == "aspectRatio" {
            continue;
        }
        output.insert(field.clone(), value.clone());
    }

    Some(output)
}

fn merge_selection_records(
    records: &[Option<HashMap<String, CapabilityValue>>],
) -> HashMap<String, CapabilityValue> {
    let mut merged = HashMap::new();
    for record in records.iter().flatten() {
        for (field, value) in record {
            merged.insert(field.clone(), value.clone());
        }
    }
    merged
}

pub fn get_capability_option_fields(
    model_type: UnifiedModelType,
    capabilities: Option<&ModelCapabilities>,
) -> HashMap<String, Vec<CapabilityValue>> {
    let mut fields = HashMap::new();
    let Some(capabilities) = capabilities else {
        return fields;
    };

    match model_type {
        UnifiedModelType::Llm => {
            if let Some(namespace) = capabilities.llm.as_ref()
                && let Some(options) = namespace.reasoning_effort_options.as_ref()
            {
                fields.insert(
                    "reasoningEffort".to_string(),
                    options
                        .iter()
                        .map(|item| CapabilityValue::String(item.clone()))
                        .collect(),
                );
            }
        }
        UnifiedModelType::Image => {
            if let Some(namespace) = capabilities.image.as_ref()
                && let Some(options) = namespace.resolution_options.as_ref()
            {
                fields.insert(
                    "resolution".to_string(),
                    options
                        .iter()
                        .map(|item| CapabilityValue::String(item.clone()))
                        .collect(),
                );
            }
        }
        UnifiedModelType::Video => {
            if let Some(namespace) = capabilities.video.as_ref() {
                if let Some(options) = namespace.generation_mode_options.as_ref() {
                    fields.insert(
                        "generationMode".to_string(),
                        options
                            .iter()
                            .map(|item| CapabilityValue::String(item.clone()))
                            .collect(),
                    );
                }
                if let Some(options) = namespace.generate_audio_options.as_ref() {
                    fields.insert(
                        "generateAudio".to_string(),
                        options
                            .iter()
                            .map(|item| CapabilityValue::Bool(*item))
                            .collect(),
                    );
                }
                if let Some(options) = namespace.duration_options.as_ref() {
                    fields.insert(
                        "duration".to_string(),
                        options
                            .iter()
                            .map(|item| CapabilityValue::Number(*item))
                            .collect(),
                    );
                }
                if let Some(options) = namespace.fps_options.as_ref() {
                    fields.insert(
                        "fps".to_string(),
                        options
                            .iter()
                            .map(|item| CapabilityValue::Number(*item))
                            .collect(),
                    );
                }
                if let Some(options) = namespace.resolution_options.as_ref() {
                    fields.insert(
                        "resolution".to_string(),
                        options
                            .iter()
                            .map(|item| CapabilityValue::String(item.clone()))
                            .collect(),
                    );
                }
            }
        }
        UnifiedModelType::Audio => {
            if let Some(namespace) = capabilities.audio.as_ref() {
                if let Some(options) = namespace.voice_options.as_ref() {
                    fields.insert(
                        "voice".to_string(),
                        options
                            .iter()
                            .map(|item| CapabilityValue::String(item.clone()))
                            .collect(),
                    );
                }
                if let Some(options) = namespace.rate_options.as_ref() {
                    fields.insert(
                        "rate".to_string(),
                        options
                            .iter()
                            .map(|item| CapabilityValue::String(item.clone()))
                            .collect(),
                    );
                }
            }
        }
        UnifiedModelType::Lipsync => {
            if let Some(namespace) = capabilities.lipsync.as_ref()
                && let Some(options) = namespace.mode_options.as_ref()
            {
                fields.insert(
                    "mode".to_string(),
                    options
                        .iter()
                        .map(|item| CapabilityValue::String(item.clone()))
                        .collect(),
                );
            }
        }
    }

    fields
}

pub fn has_capability_options(
    model_type: UnifiedModelType,
    capabilities: Option<&ModelCapabilities>,
) -> bool {
    !get_capability_option_fields(model_type, capabilities).is_empty()
}

pub fn validate_capability_selection_for_model(
    model_key: &str,
    model_type: UnifiedModelType,
    capabilities: Option<&ModelCapabilities>,
    selection: Option<&HashMap<String, CapabilityValue>>,
    require_all_fields: bool,
) -> Vec<CapabilitySelectionValidationIssue> {
    let option_fields = get_capability_option_fields(model_type, capabilities);
    let mut issues = Vec::new();

    let selection = selection.cloned().unwrap_or_default();
    if option_fields.is_empty() {
        if !selection.is_empty() {
            issues.push(CapabilitySelectionValidationIssue {
                code: CapabilitySelectionValidationCode::CapabilityFieldInvalid,
                field: format!("capabilities.{model_key}"),
                message: "model has no configurable capability options".to_string(),
                allowed_values: None,
            });
        }
        return issues;
    }

    for (field, value) in &selection {
        let Some(allowed_values) = option_fields.get(field) else {
            issues.push(CapabilitySelectionValidationIssue {
                code: CapabilitySelectionValidationCode::CapabilityFieldInvalid,
                field: format!("capabilities.{model_key}.{field}"),
                message: format!("field {field} is not supported by model {model_key}"),
                allowed_values: None,
            });
            continue;
        };

        if !allowed_values.contains(value) {
            issues.push(CapabilitySelectionValidationIssue {
                code: CapabilitySelectionValidationCode::CapabilityValueNotAllowed,
                field: format!("capabilities.{model_key}.{field}"),
                message: format!("value {value} is not allowed"),
                allowed_values: Some(allowed_values.clone()),
            });
        }
    }

    if require_all_fields {
        for (field, allowed_values) in option_fields {
            if !selection.contains_key(&field) {
                issues.push(CapabilitySelectionValidationIssue {
                    code: CapabilitySelectionValidationCode::CapabilityRequired,
                    field: format!("capabilities.{model_key}.{field}"),
                    message: format!("field {field} is required for model {model_key}"),
                    allowed_values: Some(allowed_values),
                });
            }
        }
    }

    issues
}

pub fn validate_capability_selections_payload(
    selections: Option<&CapabilitySelections>,
    resolve_model_context: impl Fn(&str) -> Option<CapabilityModelContext>,
) -> Vec<CapabilitySelectionValidationIssue> {
    let Some(selections) = selections else {
        return Vec::new();
    };

    let mut issues = Vec::new();
    for (model_key, raw_selection) in selections {
        if parse_model_key_strict(model_key).is_none() {
            issues.push(CapabilitySelectionValidationIssue {
                code: CapabilitySelectionValidationCode::CapabilitySelectionInvalid,
                field: format!("capabilitySelections.{model_key}"),
                message: "model key must be provider::modelId".to_string(),
                allowed_values: None,
            });
            continue;
        }

        let Some(context) = resolve_model_context(model_key) else {
            issues.push(CapabilitySelectionValidationIssue {
                code: CapabilitySelectionValidationCode::CapabilityModelUnsupported,
                field: format!("capabilitySelections.{model_key}"),
                message: format!(
                    "model {model_key} is not supported by built-in capability catalog"
                ),
                allowed_values: None,
            });
            continue;
        };

        issues.extend(validate_capability_selection_for_model(
            model_key,
            context.model_type,
            context.capabilities.as_ref(),
            Some(raw_selection),
            false,
        ));
    }

    issues
}

pub fn resolve_generation_options_for_model(
    input: ResolveGenerationOptionsInput,
) -> (
    HashMap<String, CapabilityValue>,
    Vec<CapabilitySelectionValidationIssue>,
) {
    let default_selection =
        read_selection_for_model(input.capability_defaults.as_ref(), &input.model_key);
    let override_selection =
        read_selection_for_model(input.capability_overrides.as_ref(), &input.model_key);

    let merged = merge_selection_records(&[
        default_selection,
        override_selection,
        input.runtime_selections,
    ]);

    let issues = validate_capability_selection_for_model(
        &input.model_key,
        input.model_type,
        input.capabilities.as_ref(),
        Some(&merged),
        input.require_all_fields,
    );
    if !issues.is_empty() {
        return (HashMap::new(), issues);
    }

    let option_fields = get_capability_option_fields(input.model_type, input.capabilities.as_ref());
    let mut options = HashMap::new();
    for field in option_fields.keys() {
        if let Some(value) = merged.get(field) {
            options.insert(field.clone(), value.clone());
        }
    }

    (options, Vec::new())
}

pub fn resolve_builtin_model_context(
    model_type: UnifiedModelType,
    model_key: &str,
) -> Result<Option<CapabilityModelContext>, AppError> {
    let Some(parsed) = parse_model_key_strict(model_key) else {
        return Ok(None);
    };
    let entry =
        find_builtin_capability_catalog_entry(model_type, &parsed.provider, &parsed.model_id)?;
    Ok(entry.map(|item| CapabilityModelContext {
        model_type: item.model_type,
        capabilities: item.capabilities,
    }))
}

pub fn resolve_builtin_capabilities_by_model_key(
    model_type: UnifiedModelType,
    model_key: &str,
) -> Result<Option<ModelCapabilities>, AppError> {
    let Some(parsed) = parse_model_key_strict(model_key) else {
        return Ok(None);
    };
    find_builtin_capabilities(model_type, &parsed.provider, &parsed.model_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::types::{ModelCapabilities, VideoCapabilities};

    #[test]
    fn validate_selection_detects_invalid_values() {
        let capabilities = ModelCapabilities {
            video: Some(VideoCapabilities {
                duration_options: Some(vec![5, 10]),
                ..VideoCapabilities::default()
            }),
            ..ModelCapabilities::default()
        };

        let mut selection = HashMap::new();
        selection.insert("duration".to_string(), CapabilityValue::Number(99));

        let issues = validate_capability_selection_for_model(
            "fal::video",
            UnifiedModelType::Video,
            Some(&capabilities),
            Some(&selection),
            false,
        );

        assert!(
            issues.iter().any(|issue| {
                issue.code == CapabilitySelectionValidationCode::CapabilityValueNotAllowed
            }),
            "invalid value should be reported"
        );
    }
}
