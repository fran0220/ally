use std::collections::{HashMap, HashSet};

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use waoowaoo_core::api_config::{
    CustomModel, CustomProvider, DefaultModelsPayload, UnifiedModelType,
    get_system_capability_defaults, get_system_default_models, get_system_models_raw,
    get_system_providers,
};

use crate::{app_state::AppState, error::AppError, extractors::auth::AdminUser};

const AI_MODELS_CONFIG_KEY: &str = "ai_models";
const AI_PROVIDERS_CONFIG_KEY: &str = "ai_providers";
const AI_DEFAULT_MODELS_CONFIG_KEY: &str = "ai_default_models";
const AI_CAPABILITY_DEFAULTS_CONFIG_KEY: &str = "ai_capability_defaults";

const DEFAULT_MODEL_FIELDS: [(&str, UnifiedModelType); 7] = [
    ("analysisModel", UnifiedModelType::Llm),
    ("characterModel", UnifiedModelType::Image),
    ("locationModel", UnifiedModelType::Image),
    ("storyboardModel", UnifiedModelType::Image),
    ("editModel", UnifiedModelType::Image),
    ("videoModel", UnifiedModelType::Video),
    ("lipSyncModel", UnifiedModelType::Lipsync),
];

#[derive(Debug, Deserialize)]
pub struct AiConfigPutBody {
    providers: Vec<StoredProvider>,
    models: Vec<StoredModel>,
    #[serde(rename = "defaultModels")]
    default_models: Option<Value>,
    #[serde(rename = "capabilityDefaults")]
    capability_defaults: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredProvider {
    id: String,
    name: String,
    #[serde(rename = "baseUrl")]
    base_url: Option<String>,
    #[serde(rename = "apiKey")]
    api_key: Option<String>,
    #[serde(rename = "apiMode")]
    api_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredModel {
    #[serde(rename = "modelId")]
    model_id: String,
    #[serde(rename = "modelKey")]
    model_key: String,
    name: String,
    #[serde(rename = "type")]
    model_type: UnifiedModelType,
    provider: String,
    enabled: bool,
    price: f64,
    #[serde(rename = "customPricing")]
    custom_pricing: Option<Value>,
}

fn sanitize_provider_id(value: &str) -> String {
    value.trim().to_string()
}

fn provider_key(value: &str) -> &str {
    value.split(':').next().unwrap_or("")
}

fn assert_allowed_provider(id: &str, field: &str) -> Result<(), AppError> {
    let key = provider_key(id);
    let allowed = [
        "fal",
        "qwen",
        "openai-compatible",
        "gemini-compatible",
        "anthropic",
        "jimeng",
    ];
    if allowed.contains(&key) {
        Ok(())
    } else {
        Err(AppError::invalid_params(format!(
            "{field} provider key is not allowed"
        )))
    }
}

fn parse_model_key_strict(value: &str) -> Option<(String, String)> {
    let trimmed = value.trim();
    let mut parts = trimmed.split("::");
    let provider = parts.next()?.trim();
    let model_id = parts.next()?.trim();
    if provider.is_empty() || model_id.is_empty() || parts.next().is_some() {
        return None;
    }
    Some((provider.to_string(), model_id.to_string()))
}

fn validate_default_models(
    raw: &Value,
    enabled_models: &HashMap<String, UnifiedModelType>,
) -> Result<(), AppError> {
    let object = raw
        .as_object()
        .ok_or_else(|| AppError::invalid_params("defaultModels must be an object"))?;

    let expected_fields = DEFAULT_MODEL_FIELDS
        .iter()
        .map(|(field, _)| *field)
        .collect::<HashSet<_>>();
    for field in object.keys() {
        if !expected_fields.contains(field.as_str()) {
            return Err(AppError::invalid_params(format!(
                "defaultModels.{field} is not supported"
            )));
        }
    }

    for (field, expected_type) in DEFAULT_MODEL_FIELDS {
        let Some(raw_value) = object.get(field) else {
            continue;
        };
        let value = raw_value.as_str().unwrap_or("").trim();
        if value.is_empty() {
            continue;
        }

        let (provider, model_id) = parse_model_key_strict(value).ok_or_else(|| {
            AppError::invalid_params(format!("defaultModels.{field} must be provider::modelId"))
        })?;
        let normalized_key = format!("{provider}::{model_id}");
        let actual_type = enabled_models.get(&normalized_key).ok_or_else(|| {
            AppError::invalid_params(format!(
                "defaultModels.{field} points to disabled/missing model"
            ))
        })?;
        if actual_type != &expected_type {
            return Err(AppError::invalid_params(format!(
                "defaultModels.{field} has invalid model type"
            )));
        }
    }

    Ok(())
}

fn validate_payload(payload: &AiConfigPutBody) -> Result<(), AppError> {
    for (index, provider) in payload.providers.iter().enumerate() {
        let id = sanitize_provider_id(&provider.id);
        if id.is_empty() || provider.name.trim().is_empty() {
            return Err(AppError::invalid_params(format!(
                "providers[{index}] missing id or name"
            )));
        }
        assert_allowed_provider(&id, &format!("providers[{index}].id"))?;
    }

    let mut enabled_models: HashMap<String, UnifiedModelType> = HashMap::new();
    for (index, model) in payload.models.iter().enumerate() {
        if model.model_id.trim().is_empty() {
            return Err(AppError::invalid_params(format!(
                "models[{index}].modelId is required"
            )));
        }

        let parsed = parse_model_key_strict(&model.model_key)
            .ok_or_else(|| AppError::invalid_params(format!("models[{index}].modelKey invalid")))?;

        if parsed.0 != model.provider.trim() || parsed.1 != model.model_id.trim() {
            return Err(AppError::invalid_params(format!(
                "models[{index}] provider/modelId mismatch with modelKey"
            )));
        }

        assert_allowed_provider(&model.provider, &format!("models[{index}].provider"))?;

        if model.enabled {
            enabled_models.insert(model.model_key.trim().to_string(), model.model_type);
        }
    }

    if let Some(default_models) = payload.default_models.as_ref() {
        validate_default_models(default_models, &enabled_models)?;
    }

    if let Some(capability_defaults) = payload.capability_defaults.as_ref()
        && !capability_defaults.is_object()
    {
        return Err(AppError::invalid_params(
            "capabilityDefaults must be a json object",
        ));
    }

    Ok(())
}

fn map_response(
    providers: Vec<CustomProvider>,
    models: Vec<CustomModel>,
    default_models: DefaultModelsPayload,
    capability_defaults: Value,
) -> Value {
    json!({
      "providers": providers.into_iter().map(|provider| {
        json!({
          "id": provider.id,
          "name": provider.name,
          "baseUrl": provider.base_url,
          "apiMode": provider.api_mode,
          "apiKey": provider.api_key.unwrap_or_default(),
        })
      }).collect::<Vec<_>>(),
      "models": models,
      "defaultModels": default_models,
      "capabilityDefaults": capability_defaults,
    })
}

async fn read_payload(state: &AppState) -> Result<Value, AppError> {
    let providers = get_system_providers(&state.mysql).await?;
    let models = get_system_models_raw(&state.mysql).await?;
    let default_models = get_system_default_models(&state.mysql).await?;
    let capability_defaults = get_system_capability_defaults(&state.mysql).await?;
    Ok(map_response(
        providers,
        models,
        default_models,
        capability_defaults,
    ))
}

async fn upsert_system_config(
    state: &AppState,
    admin_user_id: &str,
    key: &str,
    value: String,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO system_config (`key`, `value`, updatedAt, updatedBy) VALUES (?, ?, NOW(3), ?) ON DUPLICATE KEY UPDATE `value` = VALUES(`value`), updatedAt = NOW(3), updatedBy = VALUES(updatedBy)",
    )
    .bind(key)
    .bind(value)
    .bind(admin_user_id)
    .execute(&state.mysql)
    .await?;

    Ok(())
}

pub async fn get(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<Value>, AppError> {
    Ok(Json(read_payload(&state).await?))
}

pub async fn update(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(payload): Json<AiConfigPutBody>,
) -> Result<Json<Value>, AppError> {
    validate_payload(&payload)?;

    let providers_json = serde_json::to_string(&payload.providers)
        .map_err(|err| AppError::internal(format!("failed to serialize providers: {err}")))?;
    let models_json = serde_json::to_string(&payload.models)
        .map_err(|err| AppError::internal(format!("failed to serialize models: {err}")))?;
    let default_models_json = payload
        .default_models
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|err| AppError::internal(format!("failed to serialize defaultModels: {err}")))?;
    let capability_defaults_json = payload
        .capability_defaults
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|err| {
            AppError::internal(format!("failed to serialize capabilityDefaults: {err}"))
        })?;

    upsert_system_config(&state, &admin.0.id, AI_PROVIDERS_CONFIG_KEY, providers_json).await?;
    upsert_system_config(&state, &admin.0.id, AI_MODELS_CONFIG_KEY, models_json).await?;
    if let Some(default_models_json) = default_models_json {
        upsert_system_config(
            &state,
            &admin.0.id,
            AI_DEFAULT_MODELS_CONFIG_KEY,
            default_models_json,
        )
        .await?;
    }
    if let Some(capability_defaults_json) = capability_defaults_json {
        upsert_system_config(
            &state,
            &admin.0.id,
            AI_CAPABILITY_DEFAULTS_CONFIG_KEY,
            capability_defaults_json,
        )
        .await?;
    }

    Ok(Json(read_payload(&state).await?))
}

pub fn router() -> axum::Router<AppState> {
    axum::Router::new().route("/api/admin/ai-config", axum::routing::get(get).put(update))
}
