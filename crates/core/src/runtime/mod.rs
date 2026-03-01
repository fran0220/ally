pub mod graph_executor;
pub mod pipeline_graph;
pub mod publisher;
pub mod quick_run_graph;
pub mod service;
pub mod task_bridge;
pub mod types;
pub mod workflow;

use sqlx::MySqlPool;

use crate::{
    api_config::{
        CustomProvider, UnifiedModelType, get_system_models_raw, get_system_providers,
        parse_model_key_strict,
    },
    errors::AppError,
};

#[derive(Debug, Clone)]
pub struct RuntimeModelConfig {
    pub model_key: String,
    pub model_id: String,
    pub provider_id: String,
    pub provider_key: String,
    pub model_type: UnifiedModelType,
}

#[derive(Debug, Clone)]
pub struct RuntimeProviderConfig {
    pub id: String,
    pub name: String,
    pub provider_key: String,
    pub api_key: String,
    pub base_url: Option<String>,
}

pub fn provider_key(provider_id: &str) -> String {
    provider_id
        .split(':')
        .next()
        .unwrap_or(provider_id)
        .trim()
        .to_lowercase()
}

fn normalize_base_url(provider_id: &str, base_url: Option<String>) -> Option<String> {
    let raw = base_url
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())?;

    if provider_key(provider_id) != "openai-compatible" {
        return Some(raw);
    }

    let trimmed = raw.trim_end_matches('/').to_string();
    if trimmed.ends_with("/v1") || trimmed.contains("/v1/") {
        return Some(trimmed);
    }

    Some(format!("{trimmed}/v1"))
}

fn normalize_provider_api_key(raw: &str) -> String {
    // TypeScript keeps provider keys encrypted in some deployments and decrypts on read.
    // Rust runtime currently accepts plaintext keys; encrypted values will fail fast at call site.
    raw.trim().to_string()
}

pub async fn resolve_model_config(
    pool: &MySqlPool,
    model_key: &str,
    expected_type: Option<UnifiedModelType>,
) -> Result<RuntimeModelConfig, AppError> {
    let parsed = parse_model_key_strict(model_key)
        .ok_or_else(|| AppError::invalid_params(format!("invalid model key: {model_key}")))?;

    let models = get_system_models_raw(pool).await?;
    let selected = models
        .into_iter()
        .find(|item| item.enabled && item.model_key == parsed.model_key)
        .ok_or_else(|| {
            AppError::invalid_params(format!(
                "model is not enabled or not found: {}",
                parsed.model_key
            ))
        })?;

    if let Some(expected) = expected_type
        && expected != selected.model_type
    {
        return Err(AppError::invalid_params(format!(
            "model type mismatch for {}",
            parsed.model_key
        )));
    }

    Ok(RuntimeModelConfig {
        model_key: parsed.model_key,
        model_id: selected.model_id,
        provider_id: selected.provider.clone(),
        provider_key: provider_key(&selected.provider),
        model_type: selected.model_type,
    })
}

fn pick_provider_by_id(
    providers: Vec<CustomProvider>,
    provider_id: &str,
) -> Option<CustomProvider> {
    providers.into_iter().find(|item| item.id == provider_id)
}

pub async fn resolve_provider_config(
    pool: &MySqlPool,
    provider_id: &str,
) -> Result<RuntimeProviderConfig, AppError> {
    let providers = get_system_providers(pool).await?;
    let provider = pick_provider_by_id(providers, provider_id).ok_or_else(|| {
        AppError::invalid_params(format!("provider is not configured: {provider_id}"))
    })?;

    let api_key = provider
        .api_key
        .as_deref()
        .map(normalize_provider_api_key)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::invalid_params(format!("provider api key missing: {provider_id}"))
        })?;

    Ok(RuntimeProviderConfig {
        id: provider.id.clone(),
        name: provider.name,
        provider_key: provider_key(&provider.id),
        api_key,
        base_url: normalize_base_url(&provider.id, provider.base_url),
    })
}

pub async fn resolve_model_with_provider(
    pool: &MySqlPool,
    model_key: &str,
    expected_type: Option<UnifiedModelType>,
) -> Result<(RuntimeModelConfig, RuntimeProviderConfig), AppError> {
    let model = resolve_model_config(pool, model_key, expected_type).await?;
    let provider = resolve_provider_config(pool, &model.provider_id).await?;
    Ok((model, provider))
}
