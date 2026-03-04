use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::UNIX_EPOCH,
};

use once_cell::sync::Lazy;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::Value;

use crate::{api_config::parse_model_key_strict, errors::AppError};

use super::{
    money::{decimal_from_f64, normalize_money},
    types::{BillingApiType, TaskBillingInfo},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BuiltinPricingCatalogEntry {
    api_type: BillingApiType,
    provider: String,
    model_id: String,
    pricing: BuiltinPricingDefinition,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BuiltinPricingDefinition {
    mode: PricingMode,
    flat_amount: Option<f64>,
    tiers: Option<Vec<BuiltinPricingTier>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BuiltinPricingTier {
    when: HashMap<String, Value>,
    amount: f64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum PricingMode {
    Flat,
    Capability,
}

#[derive(Debug, Clone)]
struct PricingCatalogCache {
    signature: String,
    exact: HashMap<String, BuiltinPricingCatalogEntry>,
    by_model_id: HashMap<String, Vec<BuiltinPricingCatalogEntry>>,
}

static PRICING_CACHE: Lazy<Mutex<Option<PricingCatalogCache>>> = Lazy::new(|| Mutex::new(None));

fn push_unique_dir(dirs: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !dirs.contains(&candidate) {
        dirs.push(candidate);
    }
}

fn candidate_pricing_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(cwd) = env::current_dir() {
        push_unique_dir(&mut dirs, cwd.join("standards/pricing"));
        push_unique_dir(&mut dirs, cwd.join("../standards/pricing"));
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    push_unique_dir(&mut dirs, manifest_dir.join("../../standards/pricing"));
    push_unique_dir(&mut dirs, manifest_dir.join("../../../standards/pricing"));
    dirs
}

fn resolve_pricing_dir() -> Result<PathBuf, AppError> {
    for candidate in candidate_pricing_dirs() {
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }

    Err(AppError::internal(
        "pricing catalog directory not found under standards/pricing",
    ))
}

fn resolve_pricing_files(dir: &Path) -> Result<Vec<PathBuf>, AppError> {
    let mut files = fs::read_dir(dir)
        .map_err(|error| {
            AppError::internal(format!(
                "failed to read pricing catalog directory {}: {error}",
                dir.display()
            ))
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();

    files.sort();
    if files.is_empty() {
        return Err(AppError::internal(format!(
            "pricing catalog is empty: {}",
            dir.display()
        )));
    }

    Ok(files)
}

fn build_catalog_signature(files: &[PathBuf]) -> Result<String, AppError> {
    let mut parts = Vec::with_capacity(files.len());
    for file in files {
        let stat = fs::metadata(file).map_err(|error| {
            AppError::internal(format!(
                "failed to stat pricing catalog {}: {error}",
                file.display()
            ))
        })?;

        let modified = stat
            .modified()
            .ok()
            .and_then(|stamp| stamp.duration_since(UNIX_EPOCH).ok())
            .map(|elapsed| elapsed.as_millis())
            .unwrap_or(0);

        parts.push(format!("{}:{}:{}", file.display(), stat.len(), modified));
    }

    Ok(parts.join("|"))
}

fn build_cache(entries: Vec<BuiltinPricingCatalogEntry>, signature: String) -> PricingCatalogCache {
    let mut exact = HashMap::new();
    let mut by_model_id = HashMap::new();

    for entry in entries {
        let provider = entry.provider.trim().to_string();
        let model_id = entry.model_id.trim().to_string();
        let exact_key = format!("{}::{provider}::{model_id}", entry.api_type.as_str());
        exact.insert(exact_key, entry.clone());

        let model_key = format!("{}::{model_id}", entry.api_type.as_str());
        by_model_id
            .entry(model_key)
            .or_insert_with(Vec::new)
            .push(entry);
    }

    PricingCatalogCache {
        signature,
        exact,
        by_model_id,
    }
}

fn load_catalog() -> Result<PricingCatalogCache, AppError> {
    let dir = resolve_pricing_dir()?;
    let files = resolve_pricing_files(&dir)?;
    let signature = build_catalog_signature(&files)?;

    {
        let guard = PRICING_CACHE
            .lock()
            .map_err(|_| AppError::internal("pricing catalog cache poisoned"))?;
        if let Some(cache) = guard.as_ref()
            && cache.signature == signature
        {
            return Ok(cache.clone());
        }
    }

    let mut entries = Vec::new();
    for file in files {
        let content = fs::read_to_string(&file).map_err(|error| {
            AppError::internal(format!(
                "failed to read pricing catalog file {}: {error}",
                file.display()
            ))
        })?;

        let parsed =
            serde_json::from_str::<Vec<BuiltinPricingCatalogEntry>>(&content).map_err(|error| {
                AppError::invalid_params(format!(
                    "pricing catalog file must be json array: {} ({error})",
                    file.display()
                ))
            })?;

        entries.extend(parsed);
    }

    let cache = build_cache(entries, signature);
    PRICING_CACHE
        .lock()
        .map_err(|_| AppError::internal("pricing catalog cache poisoned"))?
        .replace(cache.clone());

    Ok(cache)
}

fn provider_alias(provider_key: &str) -> Option<&'static str> {
    match provider_key {
        "gemini-compatible" => Some("google"),
        _ => None,
    }
}

fn provider_lookup_keys(provider: &str) -> Vec<String> {
    let mut keys = Vec::new();

    let normalized = provider.trim();
    if !normalized.is_empty() {
        keys.push(normalized.to_string());
    }

    let provider_key = normalized.split(':').next().unwrap_or(normalized).trim();
    if !provider_key.is_empty() {
        keys.push(provider_key.to_string());

        if let Some(alias) = provider_alias(provider_key) {
            keys.push(alias.to_string());
        }
    }

    keys.sort();
    keys.dedup();
    keys
}

enum EntryResolution {
    Resolved(BuiltinPricingCatalogEntry),
    Missing,
    Ambiguous(Vec<BuiltinPricingCatalogEntry>),
}

fn resolve_entry(
    cache: &PricingCatalogCache,
    api_type: BillingApiType,
    model: &str,
) -> EntryResolution {
    let model = model.trim();
    if model.is_empty() {
        return EntryResolution::Missing;
    }

    if let Some(parsed) = parse_model_key_strict(model) {
        for provider in provider_lookup_keys(&parsed.provider) {
            let key = format!("{}::{provider}::{}", api_type.as_str(), parsed.model_id);
            if let Some(entry) = cache.exact.get(&key) {
                return EntryResolution::Resolved(entry.clone());
            }
        }
        return EntryResolution::Missing;
    }

    let model_key = format!("{}::{model}", api_type.as_str());
    let Some(entries) = cache.by_model_id.get(&model_key) else {
        return EntryResolution::Missing;
    };

    if entries.len() == 1 {
        return EntryResolution::Resolved(entries[0].clone());
    }

    EntryResolution::Ambiguous(entries.clone())
}

fn read_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(raw) => raw.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn value_matches(expected: &Value, actual: &Value) -> bool {
    match (expected, actual) {
        (Value::Number(_), Value::Number(_)) => {
            let left = read_number(expected);
            let right = read_number(actual);
            match (left, right) {
                (Some(a), Some(b)) => (a - b).abs() < 1e-9,
                _ => expected == actual,
            }
        }
        _ => expected == actual,
    }
}

fn resolve_pricing_amount(
    api_type: BillingApiType,
    model: &str,
    selections: &HashMap<String, Value>,
) -> Result<Decimal, AppError> {
    let cache = load_catalog()?;
    let entry = match resolve_entry(&cache, api_type, model) {
        EntryResolution::Resolved(entry) => entry,
        EntryResolution::Missing => {
            return Err(AppError::invalid_params(format!(
                "billing pricing is not configured for model {model} ({})",
                api_type.as_str()
            )));
        }
        EntryResolution::Ambiguous(entries) => {
            let candidates = entries
                .iter()
                .map(|item| format!("{}::{}", item.provider, item.model_id))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(AppError::invalid_params(format!(
                "billing pricing model id is ambiguous for {model} ({}): {candidates}",
                api_type.as_str()
            )));
        }
    };

    match entry.pricing.mode {
        PricingMode::Flat => {
            let amount = entry.pricing.flat_amount.ok_or_else(|| {
                AppError::invalid_params(format!(
                    "flat billing pricing missing amount for {}::{}",
                    entry.provider, entry.model_id
                ))
            })?;
            decimal_from_f64(amount)
                .ok_or_else(|| AppError::invalid_params("invalid billing flat pricing amount"))
        }
        PricingMode::Capability => {
            let tiers = entry.pricing.tiers.unwrap_or_default();
            for tier in tiers {
                let matches_all = tier.when.iter().all(|(field, expected)| {
                    let Some(actual) = selections.get(field) else {
                        return false;
                    };
                    value_matches(expected, actual)
                });
                if matches_all {
                    return decimal_from_f64(tier.amount).ok_or_else(|| {
                        AppError::invalid_params("invalid billing capability pricing amount")
                    });
                }
            }

            Err(AppError::invalid_params(format!(
                "billing capability pricing not found for model {model} ({})",
                api_type.as_str()
            )))
        }
    }
}

fn extract_scalar_selections(metadata: Option<&Value>) -> HashMap<String, Value> {
    let mut selections = HashMap::new();

    let Some(Value::Object(map)) = metadata else {
        return selections;
    };

    for (field, value) in map {
        if value.is_string() || value.is_number() || value.is_boolean() {
            selections.insert(field.clone(), value.clone());
        }
    }

    selections
}

fn read_number_field(metadata: Option<&Value>, key: &str) -> Option<f64> {
    let Value::Object(map) = metadata? else {
        return None;
    };
    map.get(key).and_then(read_number)
}

pub fn quote_task_cost(info: &TaskBillingInfo) -> Result<Decimal, AppError> {
    if !info.billable {
        return Ok(Decimal::ZERO);
    }

    let api_type = info
        .api_type
        .ok_or_else(|| AppError::invalid_params("billing info apiType is required"))?;
    let model = info
        .model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::invalid_params("billing info model is required"))?;

    let quantity = info.quantity.unwrap_or(0.0).max(0.0);
    if quantity <= 0.0 {
        return Ok(Decimal::ZERO);
    }

    let metadata = info.metadata.as_ref();
    match api_type {
        BillingApiType::Text => {
            let input_tokens = read_number_field(metadata, "inputTokens")
                .unwrap_or(quantity)
                .max(0.0);
            let output_tokens = read_number_field(metadata, "outputTokens")
                .unwrap_or(0.0)
                .max(0.0);

            let input_price = resolve_pricing_amount(
                BillingApiType::Text,
                model,
                &HashMap::from([(
                    String::from("tokenType"),
                    Value::String("input".to_string()),
                )]),
            )?;
            let output_price = resolve_pricing_amount(
                BillingApiType::Text,
                model,
                &HashMap::from([(
                    String::from("tokenType"),
                    Value::String("output".to_string()),
                )]),
            )?;

            let input_decimal = decimal_from_f64(input_tokens)
                .ok_or_else(|| AppError::invalid_params("invalid input token count"))?;
            let output_decimal = decimal_from_f64(output_tokens)
                .ok_or_else(|| AppError::invalid_params("invalid output token count"))?;
            let per_million = Decimal::from(1_000_000_u64);

            Ok(normalize_money(
                (input_decimal * input_price / per_million)
                    + (output_decimal * output_price / per_million),
            ))
        }
        _ => {
            let selections = extract_scalar_selections(metadata);
            let unit_price = resolve_pricing_amount(api_type, model, &selections)?;
            let quantity_decimal = decimal_from_f64(quantity)
                .ok_or_else(|| AppError::invalid_params("invalid billing quantity"))?;

            Ok(normalize_money(unit_price * quantity_decimal))
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::billing::types::{BillingApiType, TaskBillingInfo, UsageUnit};

    #[test]
    fn quote_task_cost_resolves_text_model_from_catalog() {
        let info = TaskBillingInfo {
            billable: true,
            source: Some("task".to_string()),
            task_type: Some("analyze_novel".to_string()),
            api_type: Some(BillingApiType::Text),
            model: Some("openai-compatible::claude-sonnet-4-6".to_string()),
            quantity: Some(4_200.0),
            unit: Some(UsageUnit::Token),
            max_frozen_cost: None,
            pricing_version: None,
            action: Some("analyze_novel".to_string()),
            metadata: Some(json!({"inputTokens": 3000, "outputTokens": 1200})),
            billing_key: None,
            freeze_id: None,
            mode_snapshot: None,
            status: None,
            charged_cost: None,
        };

        let quoted = quote_task_cost(&info).expect("text pricing should resolve from catalog");
        assert!(quoted > Decimal::ZERO);
    }

    #[test]
    fn quote_task_cost_resolves_flat_price() {
        let info = TaskBillingInfo {
            billable: true,
            source: Some("task".to_string()),
            task_type: Some("voice_design".to_string()),
            api_type: Some(BillingApiType::VoiceDesign),
            model: Some("qwen::qwen".to_string()),
            quantity: Some(1.0),
            unit: Some(UsageUnit::Call),
            max_frozen_cost: None,
            pricing_version: None,
            action: Some("voice_design".to_string()),
            metadata: None,
            billing_key: None,
            freeze_id: None,
            mode_snapshot: None,
            status: None,
            charged_cost: None,
        };

        let quoted = quote_task_cost(&info).expect("flat pricing should resolve");
        assert!(quoted > Decimal::ZERO);
    }
}
