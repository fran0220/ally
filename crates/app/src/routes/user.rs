use std::{collections::HashMap, time::Duration};

use axum::{Json, extract::State, http::HeaderMap as AxumHeaderMap};
use chrono::{NaiveDateTime, Utc};
use reqwest::header::{AUTHORIZATION, HeaderMap as ReqwestHeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{MySql, QueryBuilder};
use waoowaoo_core::api_config::{
    CustomModel, UnifiedModelType, UpdateUserApiConfigInput, get_system_models_raw,
    get_system_providers, read_user_api_config, update_user_api_config,
};

use crate::{app_state::AppState, error::AppError, extractors::auth::AuthUser};

#[derive(Debug, Deserialize)]
pub struct TestConnectionRequest {
    pub provider: Option<String>,
    #[serde(rename = "apiKey")]
    pub api_key: Option<String>,
    #[serde(rename = "baseUrl")]
    pub base_url: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserModelOption {
    value: String,
    label: String,
    provider: String,
    #[serde(rename = "providerName")]
    provider_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserModelsPayload {
    llm: Vec<UserModelOption>,
    image: Vec<UserModelOption>,
    video: Vec<UserModelOption>,
    audio: Vec<UserModelOption>,
    lipsync: Vec<UserModelOption>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct PreferenceRow {
    id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
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
    #[sqlx(rename = "videoRatio")]
    video_ratio: String,
    #[sqlx(rename = "artStyle")]
    art_style: String,
    #[sqlx(rename = "ttsRate")]
    tts_rate: String,
    #[sqlx(rename = "capabilityDefaults")]
    capability_defaults: Option<String>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

async fn build_api_config_response(
    pool: &sqlx::MySqlPool,
    user_id: &str,
) -> Result<Value, AppError> {
    let data = read_user_api_config(pool, user_id).await?;
    serde_json::to_value(data)
        .map_err(|err| AppError::internal(format!("failed to serialize api-config: {err}")))
}

fn push_grouped_model(
    grouped: &mut UserModelsPayload,
    model: CustomModel,
    provider_name: Option<String>,
) {
    let option = UserModelOption {
        value: model.model_key,
        label: model.name,
        provider: model.provider,
        provider_name,
    };

    match model.model_type {
        UnifiedModelType::Llm => grouped.llm.push(option),
        UnifiedModelType::Image => grouped.image.push(option),
        UnifiedModelType::Video => grouped.video.push(option),
        UnifiedModelType::Audio => grouped.audio.push(option),
        UnifiedModelType::Lipsync => grouped.lipsync.push(option),
    }
}

fn dedupe_models(items: Vec<UserModelOption>) -> Vec<UserModelOption> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for item in items {
        if seen.insert(item.value.clone()) {
            out.push(item);
        }
    }
    out
}

pub async fn models(
    State(state): State<AppState>,
    _user: AuthUser,
) -> Result<Json<UserModelsPayload>, AppError> {
    let models = get_system_models_raw(&state.mysql).await?;
    let providers = get_system_providers(&state.mysql).await?;

    let mut provider_name_map: HashMap<String, String> = HashMap::new();
    let mut enabled_provider_ids = std::collections::HashSet::new();

    for provider in providers {
        provider_name_map.insert(provider.id.clone(), provider.name.clone());
        if provider
            .api_key
            .as_ref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
        {
            enabled_provider_ids.insert(provider.id);
        }
    }

    let mut grouped = UserModelsPayload {
        llm: Vec::new(),
        image: Vec::new(),
        video: Vec::new(),
        audio: Vec::new(),
        lipsync: Vec::new(),
    };

    for model in models {
        if !model.enabled {
            continue;
        }
        if !enabled_provider_ids.contains(&model.provider) {
            continue;
        }
        let provider_name = provider_name_map.get(&model.provider).cloned();
        push_grouped_model(&mut grouped, model, provider_name);
    }

    grouped.llm = dedupe_models(grouped.llm);
    grouped.image = dedupe_models(grouped.image);
    grouped.video = dedupe_models(grouped.video);
    grouped.audio = dedupe_models(grouped.audio);
    grouped.lipsync = dedupe_models(grouped.lipsync);

    Ok(Json(grouped))
}

pub async fn get_api_config(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Value>, AppError> {
    let payload = build_api_config_response(&state.mysql, &user.id).await?;
    Ok(Json(payload))
}

pub async fn update_api_config(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let object = payload
        .as_object()
        .ok_or_else(|| AppError::invalid_params("request body must be an object"))?;

    let has_default_models = object.contains_key("defaultModels");
    let has_capability_defaults = object.contains_key("capabilityDefaults");

    if !has_default_models && !has_capability_defaults {
        return Err(AppError::invalid_params(
            "request body must include defaultModels or capabilityDefaults",
        ));
    }

    if has_default_models || has_capability_defaults {
        let mut core_payload = serde_json::Map::new();
        if let Some(default_models) = object.get("defaultModels") {
            core_payload.insert("defaultModels".to_string(), default_models.clone());
        }
        if let Some(capability_defaults) = object.get("capabilityDefaults") {
            core_payload.insert(
                "capabilityDefaults".to_string(),
                capability_defaults.clone(),
            );
        }

        let normalized_payload: UpdateUserApiConfigInput =
            serde_json::from_value(Value::Object(core_payload)).map_err(|err| {
                AppError::invalid_params(format!("invalid api-config payload: {err}"))
            })?;

        update_user_api_config(&state.mysql, &user.id, normalized_payload).await?;
    }

    let response = build_api_config_response(&state.mysql, &user.id).await?;
    Ok(Json(response))
}

fn normalize_provider(input: Option<String>, base_url: Option<&str>) -> Result<String, AppError> {
    let provider = input
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .to_lowercase();

    if provider.is_empty() {
        if base_url.unwrap_or_default().trim().is_empty() {
            return Err(AppError::invalid_params("missing required field provider"));
        }
        return Ok("custom".to_string());
    }

    match provider.as_str() {
        "openrouter" | "google" | "anthropic" | "openai" | "custom" => Ok(provider),
        _ => Err(AppError::invalid_params(format!(
            "unsupported provider: {provider}"
        ))),
    }
}

fn request_locale(headers: &AxumHeaderMap) -> &'static str {
    let is_english = headers
        .get(axum::http::header::ACCEPT_LANGUAGE)
        .and_then(|raw| raw.to_str().ok())
        .and_then(|raw| raw.split(',').next())
        .and_then(|raw| raw.split(';').next())
        .map(str::trim)
        .map(|raw| raw.to_ascii_lowercase())
        .filter(|raw| !raw.is_empty())
        .is_some_and(|raw| raw == "en" || raw.starts_with("en-"));
    if is_english { "en" } else { "zh" }
}

fn localized_msg<'a>(locale: &str, zh: &'a str, en: &'a str) -> &'a str {
    if locale == "en" { en } else { zh }
}

fn normalize_openai_base_url(provider: &str, base_url: Option<String>) -> Result<String, AppError> {
    match provider {
        "openrouter" => Ok("https://openrouter.ai/api/v1".to_string()),
        "openai" => Ok("https://api.openai.com/v1".to_string()),
        "anthropic" => Ok("https://api.anthropic.com/v1".to_string()),
        "custom" => {
            let Some(raw) = base_url else {
                return Err(AppError::invalid_params("custom provider requires baseUrl"));
            };
            let mut normalized = raw.trim().to_string();
            if normalized.is_empty() {
                return Err(AppError::invalid_params("custom provider requires baseUrl"));
            }
            if !normalized.ends_with("/v1") {
                normalized = format!("{}/v1", normalized.trim_end_matches('/'));
            }
            Ok(normalized)
        }
        _ => Err(AppError::invalid_params(
            "provider is not openai-compatible",
        )),
    }
}

async fn test_openai_compatible(
    provider: &str,
    api_key: &str,
    base_url: String,
    locale: &str,
) -> Result<Value, AppError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| AppError::internal(format!("failed to build http client: {err}")))?;

    let mut headers = ReqwestHeaderMap::new();
    let bearer = format!("Bearer {api_key}");
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&bearer)
            .map_err(|err| AppError::invalid_params(format!("invalid api key header: {err}")))?,
    );

    if provider == "anthropic" {
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    }

    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let response = client
        .get(url)
        .headers(headers)
        .send()
        .await
        .map_err(|err| AppError::internal(format!("request failed: {err}")))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(AppError::invalid_params(format!(
            "provider auth failed (status {}): {}",
            status, body
        )));
    }

    Ok(json!({
        "provider": provider,
        "message": format!(
            "{provider} {}",
            localized_msg(locale, "连接成功", "Connection successful")
        ),
    }))
}

