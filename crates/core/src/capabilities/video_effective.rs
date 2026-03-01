use std::collections::{HashMap, HashSet};

use super::types::{CapabilityFieldI18n, CapabilityValue, VideoCapabilities, VideoPricingTier};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveVideoCapabilityDefinition {
    pub field: String,
    pub options: Vec<CapabilityValue>,
    pub field_i18n: Option<CapabilityFieldI18n>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveVideoCapabilityField {
    pub field: String,
    pub options: Vec<CapabilityValue>,
    pub field_i18n: Option<CapabilityFieldI18n>,
    pub value: Option<CapabilityValue>,
}

#[derive(Debug, Clone)]
pub struct NormalizeVideoSelectionsInput {
    pub definitions: Vec<EffectiveVideoCapabilityDefinition>,
    pub pricing_tiers: Vec<VideoPricingTier>,
    pub selection: HashMap<String, CapabilityValue>,
    pub pinned_fields: Vec<String>,
}

fn push_unique_value(values: &mut Vec<CapabilityValue>, candidate: CapabilityValue) {
    if !values.contains(&candidate) {
        values.push(candidate);
    }
}

fn collect_field_i18n(
    video_capabilities: Option<&VideoCapabilities>,
) -> HashMap<String, CapabilityFieldI18n> {
    video_capabilities
        .and_then(|item| item.field_i18n.clone())
        .unwrap_or_default()
}

fn build_definitions_from_capabilities(
    video_capabilities: Option<&VideoCapabilities>,
    field_i18n: &HashMap<String, CapabilityFieldI18n>,
) -> Vec<EffectiveVideoCapabilityDefinition> {
    let mut definitions = Vec::new();
    let Some(video_capabilities) = video_capabilities else {
        return definitions;
    };

    if let Some(options) = video_capabilities.generation_mode_options.as_ref()
        && !options.is_empty()
    {
        definitions.push(EffectiveVideoCapabilityDefinition {
            field: "generationMode".to_string(),
            options: options
                .iter()
                .map(|item| CapabilityValue::String(item.clone()))
                .collect(),
            field_i18n: field_i18n.get("generationMode").cloned(),
        });
    }

    if let Some(options) = video_capabilities.generate_audio_options.as_ref()
        && !options.is_empty()
    {
        definitions.push(EffectiveVideoCapabilityDefinition {
            field: "generateAudio".to_string(),
            options: options
                .iter()
                .map(|item| CapabilityValue::Bool(*item))
                .collect(),
            field_i18n: field_i18n.get("generateAudio").cloned(),
        });
    }

    if let Some(options) = video_capabilities.duration_options.as_ref()
        && !options.is_empty()
    {
        definitions.push(EffectiveVideoCapabilityDefinition {
            field: "duration".to_string(),
            options: options
                .iter()
                .map(|item| CapabilityValue::Number(*item))
                .collect(),
            field_i18n: field_i18n.get("duration").cloned(),
        });
    }

    if let Some(options) = video_capabilities.fps_options.as_ref()
        && !options.is_empty()
    {
        definitions.push(EffectiveVideoCapabilityDefinition {
            field: "fps".to_string(),
            options: options
                .iter()
                .map(|item| CapabilityValue::Number(*item))
                .collect(),
            field_i18n: field_i18n.get("fps").cloned(),
        });
    }

    if let Some(options) = video_capabilities.resolution_options.as_ref()
        && !options.is_empty()
    {
        definitions.push(EffectiveVideoCapabilityDefinition {
            field: "resolution".to_string(),
            options: options
                .iter()
                .map(|item| CapabilityValue::String(item.clone()))
                .collect(),
            field_i18n: field_i18n.get("resolution").cloned(),
        });
    }

    definitions
}

fn build_definitions_from_pricing_tiers(
    pricing_tiers: &[VideoPricingTier],
    field_i18n: &HashMap<String, CapabilityFieldI18n>,
) -> Vec<EffectiveVideoCapabilityDefinition> {
    let mut field_order = Vec::new();
    let mut field_options: HashMap<String, Vec<CapabilityValue>> = HashMap::new();

    for tier in pricing_tiers {
        for (field, value) in &tier.when {
            if !field_options.contains_key(field) {
                field_order.push(field.clone());
                field_options.insert(field.clone(), Vec::new());
            }

            if let Some(values) = field_options.get_mut(field) {
                push_unique_value(values, value.clone());
            }
        }
    }

    let mut definitions = Vec::new();
    for field in field_order {
        let options = field_options.remove(&field).unwrap_or_default();
        if options.is_empty() {
            continue;
        }

        definitions.push(EffectiveVideoCapabilityDefinition {
            field: field.clone(),
            options,
            field_i18n: field_i18n.get(&field).cloned(),
        });
    }

    definitions
}

fn has_tier_match(
    tiers: &[VideoPricingTier],
    selection: &HashMap<String, CapabilityValue>,
) -> bool {
    if tiers.is_empty() {
        return true;
    }

    tiers.iter().any(|tier| {
        selection.iter().all(|(field, value)| {
            let Some(tier_value) = tier.when.get(field) else {
                return true;
            };
            tier_value == value
        })
    })
}

fn get_compatible_options_for_field(
    field: &str,
    options: &[CapabilityValue],
    tiers: &[VideoPricingTier],
    selection: &HashMap<String, CapabilityValue>,
) -> Vec<CapabilityValue> {
    if tiers.is_empty() {
        return options.to_vec();
    }

    options
        .iter()
        .filter_map(|candidate| {
            let mut next_selection = selection.clone();
            next_selection.insert(field.to_string(), candidate.clone());
            has_tier_match(tiers, &next_selection).then_some(candidate.clone())
        })
        .collect()
}

pub fn resolve_effective_video_capability_definitions(
    video_capabilities: Option<&VideoCapabilities>,
    pricing_tiers: Option<&[VideoPricingTier]>,
) -> Vec<EffectiveVideoCapabilityDefinition> {
    let field_i18n = collect_field_i18n(video_capabilities);
    let capability_definitions =
        build_definitions_from_capabilities(video_capabilities, &field_i18n);
    if !capability_definitions.is_empty() {
        return capability_definitions;
    }

    if let Some(pricing_tiers) = pricing_tiers
        && !pricing_tiers.is_empty()
    {
        return build_definitions_from_pricing_tiers(pricing_tiers, &field_i18n);
    }

    Vec::new()
}

pub fn normalize_video_generation_selections(
    input: NormalizeVideoSelectionsInput,
) -> HashMap<String, CapabilityValue> {
    if input.definitions.is_empty() {
        return HashMap::new();
    }

    let field_set = input
        .definitions
        .iter()
        .map(|definition| definition.field.clone())
        .collect::<HashSet<_>>();

    let mut normalized = HashMap::new();
    for (field, value) in input.selection {
        if field_set.contains(&field) {
            normalized.insert(field, value);
        }
    }

    let pinned = input.pinned_fields.into_iter().collect::<HashSet<_>>();
    let mut ordered = input.definitions;
    ordered.sort_by(|left, right| {
        let left_pinned = pinned.contains(&left.field);
        let right_pinned = pinned.contains(&right.field);
        left_pinned.cmp(&right_pinned)
    });

    let max_attempts = std::cmp::max(4, ordered.len() * 3);
    for _ in 0..max_attempts {
        let mut changed = false;

        for definition in &ordered {
            let compatible = get_compatible_options_for_field(
                &definition.field,
                &definition.options,
                &input.pricing_tiers,
                &normalized,
            );
            let current = normalized.get(&definition.field).cloned();

            if compatible.is_empty() {
                if current.is_some() {
                    normalized.remove(&definition.field);
                    changed = true;
                }
                continue;
            }

            let need_replace = current
                .as_ref()
                .map(|value| !compatible.contains(value))
                .unwrap_or(true);
            if need_replace {
                normalized.insert(definition.field.clone(), compatible[0].clone());
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }

    normalized
}

pub fn resolve_effective_video_capability_fields(
    definitions: &[EffectiveVideoCapabilityDefinition],
    pricing_tiers: Option<&[VideoPricingTier]>,
    selection: Option<&HashMap<String, CapabilityValue>>,
) -> Vec<EffectiveVideoCapabilityField> {
    let normalized = normalize_video_generation_selections(NormalizeVideoSelectionsInput {
        definitions: definitions.to_vec(),
        pricing_tiers: pricing_tiers.unwrap_or(&[]).to_vec(),
        selection: selection.cloned().unwrap_or_default(),
        pinned_fields: Vec::new(),
    });

    definitions
        .iter()
        .map(|definition| {
            let options = get_compatible_options_for_field(
                &definition.field,
                &definition.options,
                pricing_tiers.unwrap_or(&[]),
                &normalized,
            );
            let value = normalized
                .get(&definition.field)
                .cloned()
                .filter(|candidate| options.contains(candidate));

            EffectiveVideoCapabilityField {
                field: definition.field.clone(),
                options,
                field_i18n: definition.field_i18n.clone(),
                value,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_video_selection_respects_pricing_tier_compatibility() {
        let definitions = vec![
            EffectiveVideoCapabilityDefinition {
                field: "duration".to_string(),
                options: vec![CapabilityValue::Number(5), CapabilityValue::Number(10)],
                field_i18n: None,
            },
            EffectiveVideoCapabilityDefinition {
                field: "resolution".to_string(),
                options: vec![
                    CapabilityValue::String("720p".to_string()),
                    CapabilityValue::String("1080p".to_string()),
                ],
                field_i18n: None,
            },
        ];

        let pricing_tiers = vec![
            VideoPricingTier {
                when: HashMap::from([
                    ("duration".to_string(), CapabilityValue::Number(5)),
                    (
                        "resolution".to_string(),
                        CapabilityValue::String("720p".to_string()),
                    ),
                ]),
            },
            VideoPricingTier {
                when: HashMap::from([
                    ("duration".to_string(), CapabilityValue::Number(10)),
                    (
                        "resolution".to_string(),
                        CapabilityValue::String("1080p".to_string()),
                    ),
                ]),
            },
        ];

        let selection = HashMap::from([
            ("duration".to_string(), CapabilityValue::Number(5)),
            (
                "resolution".to_string(),
                CapabilityValue::String("1080p".to_string()),
            ),
        ]);

        let normalized = normalize_video_generation_selections(NormalizeVideoSelectionsInput {
            definitions,
            pricing_tiers,
            selection,
            pinned_fields: vec!["duration".to_string()],
        });

        assert_eq!(
            normalized.get("duration"),
            Some(&CapabilityValue::Number(5)),
            "pinned duration should be preserved"
        );
        assert_eq!(
            normalized.get("resolution"),
            Some(&CapabilityValue::String("720p".to_string())),
            "resolution should be adjusted to compatible value"
        );
    }
}
