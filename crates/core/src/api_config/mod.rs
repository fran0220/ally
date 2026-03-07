use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sqlx::MySqlPool;

use crate::errors::AppError;

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UnifiedModelType {
    Llm,
    Image,
    Video,
    Audio,
    Lipsync,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCustomPricing {
    pub input: Option<f64>,
    pub output: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomModel {
    #[serde(rename = "modelId")]
    pub model_id: String,
    #[serde(rename = "modelKey")]
    pub model_key: String,
    pub name: String,
    pub provider: String,
    #[serde(rename = "type")]
    pub model_type: UnifiedModelType,
    pub enabled: bool,
    #[serde(
        rename = "customPricing",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_pricing: Option<ModelCustomPricing>,
    #[serde(default)]
    pub price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProvider {
    pub id: String,
    pub name: String,
    #[serde(rename = "baseUrl")]
    pub base_url: Option<String>,
    #[serde(rename = "apiMode")]
    pub api_mode: Option<String>,
    #[serde(rename = "apiKey")]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserApiProviderView {
    pub id: String,
    pub name: String,
    #[serde(rename = "baseUrl")]
    pub base_url: Option<String>,
    #[serde(rename = "apiMode")]
    pub api_mode: Option<String>,
    #[serde(rename = "hasApiKey")]
    pub has_api_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefaultModelsPayload {
    #[serde(rename = "analysisModel", default)]
    pub analysis_model: String,
    #[serde(rename = "characterModel", default)]
    pub character_model: String,
    #[serde(rename = "locationModel", default)]
    pub location_model: String,
    #[serde(rename = "storyboardModel", default)]
    pub storyboard_model: String,
    #[serde(rename = "editModel", default)]
    pub edit_model: String,
    #[serde(rename = "videoModel", default)]
    pub video_model: String,
    #[serde(rename = "lipSyncModel", default)]
    pub lip_sync_model: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserApiConfigResponse {
    pub models: Vec<CustomModel>,
    pub providers: Vec<UserApiProviderView>,
    #[serde(rename = "defaultModels")]
    pub default_models: DefaultModelsPayload,
    #[serde(rename = "capabilityDefaults")]
    pub capability_defaults: Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateUserApiConfigInput {
    #[serde(rename = "defaultModels")]
    pub default_models: Option<Value>,
    #[serde(rename = "capabilityDefaults")]
    pub capability_defaults: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct ParsedModelKey {
    pub provider: String,
    pub model_id: String,
    pub model_key: String,
}

#[derive(sqlx::FromRow)]
struct SystemConfigRow {
    value: String,
}

#[derive(sqlx::FromRow)]
struct PreferenceRow {
    #[sqlx(rename = "analysisModel")]
    analysis_model: Option<String>,
    #[sqlx(rename = "characterModel")]
    character_model: Option<String>,
    #[sqlx(rename = "locationModel")]
    location_model: Option<String>,
    #[sqlx(rename = "storyboardModel")]
    storyboard_model: Option<String>,
    #[sqlx(rename = "editModel")]
    edit_model: Option<String>,
    #[sqlx(rename = "videoModel")]
    video_model: Option<String>,
    #[sqlx(rename = "lipSyncModel")]
    lip_sync_model: Option<String>,
    #[sqlx(rename = "capabilityDefaults")]
    capability_defaults: Option<String>,
}

pub fn parse_model_key_strict(raw: &str) -> Option<ParsedModelKey> {
    let raw = raw.trim();
    let marker_index = raw.find("::")?;
    let provider = raw[..marker_index].trim();
    let model_id = raw[marker_index + 2..].trim();
    if provider.is_empty() || model_id.is_empty() {
        return None;
    }
    Some(ParsedModelKey {
        provider: provider.to_string(),
        model_id: model_id.to_string(),
        model_key: format!("{provider}::{model_id}"),
    })
}

async fn get_system_config_value(
    pool: &MySqlPool,
    key: &str,
    fallback: &str,
) -> Result<String, AppError> {
    let row = sqlx::query_as::<_, SystemConfigRow>(
        "SELECT `value` FROM system_config WHERE `key` = ? LIMIT 1",
    )
    .bind(key)
    .fetch_optional(pool)
    .await?;

    Ok(row
        .map(|item| item.value)
        .unwrap_or_else(|| fallback.to_string()))
}

pub async fn get_system_models_raw(pool: &MySqlPool) -> Result<Vec<CustomModel>, AppError> {
    let models_raw = get_system_config_value(pool, AI_MODELS_CONFIG_KEY, "[]").await?;

    serde_json::from_str::<Vec<CustomModel>>(&models_raw)
        .map_err(|err| AppError::internal(format!("invalid ai_models config json: {err}")))
}

pub async fn get_system_providers(pool: &MySqlPool) -> Result<Vec<CustomProvider>, AppError> {
    let raw = get_system_config_value(pool, AI_PROVIDERS_CONFIG_KEY, "[]").await?;
    serde_json::from_str::<Vec<CustomProvider>>(&raw)
        .map_err(|err| AppError::internal(format!("invalid ai_providers config json: {err}")))
}

pub async fn get_system_default_models(pool: &MySqlPool) -> Result<DefaultModelsPayload, AppError> {
    let raw = get_system_config_value(pool, AI_DEFAULT_MODELS_CONFIG_KEY, "{}").await?;
    serde_json::from_str::<DefaultModelsPayload>(&raw)
        .map_err(|err| AppError::internal(format!("invalid ai_default_models config json: {err}")))
}

pub async fn get_system_capability_defaults(pool: &MySqlPool) -> Result<Value, AppError> {
    let raw = get_system_config_value(pool, AI_CAPABILITY_DEFAULTS_CONFIG_KEY, "{}").await?;
    let parsed = serde_json::from_str::<Value>(&raw).map_err(|err| {
        AppError::internal(format!("invalid ai_capability_defaults config json: {err}"))
    })?;
    if !parsed.is_object() {
        return Err(AppError::internal(
            "invalid ai_capability_defaults config json: value must be a json object",
        ));
    }
    Ok(parsed)
}

fn sanitize_default_model(
    value: Option<&str>,
    expected_type: UnifiedModelType,
    enabled_models: &HashMap<String, CustomModel>,
) -> String {
    let Some(value) = value.map(str::trim).filter(|item| !item.is_empty()) else {
        return String::new();
    };
    let Some(parsed) = parse_model_key_strict(value) else {
        return String::new();
    };
    let Some(model) = enabled_models.get(&parsed.model_key) else {
        return String::new();
    };
    if model.model_type != expected_type {
        return String::new();
    }
    parsed.model_key
}

fn resolve_default_model_with_fallback(
    user_value: Option<&str>,
    system_value: Option<&str>,
    expected_type: UnifiedModelType,
    enabled_models: &HashMap<String, CustomModel>,
) -> String {
    let user_model = sanitize_default_model(user_value, expected_type, enabled_models);
    if !user_model.is_empty() {
        return user_model;
    }
    sanitize_default_model(system_value, expected_type, enabled_models)
}

fn parse_capability_defaults(raw: Option<&str>, error_prefix: &str) -> Result<Value, AppError> {
    match raw {
        Some(value) if !value.trim().is_empty() => {
            let parsed = serde_json::from_str::<Value>(value)
                .map_err(|err| AppError::invalid_params(format!("{error_prefix}: {err}")))?;
            if !parsed.is_object() {
                return Err(AppError::invalid_params(
                    "capabilityDefaults must be a json object",
                ));
            }
            Ok(parsed)
        }
        _ => Ok(json!({})),
    }
}

fn parse_stored_capability_defaults(raw: Option<&str>) -> Result<Value, AppError> {
    parse_capability_defaults(raw, "invalid capabilityDefaults json")
}

fn merge_json_objects(base: &mut Map<String, Value>, overlay: &Map<String, Value>) {
    for (key, overlay_value) in overlay {
        match (base.get_mut(key), overlay_value) {
            (Some(Value::Object(base_object)), Value::Object(overlay_object)) => {
                merge_json_objects(base_object, overlay_object);
            }
            _ => {
                base.insert(key.clone(), overlay_value.clone());
            }
        }
    }
}

fn merge_capability_defaults(system: Value, user: Value) -> Value {
    let mut merged = match system {
        Value::Object(object) => object,
        _ => Map::new(),
    };
    if let Value::Object(overlay) = user {
        merge_json_objects(&mut merged, &overlay);
    }
    Value::Object(merged)
}

pub async fn read_user_api_config(
    pool: &MySqlPool,
    user_id: &str,
) -> Result<UserApiConfigResponse, AppError> {
    let models = get_system_models_raw(pool).await?;
    let providers = get_system_providers(pool).await?;
    let system_default_models = get_system_default_models(pool).await?;
    let system_capability_defaults = get_system_capability_defaults(pool).await?;

    let pref = sqlx::query_as::<_, PreferenceRow>(
        "SELECT analysisModel, characterModel, locationModel, storyboardModel, editModel, videoModel, lipSyncModel, capabilityDefaults FROM user_preferences WHERE userId = ? LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let enabled_model_map = models
        .iter()
        .filter(|model| model.enabled)
        .map(|model| (model.model_key.clone(), model.clone()))
        .collect::<HashMap<_, _>>();

    let default_models = DefaultModelsPayload {
        analysis_model: resolve_default_model_with_fallback(
            pref.as_ref()
                .and_then(|item| item.analysis_model.as_deref()),
            Some(system_default_models.analysis_model.as_str()),
            UnifiedModelType::Llm,
            &enabled_model_map,
        ),
        character_model: resolve_default_model_with_fallback(
            pref.as_ref()
                .and_then(|item| item.character_model.as_deref()),
            Some(system_default_models.character_model.as_str()),
            UnifiedModelType::Image,
            &enabled_model_map,
        ),
        location_model: resolve_default_model_with_fallback(
            pref.as_ref()
                .and_then(|item| item.location_model.as_deref()),
            Some(system_default_models.location_model.as_str()),
            UnifiedModelType::Image,
            &enabled_model_map,
        ),
        storyboard_model: resolve_default_model_with_fallback(
            pref.as_ref()
                .and_then(|item| item.storyboard_model.as_deref()),
            Some(system_default_models.storyboard_model.as_str()),
            UnifiedModelType::Image,
            &enabled_model_map,
        ),
        edit_model: resolve_default_model_with_fallback(
            pref.as_ref().and_then(|item| item.edit_model.as_deref()),
            Some(system_default_models.edit_model.as_str()),
            UnifiedModelType::Image,
            &enabled_model_map,
        ),
        video_model: resolve_default_model_with_fallback(
            pref.as_ref().and_then(|item| item.video_model.as_deref()),
            Some(system_default_models.video_model.as_str()),
            UnifiedModelType::Video,
            &enabled_model_map,
        ),
        lip_sync_model: resolve_default_model_with_fallback(
            pref.as_ref()
                .and_then(|item| item.lip_sync_model.as_deref()),
            Some(system_default_models.lip_sync_model.as_str()),
            UnifiedModelType::Lipsync,
            &enabled_model_map,
        ),
    };

    Ok(UserApiConfigResponse {
        models,
        providers: providers
            .into_iter()
            .map(|provider| {
                let has_api_key = provider
                    .api_key
                    .as_ref()
                    .map(|api_key| !api_key.trim().is_empty())
                    .unwrap_or(false);
                UserApiProviderView {
                    id: provider.id,
                    name: provider.name,
                    base_url: provider.base_url,
                    api_mode: provider.api_mode,
                    has_api_key,
                }
            })
            .collect(),
        default_models,
        capability_defaults: merge_capability_defaults(
            system_capability_defaults,
            parse_stored_capability_defaults(
                pref.as_ref()
                    .and_then(|item| item.capability_defaults.as_deref()),
            )?,
        ),
    })
}

fn validate_and_normalize_default_models(
    value: Value,
    enabled_models: &HashMap<String, CustomModel>,
) -> Result<Map<String, Value>, AppError> {
    let object = value
        .as_object()
        .ok_or_else(|| AppError::invalid_params("defaultModels must be an object"))?;

    let mut update = Map::new();

    for (field, expected_type) in DEFAULT_MODEL_FIELDS {
        if let Some(raw_value) = object.get(field) {
            let value = raw_value.as_str().unwrap_or("").trim();
            if value.is_empty() {
                update.insert(field.to_string(), Value::Null);
                continue;
            }

            let parsed = parse_model_key_strict(value).ok_or_else(|| {
                AppError::invalid_params(format!("{field} must be provider::modelId"))
            })?;
            let model = enabled_models.get(&parsed.model_key).ok_or_else(|| {
                AppError::invalid_params(format!("{field} points to disabled/missing model"))
            })?;
            if model.model_type != expected_type {
                return Err(AppError::invalid_params(format!(
                    "{field} has invalid model type"
                )));
            }
            update.insert(field.to_string(), Value::String(parsed.model_key));
        }
    }

    Ok(update)
}

pub async fn update_user_api_config(
    pool: &MySqlPool,
    user_id: &str,
    input: UpdateUserApiConfigInput,
) -> Result<UserApiConfigResponse, AppError> {
    let models = get_system_models_raw(pool).await?;
    let enabled_model_map = models
        .iter()
        .filter(|model| model.enabled)
        .map(|model| (model.model_key.clone(), model.clone()))
        .collect::<HashMap<_, _>>();

    let mut updates = Map::new();

    if let Some(default_models) = input.default_models {
        let normalized = validate_and_normalize_default_models(default_models, &enabled_model_map)?;
        updates.extend(normalized);
    }

    if let Some(capability_defaults) = input.capability_defaults {
        if !capability_defaults.is_object() {
            return Err(AppError::invalid_params(
                "capabilityDefaults must be a json object",
            ));
        }
        let normalized = if capability_defaults
            .as_object()
            .map(|value| value.is_empty())
            .unwrap_or(true)
        {
            Value::Null
        } else {
            Value::String(capability_defaults.to_string())
        };
        updates.insert("capabilityDefaults".to_string(), normalized);
    }

    if updates.is_empty() {
        return Err(AppError::invalid_params(
            "request body must include defaultModels or capabilityDefaults",
        ));
    }

    let existing_id: Option<(String,)> =
        sqlx::query_as("SELECT id FROM user_preferences WHERE userId = ? LIMIT 1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

    if existing_id.is_none() {
        sqlx::query(
            "INSERT INTO user_preferences (id, userId, videoRatio, videoResolution, artStyle, ttsRate, imageResolution, createdAt, updatedAt) VALUES (?, ?, '9:16', '720p', 'american-comic', '+50%', '2K', NOW(3), NOW(3))",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(user_id)
        .execute(pool)
        .await?;
    }

    let mut set_clauses: Vec<String> = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();

    for (column, value) in &updates {
        match value {
            Value::Null => {
                set_clauses.push(format!("{column} = NULL"));
            }
            Value::String(item) => {
                set_clauses.push(format!("{column} = ?"));
                bind_values.push(item.clone());
            }
            _ => {
                return Err(AppError::invalid_params(format!(
                    "{column} must be string or null"
                )));
            }
        }
    }
    set_clauses.push("updatedAt = NOW(3)".to_string());

    let sql = format!(
        "UPDATE user_preferences SET {} WHERE userId = ?",
        set_clauses.join(", ")
    );
    let mut query = sqlx::query(&sql);
    for val in &bind_values {
        query = query.bind(val);
    }
    query = query.bind(user_id);
    query.execute(pool).await?;

    read_user_api_config(pool, user_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn build_model(model_key: &str, model_type: UnifiedModelType) -> CustomModel {
        let parsed = parse_model_key_strict(model_key).expect("valid model key");
        CustomModel {
            model_id: parsed.model_id,
            model_key: parsed.model_key.clone(),
            name: "Test".to_string(),
            provider: parsed.provider,
            model_type,
            enabled: true,
            custom_pricing: None,
            price: 0.0,
        }
    }

    #[test]
    fn parse_model_key_is_strict() {
        assert!(parse_model_key_strict("openai-compatible::gpt-4.1").is_some());
        assert!(parse_model_key_strict("gpt-4.1").is_none());
        let parsed = parse_model_key_strict("openai-compatible::foo::bar")
            .expect("model key should split on first delimiter");
        assert_eq!(parsed.provider, "openai-compatible");
        assert_eq!(parsed.model_id, "foo::bar");
    }

    #[test]
    fn default_model_uses_system_fallback_when_user_value_is_missing_or_invalid() {
        let system_model_key = "openai-compatible::gpt-4.1";
        let system_model = build_model(system_model_key, UnifiedModelType::Llm);
        let enabled_models = HashMap::from([(system_model.model_key.clone(), system_model)]);

        let resolved_missing = resolve_default_model_with_fallback(
            None,
            Some(system_model_key),
            UnifiedModelType::Llm,
            &enabled_models,
        );
        assert_eq!(resolved_missing, system_model_key);

        let resolved_invalid_user = resolve_default_model_with_fallback(
            Some("openai-compatible::missing-model"),
            Some(system_model_key),
            UnifiedModelType::Llm,
            &enabled_models,
        );
        assert_eq!(resolved_invalid_user, system_model_key);
    }

    #[test]
    fn capability_defaults_merge_preserves_system_values_and_overrides_user_values() {
        let system = json!({
            "fal::image-a": {
                "ratio": "16:9",
                "style": "realistic"
            },
            "qwen::video-a": {
                "fps": 24
            }
        });
        let user = json!({
            "fal::image-a": {
                "style": "anime"
            },
            "openai-compatible::gpt-4.1": {
                "reasoningEffort": "high"
            }
        });

        let merged = merge_capability_defaults(system, user);
        assert_eq!(merged["fal::image-a"]["ratio"], "16:9");
        assert_eq!(merged["fal::image-a"]["style"], "anime");
        assert_eq!(merged["qwen::video-a"]["fps"], 24);
        assert_eq!(
            merged["openai-compatible::gpt-4.1"]["reasoningEffort"],
            "high"
        );
    }
}