async fn test_google(api_key: &str, locale: &str) -> Result<Value, AppError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| AppError::internal(format!("failed to build http client: {err}")))?;

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        urlencoding::encode(api_key)
    );
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|err| AppError::internal(format!("google request failed: {err}")))?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::invalid_params(format!(
            "google auth failed: {}",
            body
        )));
    }

    Ok(json!({
        "provider": "google",
        "message": format!(
            "google {}",
            localized_msg(locale, "连接成功", "Connection successful")
        ),
    }))
}

pub async fn test_connection(
    _user: AuthUser,
    headers: AxumHeaderMap,
    Json(payload): Json<TestConnectionRequest>,
) -> Result<Json<Value>, AppError> {
    let locale = request_locale(&headers);
    let api_key = payload
        .api_key
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    if api_key.is_empty() {
        return Err(AppError::invalid_params("missing required field apiKey"));
    }

    let provider = normalize_provider(payload.provider, payload.base_url.as_deref())?;
    let started_at = Utc::now();

    let result = match provider.as_str() {
        "google" => test_google(&api_key, locale).await?,
        "openrouter" | "openai" | "anthropic" | "custom" => {
            let base_url = normalize_openai_base_url(&provider, payload.base_url)?;
            test_openai_compatible(&provider, &api_key, base_url, locale).await?
        }
        _ => return Err(AppError::invalid_params("unsupported provider")),
    };

    let latency_ms = (Utc::now() - started_at).num_milliseconds();

    Ok(Json(json!({
        "success": true,
        "latencyMs": latency_ms,
        "model": payload.model,
        "provider": result.get("provider").cloned().unwrap_or(Value::String(provider)),
        "message": result.get("message").cloned().unwrap_or_else(|| {
            Value::String(localized_msg(locale, "连接成功", "Connection successful").to_string())
        }),
    })))
}

