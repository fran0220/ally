use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use waoowaoo_core::api_config::{
    CustomModel, CustomProvider, UnifiedModelType, get_system_models_raw, get_system_providers,
};

use crate::{app_state::AppState, error::AppError, extractors::auth::AdminUser};

const AI_MODELS_CONFIG_KEY: &str = "ai_models";
const AI_PROVIDERS_CONFIG_KEY: &str = "ai_providers";

#[derive(Debug, Deserialize)]
pub struct AiConfigPutBody {
    providers: Vec<StoredProvider>,
    models: Vec<StoredModel>,
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
    let allowed = ["fal", "qwen", "openai-compatible", "gemini-compatible"];
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

fn validate_payload(payload: &AiConfigPutBody) -> Result<(), AppError> {
    for (index, provider) in payload.providers.iter().enumerate() {
        let id = sanitize_provider_id(&provider.id);
        if id.is_empty() || provider.name.trim().is_empty() {
            return Err(AppError::invalid_params(format!(
                "providers[{index}] missing id or name"
            )));
        }
        assert_allowed_provider(&id, &format!("providers[{index}].id"))?;
        if let Some(mode) = &provider.api_mode
            && !mode.trim().is_empty()
            && mode != "gemini-sdk"
        {
            return Err(AppError::invalid_params(format!(
                "providers[{index}].apiMode invalid"
            )));
        }
    }

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
    }

    Ok(())
}

fn map_response(providers: Vec<CustomProvider>, models: Vec<CustomModel>) -> Value {
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
    })
}

async fn read_payload(state: &AppState) -> Result<Value, AppError> {
    let providers = get_system_providers(&state.mysql).await?;
    let models = get_system_models_raw(&state.mysql).await?;
    Ok(map_response(providers, models))
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

    sqlx::query(
        "INSERT INTO system_config (`key`, `value`, updatedAt, updatedBy) VALUES (?, ?, NOW(3), ?) ON DUPLICATE KEY UPDATE `value` = VALUES(`value`), updatedAt = NOW(3), updatedBy = VALUES(updatedBy)",
    )
    .bind(AI_PROVIDERS_CONFIG_KEY)
    .bind(providers_json)
    .bind(&admin.0.id)
    .execute(&state.mysql)
    .await?;

    sqlx::query(
        "INSERT INTO system_config (`key`, `value`, updatedAt, updatedBy) VALUES (?, ?, NOW(3), ?) ON DUPLICATE KEY UPDATE `value` = VALUES(`value`), updatedAt = NOW(3), updatedBy = VALUES(updatedBy)",
    )
    .bind(AI_MODELS_CONFIG_KEY)
    .bind(models_json)
    .bind(&admin.0.id)
    .execute(&state.mysql)
    .await?;

    Ok(Json(read_payload(&state).await?))
}

pub fn router() -> axum::Router<AppState> {
    axum::Router::new().route("/api/admin/ai-config", axum::routing::get(get).put(update))
}