async fn ensure_preference_exists(pool: &sqlx::MySqlPool, user_id: &str) -> Result<(), AppError> {
    let exists: Option<(String,)> =
        sqlx::query_as("SELECT id FROM user_preferences WHERE userId = ? LIMIT 1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

    if exists.is_none() {
        sqlx::query(
            "INSERT INTO user_preferences (id, userId, videoRatio, videoResolution, artStyle, ttsRate, imageResolution, createdAt, updatedAt) VALUES (?, ?, '9:16', '720p', 'american-comic', '+50%', '2K', NOW(3), NOW(3))",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(user_id)
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn read_preference(pool: &sqlx::MySqlPool, user_id: &str) -> Result<PreferenceRow, AppError> {
    let row = sqlx::query_as::<_, PreferenceRow>(
        "SELECT id, userId, analysisModel, characterModel, locationModel, storyboardModel, editModel, videoModel, lipSyncModel, videoRatio, artStyle, ttsRate, capabilityDefaults, createdAt, updatedAt FROM user_preferences WHERE userId = ? LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    row.ok_or_else(|| AppError::internal("user preference should exist after ensure step"))
}

pub async fn get_preference(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Value>, AppError> {
    ensure_preference_exists(&state.mysql, &user.id).await?;
    let preference = read_preference(&state.mysql, &user.id).await?;

    Ok(Json(json!({
        "preference": preference,
    })))
}

pub async fn update_preference(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let object = payload
        .as_object()
        .ok_or_else(|| AppError::invalid_params("request body must be an object"))?;

    let allowed_fields = [
        "analysisModel",
        "characterModel",
        "locationModel",
        "storyboardModel",
        "editModel",
        "videoModel",
        "lipSyncModel",
        "videoRatio",
        "artStyle",
        "ttsRate",
    ];

    let mut updates = Vec::<(&str, Option<String>)>::new();

    for field in allowed_fields {
        if let Some(value) = object.get(field) {
            let normalized = match value {
                Value::String(text) => Some(text.trim().to_string()),
                Value::Null => None,
                _ => {
                    return Err(AppError::invalid_params(format!(
                        "{field} must be a string or null"
                    )));
                }
            };
            updates.push((field, normalized));
        }
    }

    if updates.is_empty() {
        return Err(AppError::invalid_params("no allowed fields to update"));
    }

    ensure_preference_exists(&state.mysql, &user.id).await?;

    let mut builder: QueryBuilder<'_, MySql> = QueryBuilder::new("UPDATE user_preferences SET ");
    let mut separated = builder.separated(", ");

    for (field, value) in updates {
        separated
            .push(format!("{field} = "))
            .push_bind_unseparated(value);
    }

    separated.push("updatedAt = NOW(3)");

    builder.push(" WHERE userId = ");
    builder.push_bind(&user.id);

    builder.build().execute(&state.mysql).await?;

    let preference = read_preference(&state.mysql, &user.id).await?;
    Ok(Json(json!({ "preference": preference })))
}
