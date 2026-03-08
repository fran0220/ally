use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::{FromRequest, Multipart, Path, Query, Request, State},
    http::{HeaderMap, header},
    routing::{get, patch, post},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{MySql, QueryBuilder};
use tokio::fs;
use uuid::Uuid;
use waoowaoo_core::image_label::{UpdateImageLabelOptions, update_image_label};

use crate::{
    app_state::AppState, error::AppError, extractors::auth::AuthUser, routes::task_submit,
};

const GLOBAL_ASSET_PROJECT_ID: &str = "global-asset-hub";

#[derive(Debug, Deserialize)]
struct CharacterListQuery {
    #[serde(default, rename = "folderId")]
    folder_id: Option<String>,
    #[serde(default)]
    search: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocationListQuery {
    #[serde(default, rename = "folderId")]
    folder_id: Option<String>,
    #[serde(default)]
    search: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VoiceListQuery {
    #[serde(default, rename = "folderId")]
    folder_id: Option<String>,
    #[serde(default)]
    search: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PickerQuery {
    #[serde(default, rename = "type")]
    picker_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FolderCreateBody {
    name: String,
}

#[derive(Debug, Deserialize)]
struct FolderUpdateBody {
    name: String,
}

#[derive(Debug, Deserialize)]
struct CharacterCreateBody {
    name: String,
    #[serde(default)]
    aliases: Option<Value>,
    #[serde(default, rename = "profileData")]
    profile_data: Option<Value>,
    #[serde(default, rename = "folderId")]
    folder_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CharacterUpdateBody {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    aliases: Option<Value>,
    #[serde(default, rename = "profileData")]
    profile_data: Option<Value>,
    #[serde(default, rename = "folderId")]
    folder_id: Option<String>,
    #[serde(default, rename = "voiceId")]
    voice_id: Option<String>,
    #[serde(default, rename = "voiceType")]
    voice_type: Option<String>,
    #[serde(default, rename = "customVoiceUrl")]
    custom_voice_url: Option<String>,
    #[serde(default, rename = "customVoiceMediaId")]
    custom_voice_media_id: Option<String>,
    #[serde(default, rename = "globalVoiceId")]
    global_voice_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AppearanceBody {
    #[serde(default, rename = "characterId")]
    character_id: Option<String>,
    #[serde(default, rename = "appearanceIndex")]
    appearance_index: Option<i32>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    descriptions: Option<Value>,
    #[serde(default, rename = "imageUrl")]
    image_url: Option<String>,
    #[serde(default, rename = "imageUrls")]
    image_urls: Option<Value>,
    #[serde(default, rename = "selectedIndex")]
    selected_index: Option<i32>,
    #[serde(default, rename = "changeReason")]
    change_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocationCreateBody {
    name: String,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default, rename = "folderId")]
    folder_id: Option<String>,
    #[serde(default, rename = "imageUrl")]
    image_url: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocationUpdateBody {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default, rename = "folderId")]
    folder_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VoiceCreateBody {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default, rename = "folderId")]
    folder_id: Option<String>,
    #[serde(default, rename = "voiceId")]
    voice_id: Option<String>,
    #[serde(default, rename = "voiceType")]
    voice_type: Option<String>,
    #[serde(default, rename = "customVoiceUrl")]
    custom_voice_url: Option<String>,
    #[serde(default, rename = "customVoiceMediaId")]
    custom_voice_media_id: Option<String>,
    #[serde(default, rename = "voicePrompt")]
    voice_prompt: Option<String>,
    #[serde(default)]
    gender: Option<String>,
    #[serde(default)]
    language: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VoiceUpdateBody {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default, rename = "folderId")]
    folder_id: Option<String>,
    #[serde(default, rename = "voiceId")]
    voice_id: Option<String>,
    #[serde(default, rename = "voiceType")]
    voice_type: Option<String>,
    #[serde(default, rename = "customVoiceUrl")]
    custom_voice_url: Option<String>,
    #[serde(default, rename = "customVoiceMediaId")]
    custom_voice_media_id: Option<String>,
    #[serde(default, rename = "voicePrompt")]
    voice_prompt: Option<String>,
    #[serde(default)]
    gender: Option<String>,
    #[serde(default)]
    language: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CharacterVoiceBody {
    #[serde(rename = "characterId")]
    character_id: String,
    #[serde(default, rename = "voiceId")]
    voice_id: Option<String>,
    #[serde(default, rename = "voiceType")]
    voice_type: Option<String>,
    #[serde(default, rename = "customVoiceUrl")]
    custom_voice_url: Option<String>,
    #[serde(default, rename = "customVoiceMediaId")]
    custom_voice_media_id: Option<String>,
    #[serde(default, rename = "globalVoiceId")]
    global_voice_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CharacterVoiceDesignBody {
    #[serde(default, rename = "voiceId")]
    voice_id: Option<String>,
    #[serde(default, rename = "audioBase64")]
    audio_base64: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CharacterVoicePostBody {
    #[serde(default, rename = "characterId")]
    character_id: Option<String>,
    #[serde(default, rename = "voiceDesign")]
    voice_design: Option<CharacterVoiceDesignBody>,
    #[serde(default, rename = "voiceId")]
    voice_id: Option<String>,
    #[serde(default, rename = "voiceType")]
    voice_type: Option<String>,
    #[serde(default, rename = "customVoiceUrl")]
    custom_voice_url: Option<String>,
    #[serde(default, rename = "customVoiceMediaId")]
    custom_voice_media_id: Option<String>,
    #[serde(default, rename = "globalVoiceId")]
    global_voice_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SelectImageBody {
    #[serde(rename = "type")]
    asset_type: String,
    id: String,
    #[serde(default, rename = "imageIndex")]
    image_index: Option<i32>,
    #[serde(default, rename = "appearanceIndex")]
    appearance_index: Option<i32>,
    #[serde(default)]
    confirm: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct UndoImageBody {
    #[serde(rename = "type")]
    asset_type: String,
    id: String,
    #[serde(default, rename = "appearanceIndex")]
    appearance_index: Option<i32>,
    #[serde(default, rename = "imageIndex")]
    image_index: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct UpdateAssetLabelBody {
    #[serde(rename = "type")]
    asset_type: String,
    id: String,
    #[serde(default, rename = "newName")]
    new_name: Option<String>,
    #[serde(default, rename = "appearanceIndex")]
    appearance_index: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct UploadTempBody {
    #[serde(default, rename = "imageBase64")]
    image_base64: Option<String>,
    #[serde(default, rename = "imageUrl")]
    image_url: Option<String>,
    #[serde(default)]
    base64: Option<String>,
    #[serde(default)]
    extension: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UploadImageBody {
    #[serde(rename = "type")]
    asset_type: String,
    id: String,
    #[serde(default, rename = "appearanceIndex")]
    appearance_index: Option<i32>,
    #[serde(default, rename = "imageIndex")]
    image_index: Option<i32>,
    #[serde(rename = "imageUrl")]
    image_url: String,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug)]
struct UploadImageInput {
    asset_type: String,
    id: String,
    appearance_index: Option<i32>,
    image_index: Option<i32>,
    image_url: String,
    label_text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VoiceUploadBody {
    name: String,
    #[serde(default, rename = "folderId")]
    folder_id: Option<String>,
    #[serde(default, rename = "audioUrl")]
    audio_url: Option<String>,
    #[serde(default, rename = "customVoiceUrl")]
    custom_voice_url: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    gender: Option<String>,
    #[serde(default)]
    language: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct FolderRow {
    id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
    name: String,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct CharacterRow {
    id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
    #[sqlx(rename = "folderId")]
    folder_id: Option<String>,
    name: String,
    aliases: Option<String>,
    #[sqlx(rename = "profileData")]
    profile_data: Option<String>,
    #[sqlx(rename = "profileConfirmed")]
    profile_confirmed: bool,
    #[sqlx(rename = "voiceId")]
    voice_id: Option<String>,
    #[sqlx(rename = "voiceType")]
    voice_type: Option<String>,
    #[sqlx(rename = "customVoiceUrl")]
    custom_voice_url: Option<String>,
    #[sqlx(rename = "customVoiceMediaId")]
    custom_voice_media_id: Option<String>,
    #[sqlx(rename = "globalVoiceId")]
    global_voice_id: Option<String>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct AppearanceRow {
    id: String,
    #[sqlx(rename = "characterId")]
    character_id: String,
    #[sqlx(rename = "appearanceIndex")]
    appearance_index: i32,
    #[sqlx(rename = "changeReason")]
    change_reason: String,
    description: Option<String>,
    descriptions: Option<String>,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
    #[sqlx(rename = "imageMediaId")]
    image_media_id: Option<String>,
    #[sqlx(rename = "imageUrls")]
    image_urls: Option<String>,
    #[sqlx(rename = "selectedIndex")]
    selected_index: Option<i32>,
    #[sqlx(rename = "previousImageUrl")]
    previous_image_url: Option<String>,
    #[sqlx(rename = "previousImageMediaId")]
    previous_image_media_id: Option<String>,
    #[sqlx(rename = "previousImageUrls")]
    previous_image_urls: Option<String>,
    #[sqlx(rename = "previousDescription")]
    previous_description: Option<String>,
    #[sqlx(rename = "previousDescriptions")]
    previous_descriptions: Option<String>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct LocationRow {
    id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
    #[sqlx(rename = "folderId")]
    folder_id: Option<String>,
    name: String,
    summary: Option<String>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct LocationImageRow {
    id: String,
    #[sqlx(rename = "locationId")]
    location_id: String,
    #[sqlx(rename = "imageIndex")]
    image_index: i32,
    description: Option<String>,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
    #[sqlx(rename = "imageMediaId")]
    image_media_id: Option<String>,
    #[sqlx(rename = "isSelected")]
    is_selected: bool,
    #[sqlx(rename = "previousImageUrl")]
    previous_image_url: Option<String>,
    #[sqlx(rename = "previousImageMediaId")]
    previous_image_media_id: Option<String>,
    #[sqlx(rename = "previousDescription")]
    previous_description: Option<String>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct VoiceRow {
    id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
    #[sqlx(rename = "folderId")]
    folder_id: Option<String>,
    name: String,
    description: Option<String>,
    #[sqlx(rename = "voiceId")]
    voice_id: Option<String>,
    #[sqlx(rename = "voiceType")]
    voice_type: String,
    #[sqlx(rename = "customVoiceUrl")]
    custom_voice_url: Option<String>,
    #[sqlx(rename = "customVoiceMediaId")]
    custom_voice_media_id: Option<String>,
    #[sqlx(rename = "voicePrompt")]
    voice_prompt: Option<String>,
    gender: Option<String>,
    language: String,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, sqlx::FromRow)]
struct UserOwnedRow {
    #[sqlx(rename = "userId")]
    user_id: String,
}

#[derive(Debug, sqlx::FromRow)]
struct CharacterAppearanceSelectionRow {
    id: String,
    #[sqlx(rename = "imageUrls")]
    image_urls: Option<String>,
    #[sqlx(rename = "selectedIndex")]
    selected_index: Option<i32>,
}

#[derive(Debug, sqlx::FromRow)]
struct CharacterAppearanceUndoRow {
    id: String,
    description: Option<String>,
    descriptions: Option<String>,
    #[sqlx(rename = "previousImageUrl")]
    previous_image_url: Option<String>,
    #[sqlx(rename = "previousImageUrls")]
    previous_image_urls: Option<String>,
    #[sqlx(rename = "previousDescription")]
    previous_description: Option<String>,
    #[sqlx(rename = "previousDescriptions")]
    previous_descriptions: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct LocationUndoRow {
    id: String,
    #[sqlx(rename = "previousImageUrl")]
    previous_image_url: Option<String>,
    description: Option<String>,
    #[sqlx(rename = "previousDescription")]
    previous_description: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct CharacterLabelRow {
    id: String,
    #[sqlx(rename = "appearanceIndex")]
    appearance_index: i32,
    #[sqlx(rename = "changeReason")]
    change_reason: Option<String>,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
    #[sqlx(rename = "imageUrls")]
    image_urls: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct LocationLabelRow {
    id: String,
    #[sqlx(rename = "imageIndex")]
    image_index: i32,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct CharacterUploadRow {
    id: String,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
    #[sqlx(rename = "imageUrls")]
    image_urls: Option<String>,
    #[sqlx(rename = "selectedIndex")]
    selected_index: Option<i32>,
}

#[derive(Debug, sqlx::FromRow)]
struct LocationUploadRow {
    id: String,
    #[sqlx(rename = "imageIndex")]
    image_index: i32,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn normalize_optional_json(value: Option<Value>) -> Option<String> {
    value.and_then(|item| serde_json::to_string(&item).ok())
}

fn parse_json_text(value: Option<&str>) -> Option<Value> {
    value
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .filter(|parsed| !parsed.is_null())
}

fn decode_image_urls(raw: Option<&str>) -> Vec<String> {
    raw.and_then(|value| serde_json::from_str::<Value>(value).ok())
        .and_then(|value| value.as_array().cloned())
        .map(|items| {
            items
                .into_iter()
                .filter_map(|item| {
                    item.as_str()
                        .map(|text| text.trim().to_string())
                        .filter(|text| !text.is_empty())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn encode_image_urls(urls: &[String]) -> Option<String> {
    serde_json::to_string(urls).ok()
}

fn build_restored_image_urls(
    previous_image_url: Option<&str>,
    previous_image_urls_raw: Option<&str>,
) -> Vec<String> {
    let urls = decode_image_urls(previous_image_urls_raw);
    if !urls.is_empty() {
        return urls;
    }

    previous_image_url
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(|value| vec![value])
        .unwrap_or_default()
}

fn normalize_file_extension(raw: Option<&str>) -> Option<String> {
    let normalized = raw?.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }

    if !normalized
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }

    if normalized == "jpeg" {
        return Some("jpg".to_string());
    }

    Some(normalized)
}

fn image_mime_type_by_extension(ext: &str) -> &'static str {
    match ext.trim().to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "image/jpeg",
    }
}

fn localized_msg<'a>(locale: &str, zh: &'a str, en: &'a str) -> &'a str {
    if locale == "en" { en } else { zh }
}

fn build_character_image_label(
    character_name: &str,
    change_reason: Option<&str>,
    locale: &str,
) -> String {
    let reason = change_reason
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(localized_msg(locale, "形象", "Appearance"));
    format!("{} - {}", character_name.trim(), reason)
}

fn parse_optional_i32_text(raw: Option<String>, field_name: &str) -> Result<Option<i32>, AppError> {
    let Some(value) = raw else {
        return Ok(None);
    };

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    trimmed
        .parse::<i32>()
        .map(Some)
        .map_err(|_| AppError::invalid_params(format!("{field_name} must be an integer")))
}

async fn store_uploaded_blob(bytes: &[u8], ext: &str) -> Result<String, AppError> {
    let upload_root = upload_dir();
    let asset_dir = format!("{upload_root}/asset-hub");
    fs::create_dir_all(&asset_dir)
        .await
        .map_err(|err| AppError::internal(format!("failed to create asset upload dir: {err}")))?;

    let key = format!("asset-hub/{}.{}", Uuid::new_v4(), ext);
    let path = format!("{upload_root}/{key}");
    fs::write(&path, bytes)
        .await
        .map_err(|err| AppError::internal(format!("failed to write uploaded blob: {err}")))?;

    Ok(key)
}

fn row_to_character_json(row: &CharacterRow, appearances: Vec<AppearanceRow>) -> Value {
    let appearances = appearances
        .into_iter()
        .map(|appearance| {
            json!({
              "id": appearance.id,
              "characterId": appearance.character_id,
              "appearanceIndex": appearance.appearance_index,
              "changeReason": appearance.change_reason,
              "description": appearance.description,
              "descriptions": parse_json_text(appearance.descriptions.as_deref()),
              "imageUrl": appearance.image_url,
              "imageMediaId": appearance.image_media_id,
              "imageUrls": decode_image_urls(appearance.image_urls.as_deref()),
              "selectedIndex": appearance.selected_index,
              "previousImageUrl": appearance.previous_image_url,
              "previousImageMediaId": appearance.previous_image_media_id,
              "previousImageUrls": decode_image_urls(appearance.previous_image_urls.as_deref()),
              "previousDescription": appearance.previous_description,
              "previousDescriptions": parse_json_text(appearance.previous_descriptions.as_deref()),
              "createdAt": appearance.created_at,
              "updatedAt": appearance.updated_at,
            })
        })
        .collect::<Vec<_>>();

    json!({
      "id": row.id,
      "userId": row.user_id,
      "folderId": row.folder_id,
      "name": row.name,
      "aliases": parse_json_text(row.aliases.as_deref()),
      "profileData": parse_json_text(row.profile_data.as_deref()),
      "profileConfirmed": row.profile_confirmed,
      "voiceId": row.voice_id,
      "voiceType": row.voice_type,
      "customVoiceUrl": row.custom_voice_url,
      "customVoiceMediaId": row.custom_voice_media_id,
      "globalVoiceId": row.global_voice_id,
      "appearances": appearances,
      "createdAt": row.created_at,
      "updatedAt": row.updated_at,
    })
}

fn row_to_location_json(row: &LocationRow, images: Vec<LocationImageRow>) -> Value {
    json!({
      "id": row.id,
      "userId": row.user_id,
      "folderId": row.folder_id,
      "name": row.name,
      "summary": row.summary,
      "images": images,
      "createdAt": row.created_at,
      "updatedAt": row.updated_at,
    })
}

fn upload_dir() -> String {
    std::env::var("UPLOAD_DIR")
        .ok()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .unwrap_or_else(|| "./data/uploads".to_string())
}

async fn ensure_folder_belongs_to_user(
    state: &AppState,
    folder_id: &str,
    user_id: &str,
) -> Result<(), AppError> {
    let owner = sqlx::query_as::<_, UserOwnedRow>(
        "SELECT userId FROM global_asset_folders WHERE id = ? LIMIT 1",
    )
    .bind(folder_id)
    .fetch_optional(&state.mysql)
    .await?;

    let Some(owner) = owner else {
        return Err(AppError::not_found("folder not found"));
    };
    if owner.user_id != user_id {
        return Err(AppError::forbidden("folder access denied"));
    }
    Ok(())
}

async fn ensure_character_belongs_to_user(
    state: &AppState,
    character_id: &str,
    user_id: &str,
) -> Result<(), AppError> {
    let owner = sqlx::query_as::<_, UserOwnedRow>(
        "SELECT userId FROM global_characters WHERE id = ? LIMIT 1",
    )
    .bind(character_id)
    .fetch_optional(&state.mysql)
    .await?;

    let Some(owner) = owner else {
        return Err(AppError::not_found("character not found"));
    };
    if owner.user_id != user_id {
        return Err(AppError::forbidden("character access denied"));
    }
    Ok(())
}

async fn ensure_location_belongs_to_user(
    state: &AppState,
    location_id: &str,
    user_id: &str,
) -> Result<(), AppError> {
    let owner = sqlx::query_as::<_, UserOwnedRow>(
        "SELECT userId FROM global_locations WHERE id = ? LIMIT 1",
    )
    .bind(location_id)
    .fetch_optional(&state.mysql)
    .await?;

    let Some(owner) = owner else {
        return Err(AppError::not_found("location not found"));
    };
    if owner.user_id != user_id {
        return Err(AppError::forbidden("location access denied"));
    }
    Ok(())
}

async fn ensure_voice_belongs_to_user(
    state: &AppState,
    voice_id: &str,
    user_id: &str,
) -> Result<(), AppError> {
    let owner =
        sqlx::query_as::<_, UserOwnedRow>("SELECT userId FROM global_voices WHERE id = ? LIMIT 1")
            .bind(voice_id)
            .fetch_optional(&state.mysql)
            .await?;

    let Some(owner) = owner else {
        return Err(AppError::not_found("voice not found"));
    };
    if owner.user_id != user_id {
        return Err(AppError::forbidden("voice access denied"));
    }
    Ok(())
}

async fn load_appearances_map(
    state: &AppState,
    character_ids: &[String],
) -> Result<HashMap<String, Vec<AppearanceRow>>, AppError> {
    if character_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
        "SELECT id, characterId, appearanceIndex, changeReason, description, descriptions, imageUrl, imageMediaId, imageUrls, selectedIndex, previousImageUrl, previousImageMediaId, previousImageUrls, previousDescription, previousDescriptions, createdAt, updatedAt FROM global_character_appearances WHERE characterId IN (",
    );
    let mut separated = qb.separated(",");
    for id in character_ids {
        separated.push_bind(id);
    }
    separated.push_unseparated(") ORDER BY appearanceIndex ASC, createdAt ASC");

    let rows = qb
        .build_query_as::<AppearanceRow>()
        .fetch_all(&state.mysql)
        .await?;

    let mut map = HashMap::<String, Vec<AppearanceRow>>::new();
    for row in rows {
        map.entry(row.character_id.clone()).or_default().push(row);
    }
    Ok(map)
}

async fn load_location_images_map(
    state: &AppState,
    location_ids: &[String],
) -> Result<HashMap<String, Vec<LocationImageRow>>, AppError> {
    if location_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
        "SELECT id, locationId, imageIndex, description, imageUrl, imageMediaId, isSelected, previousImageUrl, previousImageMediaId, previousDescription, createdAt, updatedAt FROM global_location_images WHERE locationId IN (",
    );
    let mut separated = qb.separated(",");
    for id in location_ids {
        separated.push_bind(id);
    }
    separated.push_unseparated(") ORDER BY imageIndex ASC, createdAt ASC");

    let rows = qb
        .build_query_as::<LocationImageRow>()
        .fetch_all(&state.mysql)
        .await?;

    let mut map = HashMap::<String, Vec<LocationImageRow>>::new();
    for row in rows {
        map.entry(row.location_id.clone()).or_default().push(row);
    }
    Ok(map)
}

async fn list_folders(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query_as::<_, FolderRow>(
        "SELECT id, userId, name, createdAt, updatedAt FROM global_asset_folders WHERE userId = ? ORDER BY createdAt ASC",
    )
    .bind(&user.id)
    .fetch_all(&state.mysql)
    .await?;

    Ok(Json(json!({ "folders": rows })))
}

async fn create_folder(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<FolderCreateBody>,
) -> Result<Json<Value>, AppError> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::invalid_params("folder name is required"));
    }

    let folder_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO global_asset_folders (id, userId, name, createdAt, updatedAt) VALUES (?, ?, ?, NOW(3), NOW(3))",
    )
    .bind(&folder_id)
    .bind(&user.id)
    .bind(name)
    .execute(&state.mysql)
    .await?;

    let row = sqlx::query_as::<_, FolderRow>(
        "SELECT id, userId, name, createdAt, updatedAt FROM global_asset_folders WHERE id = ? LIMIT 1",
    )
    .bind(&folder_id)
    .fetch_one(&state.mysql)
    .await?;

    Ok(Json(json!({ "success": true, "folder": row })))
}

async fn update_folder(
    State(state): State<AppState>,
    user: AuthUser,
    Path(folder_id): Path<String>,
    Json(body): Json<FolderUpdateBody>,
) -> Result<Json<Value>, AppError> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::invalid_params("folder name is required"));
    }

    ensure_folder_belongs_to_user(&state, &folder_id, &user.id).await?;

    sqlx::query("UPDATE global_asset_folders SET name = ?, updatedAt = NOW(3) WHERE id = ?")
        .bind(name)
        .bind(&folder_id)
        .execute(&state.mysql)
        .await?;

    let row = sqlx::query_as::<_, FolderRow>(
        "SELECT id, userId, name, createdAt, updatedAt FROM global_asset_folders WHERE id = ? LIMIT 1",
    )
    .bind(&folder_id)
    .fetch_one(&state.mysql)
    .await?;

    Ok(Json(json!({ "success": true, "folder": row })))
}

async fn delete_folder(
    State(state): State<AppState>,
    user: AuthUser,
    Path(folder_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    ensure_folder_belongs_to_user(&state, &folder_id, &user.id).await?;

    let mut tx = state.mysql.begin().await?;
    sqlx::query(
        "UPDATE global_characters SET folderId = NULL, updatedAt = NOW(3) WHERE folderId = ?",
    )
    .bind(&folder_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE global_locations SET folderId = NULL, updatedAt = NOW(3) WHERE folderId = ?",
    )
    .bind(&folder_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE global_voices SET folderId = NULL, updatedAt = NOW(3) WHERE folderId = ?")
        .bind(&folder_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM global_asset_folders WHERE id = ?")
        .bind(&folder_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Json(json!({ "success": true })))
}

async fn list_characters(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<CharacterListQuery>,
) -> Result<Json<Value>, AppError> {
    let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
        "SELECT id, userId, folderId, name, aliases, profileData, profileConfirmed, voiceId, voiceType, customVoiceUrl, customVoiceMediaId, globalVoiceId, createdAt, updatedAt FROM global_characters WHERE userId = ",
    );
    qb.push_bind(&user.id);

    if let Some(folder_id) = query.folder_id {
        let folder_id = folder_id.trim().to_string();
        if !folder_id.is_empty() {
            qb.push(" AND folderId = ");
            qb.push_bind(folder_id);
        }
    }

    if let Some(search) = query.search {
        let search = search.trim().to_string();
        if !search.is_empty() {
            qb.push(" AND name LIKE ");
            qb.push_bind(format!("%{search}%"));
        }
    }

    qb.push(" ORDER BY createdAt ASC");

    let characters = qb
        .build_query_as::<CharacterRow>()
        .fetch_all(&state.mysql)
        .await?;

    let ids = characters
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    let appearances_map = load_appearances_map(&state, &ids).await?;

    let payload = characters
        .iter()
        .map(|item| {
            row_to_character_json(
                item,
                appearances_map.get(&item.id).cloned().unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>();

    Ok(Json(json!({ "characters": payload })))
}

async fn create_character(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CharacterCreateBody>,
) -> Result<Json<Value>, AppError> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::invalid_params("character name is required"));
    }

    if let Some(folder_id) = body.folder_id.as_deref() {
        let folder_id = folder_id.trim();
        if !folder_id.is_empty() {
            ensure_folder_belongs_to_user(&state, folder_id, &user.id).await?;
        }
    }

    let character_id = Uuid::new_v4().to_string();
    let appearance_id = Uuid::new_v4().to_string();

    let mut tx = state.mysql.begin().await?;
    sqlx::query(
        "INSERT INTO global_characters (id, userId, folderId, name, aliases, profileData, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
    )
    .bind(&character_id)
    .bind(&user.id)
    .bind(normalize_optional_string(body.folder_id))
    .bind(name)
    .bind(normalize_optional_json(body.aliases))
    .bind(normalize_optional_json(body.profile_data))
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO global_character_appearances (id, characterId, appearanceIndex, changeReason, createdAt, updatedAt) VALUES (?, ?, 0, 'default', NOW(3), NOW(3))",
    )
    .bind(&appearance_id)
    .bind(&character_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let character = get_character(State(state), user, Path(character_id))
        .await?
        .0
        .get("character")
        .cloned()
        .unwrap_or(Value::Null);
    Ok(Json(json!({ "success": true, "character": character })))
}

async fn get_character(
    State(state): State<AppState>,
    user: AuthUser,
    Path(character_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    ensure_character_belongs_to_user(&state, &character_id, &user.id).await?;

    let row = sqlx::query_as::<_, CharacterRow>(
        "SELECT id, userId, folderId, name, aliases, profileData, profileConfirmed, voiceId, voiceType, customVoiceUrl, customVoiceMediaId, globalVoiceId, createdAt, updatedAt FROM global_characters WHERE id = ? LIMIT 1",
    )
    .bind(&character_id)
    .fetch_one(&state.mysql)
    .await?;

    let appearances = sqlx::query_as::<_, AppearanceRow>(
        "SELECT id, characterId, appearanceIndex, changeReason, description, descriptions, imageUrl, imageMediaId, imageUrls, selectedIndex, previousImageUrl, previousImageMediaId, previousImageUrls, previousDescription, previousDescriptions, createdAt, updatedAt FROM global_character_appearances WHERE characterId = ? ORDER BY appearanceIndex ASC",
    )
    .bind(&character_id)
    .fetch_all(&state.mysql)
    .await?;

    Ok(Json(json!({
      "character": row_to_character_json(&row, appearances)
    })))
}

async fn update_character(
    State(state): State<AppState>,
    user: AuthUser,
    Path(character_id): Path<String>,
    Json(body): Json<CharacterUpdateBody>,
) -> Result<Json<Value>, AppError> {
    ensure_character_belongs_to_user(&state, &character_id, &user.id).await?;

    if let Some(folder_id) = body.folder_id.as_deref() {
        let folder_id = folder_id.trim();
        if !folder_id.is_empty() {
            ensure_folder_belongs_to_user(&state, folder_id, &user.id).await?;
        }
    }

    let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new("UPDATE global_characters SET ");
    let mut separated = qb.separated(", ");
    let mut touched = false;

    if let Some(name) = body.name {
        touched = true;
        separated
            .push("name = ")
            .push_bind_unseparated(name.trim().to_string());
    }
    if let Some(aliases) = body.aliases {
        touched = true;
        separated
            .push("aliases = ")
            .push_bind_unseparated(normalize_optional_json(Some(aliases)));
    }
    if let Some(profile_data) = body.profile_data {
        touched = true;
        separated
            .push("profileData = ")
            .push_bind_unseparated(normalize_optional_json(Some(profile_data)));
    }
    if body.folder_id.is_some() {
        touched = true;
        separated
            .push("folderId = ")
            .push_bind_unseparated(normalize_optional_string(body.folder_id));
    }
    if body.voice_id.is_some() {
        touched = true;
        separated
            .push("voiceId = ")
            .push_bind_unseparated(normalize_optional_string(body.voice_id));
    }
    if body.voice_type.is_some() {
        touched = true;
        separated
            .push("voiceType = ")
            .push_bind_unseparated(normalize_optional_string(body.voice_type));
    }
    if body.custom_voice_url.is_some() {
        touched = true;
        separated
            .push("customVoiceUrl = ")
            .push_bind_unseparated(normalize_optional_string(body.custom_voice_url));
    }
    if body.custom_voice_media_id.is_some() {
        touched = true;
        separated
            .push("customVoiceMediaId = ")
            .push_bind_unseparated(normalize_optional_string(body.custom_voice_media_id));
    }
    if body.global_voice_id.is_some() {
        touched = true;
        separated
            .push("globalVoiceId = ")
            .push_bind_unseparated(normalize_optional_string(body.global_voice_id));
    }

    if !touched {
        return Err(AppError::invalid_params("empty update payload"));
    }

    separated.push("updatedAt = NOW(3)");
    qb.push(" WHERE id = ");
    qb.push_bind(&character_id);
    qb.build().execute(&state.mysql).await?;

    let character = get_character(State(state), user, Path(character_id))
        .await?
        .0
        .get("character")
        .cloned()
        .unwrap_or(Value::Null);
    Ok(Json(json!({ "success": true, "character": character })))
}

async fn delete_character(
    State(state): State<AppState>,
    user: AuthUser,
    Path(character_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    ensure_character_belongs_to_user(&state, &character_id, &user.id).await?;
    sqlx::query("DELETE FROM global_characters WHERE id = ?")
        .bind(&character_id)
        .execute(&state.mysql)
        .await?;
    Ok(Json(json!({ "success": true })))
}

async fn upsert_character_appearance(
    State(state): State<AppState>,
    user: AuthUser,
    Path((character_id, appearance_index)): Path<(String, i32)>,
    Json(body): Json<AppearanceBody>,
) -> Result<Json<Value>, AppError> {
    ensure_character_belongs_to_user(&state, &character_id, &user.id).await?;
    if appearance_index < 0 {
        return Err(AppError::invalid_params("appearanceIndex must be >= 0"));
    }

    let exists: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM global_character_appearances WHERE characterId = ? AND appearanceIndex = ? LIMIT 1",
    )
    .bind(&character_id)
    .bind(appearance_index)
    .fetch_optional(&state.mysql)
    .await?;

    if exists.is_some() {
        return Err(AppError::conflict("appearance already exists"));
    }

    let appearance_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO global_character_appearances (id, characterId, appearanceIndex, changeReason, description, descriptions, imageUrl, imageUrls, selectedIndex, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
    )
    .bind(&appearance_id)
    .bind(&character_id)
    .bind(appearance_index)
    .bind(body.change_reason.unwrap_or_else(|| "manual".to_string()))
    .bind(normalize_optional_string(body.description))
    .bind(normalize_optional_json(body.descriptions))
    .bind(normalize_optional_string(body.image_url))
    .bind(normalize_optional_json(body.image_urls))
    .bind(body.selected_index)
    .execute(&state.mysql)
    .await?;

    Ok(Json(json!({ "success": true, "appearance": {
      "id": appearance_id,
      "characterId": character_id,
      "appearanceIndex": appearance_index
    }})))
}

async fn patch_character_appearance(
    State(state): State<AppState>,
    user: AuthUser,
    Path((character_id, appearance_index)): Path<(String, i32)>,
    Json(body): Json<AppearanceBody>,
) -> Result<Json<Value>, AppError> {
    ensure_character_belongs_to_user(&state, &character_id, &user.id).await?;

    let appearance: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM global_character_appearances WHERE characterId = ? AND appearanceIndex = ? LIMIT 1",
    )
    .bind(&character_id)
    .bind(appearance_index)
    .fetch_optional(&state.mysql)
    .await?;

    let Some((appearance_id,)) = appearance else {
        return Err(AppError::not_found("appearance not found"));
    };

    let mut qb: QueryBuilder<'_, MySql> =
        QueryBuilder::new("UPDATE global_character_appearances SET ");
    let mut separated = qb.separated(", ");
    let mut touched = false;

    if body.description.is_some() {
        touched = true;
        separated
            .push("description = ")
            .push_bind_unseparated(normalize_optional_string(body.description));
    }
    if body.descriptions.is_some() {
        touched = true;
        separated
            .push("descriptions = ")
            .push_bind_unseparated(normalize_optional_json(body.descriptions));
    }
    if body.image_url.is_some() {
        touched = true;
        separated.push("previousImageUrl = imageUrl, imageUrl = ");
        separated.push_bind_unseparated(normalize_optional_string(body.image_url));
    }
    if body.image_urls.is_some() {
        touched = true;
        separated.push("previousImageUrls = imageUrls, imageUrls = ");
        separated.push_bind_unseparated(normalize_optional_json(body.image_urls));
    }
    if body.selected_index.is_some() {
        touched = true;
        separated
            .push("selectedIndex = ")
            .push_bind_unseparated(body.selected_index);
    }
    if body.change_reason.is_some() {
        touched = true;
        separated
            .push("changeReason = ")
            .push_bind_unseparated(normalize_optional_string(body.change_reason));
    }

    if !touched {
        return Err(AppError::invalid_params("empty appearance update payload"));
    }

    separated.push("updatedAt = NOW(3)");
    qb.push(" WHERE id = ");
    qb.push_bind(&appearance_id);
    qb.build().execute(&state.mysql).await?;

    Ok(Json(json!({ "success": true })))
}

async fn delete_character_appearance(
    State(state): State<AppState>,
    user: AuthUser,
    Path((character_id, appearance_index)): Path<(String, i32)>,
) -> Result<Json<Value>, AppError> {
    ensure_character_belongs_to_user(&state, &character_id, &user.id).await?;

    sqlx::query(
        "DELETE FROM global_character_appearances WHERE characterId = ? AND appearanceIndex = ?",
    )
    .bind(&character_id)
    .bind(appearance_index)
    .execute(&state.mysql)
    .await?;

    Ok(Json(json!({ "success": true })))
}

async fn create_appearance(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<AppearanceBody>,
) -> Result<Json<Value>, AppError> {
    let character_id = body
        .character_id
        .clone()
        .ok_or_else(|| AppError::invalid_params("characterId is required"))?;

    let appearance_index = body.appearance_index.unwrap_or(0);

    upsert_character_appearance(
        State(state),
        user,
        Path((character_id, appearance_index)),
        Json(body),
    )
    .await
}

async fn update_appearance(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<AppearanceBody>,
) -> Result<Json<Value>, AppError> {
    let character_id = body
        .character_id
        .clone()
        .ok_or_else(|| AppError::invalid_params("characterId is required"))?;

    let appearance_index = body
        .appearance_index
        .ok_or_else(|| AppError::invalid_params("appearanceIndex is required"))?;

    patch_character_appearance(
        State(state),
        user,
        Path((character_id, appearance_index)),
        Json(body),
    )
    .await
}

async fn remove_appearance(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<AppearanceBody>,
) -> Result<Json<Value>, AppError> {
    let character_id = body
        .character_id
        .clone()
        .ok_or_else(|| AppError::invalid_params("characterId is required"))?;

    let appearance_index = body
        .appearance_index
        .ok_or_else(|| AppError::invalid_params("appearanceIndex is required"))?;

    delete_character_appearance(State(state), user, Path((character_id, appearance_index))).await
}

async fn list_locations(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<LocationListQuery>,
) -> Result<Json<Value>, AppError> {
    let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
        "SELECT id, userId, folderId, name, summary, createdAt, updatedAt FROM global_locations WHERE userId = ",
    );
    qb.push_bind(&user.id);

    if let Some(folder_id) = query.folder_id {
        let folder_id = folder_id.trim().to_string();
        if !folder_id.is_empty() {
            qb.push(" AND folderId = ");
            qb.push_bind(folder_id);
        }
    }

    if let Some(search) = query.search {
        let search = search.trim().to_string();
        if !search.is_empty() {
            qb.push(" AND name LIKE ");
            qb.push_bind(format!("%{search}%"));
        }
    }

    qb.push(" ORDER BY createdAt ASC");

    let locations = qb
        .build_query_as::<LocationRow>()
        .fetch_all(&state.mysql)
        .await?;

    let ids = locations
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    let images_map = load_location_images_map(&state, &ids).await?;

    let payload = locations
        .iter()
        .map(|item| {
            row_to_location_json(item, images_map.get(&item.id).cloned().unwrap_or_default())
        })
        .collect::<Vec<_>>();

    Ok(Json(json!({ "locations": payload })))
}

async fn create_location(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<LocationCreateBody>,
) -> Result<Json<Value>, AppError> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::invalid_params("location name is required"));
    }

    if let Some(folder_id) = body.folder_id.as_deref() {
        let folder_id = folder_id.trim();
        if !folder_id.is_empty() {
            ensure_folder_belongs_to_user(&state, folder_id, &user.id).await?;
        }
    }

    let location_id = Uuid::new_v4().to_string();
    let image_id = Uuid::new_v4().to_string();

    let mut tx = state.mysql.begin().await?;
    sqlx::query(
        "INSERT INTO global_locations (id, userId, folderId, name, summary, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, NOW(3), NOW(3))",
    )
    .bind(&location_id)
    .bind(&user.id)
    .bind(normalize_optional_string(body.folder_id))
    .bind(name)
    .bind(normalize_optional_string(body.summary))
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO global_location_images (id, locationId, imageIndex, description, imageUrl, isSelected, createdAt, updatedAt) VALUES (?, ?, 0, ?, ?, true, NOW(3), NOW(3))",
    )
    .bind(&image_id)
    .bind(&location_id)
    .bind(normalize_optional_string(body.description))
    .bind(normalize_optional_string(body.image_url))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let location = get_location(State(state), user, Path(location_id))
        .await?
        .0
        .get("location")
        .cloned()
        .unwrap_or(Value::Null);
    Ok(Json(json!({ "success": true, "location": location })))
}

async fn get_location(
    State(state): State<AppState>,
    user: AuthUser,
    Path(location_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    ensure_location_belongs_to_user(&state, &location_id, &user.id).await?;

    let row = sqlx::query_as::<_, LocationRow>(
        "SELECT id, userId, folderId, name, summary, createdAt, updatedAt FROM global_locations WHERE id = ? LIMIT 1",
    )
    .bind(&location_id)
    .fetch_one(&state.mysql)
    .await?;

    let images = sqlx::query_as::<_, LocationImageRow>(
        "SELECT id, locationId, imageIndex, description, imageUrl, imageMediaId, isSelected, previousImageUrl, previousImageMediaId, previousDescription, createdAt, updatedAt FROM global_location_images WHERE locationId = ? ORDER BY imageIndex ASC",
    )
    .bind(&location_id)
    .fetch_all(&state.mysql)
    .await?;

    Ok(Json(json!({
      "location": row_to_location_json(&row, images)
    })))
}

async fn update_location(
    State(state): State<AppState>,
    user: AuthUser,
    Path(location_id): Path<String>,
    Json(body): Json<LocationUpdateBody>,
) -> Result<Json<Value>, AppError> {
    ensure_location_belongs_to_user(&state, &location_id, &user.id).await?;

    if let Some(folder_id) = body.folder_id.as_deref() {
        let folder_id = folder_id.trim();
        if !folder_id.is_empty() {
            ensure_folder_belongs_to_user(&state, folder_id, &user.id).await?;
        }
    }

    let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new("UPDATE global_locations SET ");
    let mut separated = qb.separated(", ");
    let mut touched = false;

    if body.name.is_some() {
        touched = true;
        separated
            .push("name = ")
            .push_bind_unseparated(normalize_optional_string(body.name));
    }
    if body.summary.is_some() {
        touched = true;
        separated
            .push("summary = ")
            .push_bind_unseparated(normalize_optional_string(body.summary));
    }
    if body.folder_id.is_some() {
        touched = true;
        separated
            .push("folderId = ")
            .push_bind_unseparated(normalize_optional_string(body.folder_id));
    }

    if !touched {
        return Err(AppError::invalid_params("empty update payload"));
    }

    separated.push("updatedAt = NOW(3)");
    qb.push(" WHERE id = ");
    qb.push_bind(&location_id);
    qb.build().execute(&state.mysql).await?;

    let location = get_location(State(state), user, Path(location_id))
        .await?
        .0
        .get("location")
        .cloned()
        .unwrap_or(Value::Null);
    Ok(Json(json!({ "success": true, "location": location })))
}

async fn delete_location(
    State(state): State<AppState>,
    user: AuthUser,
    Path(location_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    ensure_location_belongs_to_user(&state, &location_id, &user.id).await?;
    sqlx::query("DELETE FROM global_locations WHERE id = ?")
        .bind(&location_id)
        .execute(&state.mysql)
        .await?;

    Ok(Json(json!({ "success": true })))
}

async fn list_voices(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<VoiceListQuery>,
) -> Result<Json<Value>, AppError> {
    let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
        "SELECT id, userId, folderId, name, description, voiceId, voiceType, customVoiceUrl, customVoiceMediaId, voicePrompt, gender, language, createdAt, updatedAt FROM global_voices WHERE userId = ",
    );
    qb.push_bind(&user.id);

    if let Some(folder_id) = query.folder_id {
        let folder_id = folder_id.trim().to_string();
        if !folder_id.is_empty() {
            qb.push(" AND folderId = ");
            qb.push_bind(folder_id);
        }
    }

    if let Some(search) = query.search {
        let search = search.trim().to_string();
        if !search.is_empty() {
            qb.push(" AND name LIKE ");
            qb.push_bind(format!("%{search}%"));
        }
    }

    qb.push(" ORDER BY createdAt ASC");

    let rows = qb
        .build_query_as::<VoiceRow>()
        .fetch_all(&state.mysql)
        .await?;

    Ok(Json(json!({ "voices": rows })))
}

async fn create_voice(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<VoiceCreateBody>,
) -> Result<Json<Value>, AppError> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::invalid_params("voice name is required"));
    }

    if let Some(folder_id) = body.folder_id.as_deref() {
        let folder_id = folder_id.trim();
        if !folder_id.is_empty() {
            ensure_folder_belongs_to_user(&state, folder_id, &user.id).await?;
        }
    }

    let voice_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO global_voices (id, userId, folderId, name, description, voiceId, voiceType, customVoiceUrl, customVoiceMediaId, voicePrompt, gender, language, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
    )
    .bind(&voice_id)
    .bind(&user.id)
    .bind(normalize_optional_string(body.folder_id))
    .bind(name)
    .bind(normalize_optional_string(body.description))
    .bind(normalize_optional_string(body.voice_id))
    .bind(normalize_optional_string(body.voice_type).unwrap_or_else(|| "qwen-designed".to_string()))
    .bind(normalize_optional_string(body.custom_voice_url))
    .bind(normalize_optional_string(body.custom_voice_media_id))
    .bind(normalize_optional_string(body.voice_prompt))
    .bind(normalize_optional_string(body.gender))
    .bind(normalize_optional_string(body.language).unwrap_or_else(|| "zh".to_string()))
    .execute(&state.mysql)
    .await?;

    let row = sqlx::query_as::<_, VoiceRow>(
        "SELECT id, userId, folderId, name, description, voiceId, voiceType, customVoiceUrl, customVoiceMediaId, voicePrompt, gender, language, createdAt, updatedAt FROM global_voices WHERE id = ? LIMIT 1",
    )
    .bind(&voice_id)
    .fetch_one(&state.mysql)
    .await?;

    Ok(Json(json!({ "success": true, "voice": row })))
}

async fn update_voice(
    State(state): State<AppState>,
    user: AuthUser,
    Path(voice_id): Path<String>,
    Json(body): Json<VoiceUpdateBody>,
) -> Result<Json<Value>, AppError> {
    ensure_voice_belongs_to_user(&state, &voice_id, &user.id).await?;

    if let Some(folder_id) = body.folder_id.as_deref() {
        let folder_id = folder_id.trim();
        if !folder_id.is_empty() {
            ensure_folder_belongs_to_user(&state, folder_id, &user.id).await?;
        }
    }

    let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new("UPDATE global_voices SET ");
    let mut separated = qb.separated(", ");
    let mut touched = false;

    if body.name.is_some() {
        touched = true;
        separated
            .push("name = ")
            .push_bind_unseparated(normalize_optional_string(body.name));
    }
    if body.description.is_some() {
        touched = true;
        separated
            .push("description = ")
            .push_bind_unseparated(normalize_optional_string(body.description));
    }
    if body.folder_id.is_some() {
        touched = true;
        separated
            .push("folderId = ")
            .push_bind_unseparated(normalize_optional_string(body.folder_id));
    }
    if body.voice_id.is_some() {
        touched = true;
        separated
            .push("voiceId = ")
            .push_bind_unseparated(normalize_optional_string(body.voice_id));
    }
    if body.voice_type.is_some() {
        touched = true;
        separated
            .push("voiceType = ")
            .push_bind_unseparated(normalize_optional_string(body.voice_type));
    }
    if body.custom_voice_url.is_some() {
        touched = true;
        separated
            .push("customVoiceUrl = ")
            .push_bind_unseparated(normalize_optional_string(body.custom_voice_url));
    }
    if body.custom_voice_media_id.is_some() {
        touched = true;
        separated
            .push("customVoiceMediaId = ")
            .push_bind_unseparated(normalize_optional_string(body.custom_voice_media_id));
    }
    if body.voice_prompt.is_some() {
        touched = true;
        separated
            .push("voicePrompt = ")
            .push_bind_unseparated(normalize_optional_string(body.voice_prompt));
    }
    if body.gender.is_some() {
        touched = true;
        separated
            .push("gender = ")
            .push_bind_unseparated(normalize_optional_string(body.gender));
    }
    if body.language.is_some() {
        touched = true;
        separated
            .push("language = ")
            .push_bind_unseparated(normalize_optional_string(body.language));
    }

    if !touched {
        return Err(AppError::invalid_params("empty update payload"));
    }

    separated.push("updatedAt = NOW(3)");
    qb.push(" WHERE id = ");
    qb.push_bind(&voice_id);
    qb.build().execute(&state.mysql).await?;

    let row = sqlx::query_as::<_, VoiceRow>(
        "SELECT id, userId, folderId, name, description, voiceId, voiceType, customVoiceUrl, customVoiceMediaId, voicePrompt, gender, language, createdAt, updatedAt FROM global_voices WHERE id = ? LIMIT 1",
    )
    .bind(&voice_id)
    .fetch_one(&state.mysql)
    .await?;

    Ok(Json(json!({ "success": true, "voice": row })))
}

async fn delete_voice(
    State(state): State<AppState>,
    user: AuthUser,
    Path(voice_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    ensure_voice_belongs_to_user(&state, &voice_id, &user.id).await?;
    sqlx::query("DELETE FROM global_voices WHERE id = ?")
        .bind(&voice_id)
        .execute(&state.mysql)
        .await?;

    Ok(Json(json!({ "success": true })))
}

async fn upload_voice(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<VoiceUploadBody>,
) -> Result<Json<Value>, AppError> {
    create_voice(
        State(state),
        user,
        Json(VoiceCreateBody {
            name: body.name,
            folder_id: body.folder_id,
            description: body.description,
            voice_id: None,
            voice_type: Some("custom".to_string()),
            custom_voice_url: normalize_optional_string(body.custom_voice_url)
                .or_else(|| normalize_optional_string(body.audio_url)),
            custom_voice_media_id: None,
            voice_prompt: None,
            gender: body.gender,
            language: body.language,
        }),
    )
    .await
}

async fn apply_character_voice_update(
    state: &AppState,
    character_id: &str,
    voice_id: Option<String>,
    voice_type: Option<String>,
    custom_voice_url: Option<String>,
    custom_voice_media_id: Option<String>,
    global_voice_id: Option<String>,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE global_characters SET voiceId = ?, voiceType = ?, customVoiceUrl = ?, customVoiceMediaId = ?, globalVoiceId = ?, updatedAt = NOW(3) WHERE id = ?",
    )
    .bind(voice_id)
    .bind(voice_type)
    .bind(custom_voice_url)
    .bind(custom_voice_media_id)
    .bind(global_voice_id)
    .bind(character_id)
    .execute(&state.mysql)
    .await?;

    Ok(())
}

async fn post_character_voice(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CharacterVoicePostBody>,
) -> Result<Json<Value>, AppError> {
    let character_id = normalize_optional_string(body.character_id)
        .ok_or_else(|| AppError::invalid_params("characterId is required"))?;
    ensure_character_belongs_to_user(&state, &character_id, &user.id).await?;

    if let Some(voice_design) = body.voice_design {
        let designed_voice_id = normalize_optional_string(voice_design.voice_id)
            .ok_or_else(|| AppError::invalid_params("voiceDesign.voiceId is required"))?;
        let audio_base64 = normalize_optional_string(voice_design.audio_base64)
            .ok_or_else(|| AppError::invalid_params("voiceDesign.audioBase64 is required"))?;
        let audio_buffer = STANDARD
            .decode(audio_base64)
            .map_err(|_| AppError::invalid_params("voiceDesign.audioBase64 is invalid"))?;
        let stored_audio_key = store_uploaded_blob(&audio_buffer, "wav").await?;

        apply_character_voice_update(
            &state,
            &character_id,
            Some(designed_voice_id),
            Some("custom".to_string()),
            Some(stored_audio_key.clone()),
            None,
            None,
        )
        .await?;

        return Ok(Json(json!({
          "success": true,
          "audioUrl": stored_audio_key,
        })));
    }

    let audio_url = normalize_optional_string(body.custom_voice_url);
    apply_character_voice_update(
        &state,
        &character_id,
        normalize_optional_string(body.voice_id),
        normalize_optional_string(body.voice_type),
        audio_url.clone(),
        normalize_optional_string(body.custom_voice_media_id),
        normalize_optional_string(body.global_voice_id),
    )
    .await?;

    Ok(Json(json!({
      "success": true,
      "audioUrl": audio_url,
    })))
}

async fn patch_character_voice(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CharacterVoiceBody>,
) -> Result<Json<Value>, AppError> {
    ensure_character_belongs_to_user(&state, &body.character_id, &user.id).await?;
    apply_character_voice_update(
        &state,
        &body.character_id,
        normalize_optional_string(body.voice_id),
        normalize_optional_string(body.voice_type),
        normalize_optional_string(body.custom_voice_url),
        normalize_optional_string(body.custom_voice_media_id),
        normalize_optional_string(body.global_voice_id),
    )
    .await?;

    Ok(Json(json!({ "success": true })))
}

async fn picker(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<PickerQuery>,
) -> Result<Json<Value>, AppError> {
    let picker_type = query
        .picker_type
        .map(|item| item.trim().to_string())
        .unwrap_or_default();

    match picker_type.as_str() {
        "character" => {
            list_characters(
                State(state),
                user,
                Query(CharacterListQuery {
                    folder_id: None,
                    search: None,
                }),
            )
            .await
        }
        "location" => {
            list_locations(
                State(state),
                user,
                Query(LocationListQuery {
                    folder_id: None,
                    search: None,
                }),
            )
            .await
        }
        "voice" => {
            list_voices(
                State(state),
                user,
                Query(VoiceListQuery {
                    folder_id: None,
                    search: None,
                }),
            )
            .await
        }
        _ => {
            let chars = list_characters(
                State(state.clone()),
                user.clone(),
                Query(CharacterListQuery {
                    folder_id: None,
                    search: None,
                }),
            )
            .await?
            .0;
            let locations = list_locations(
                State(state.clone()),
                user.clone(),
                Query(LocationListQuery {
                    folder_id: None,
                    search: None,
                }),
            )
            .await?
            .0;
            let voices = list_voices(
                State(state),
                user,
                Query(VoiceListQuery {
                    folder_id: None,
                    search: None,
                }),
            )
            .await?
            .0;

            Ok(Json(json!({
              "characters": chars.get("characters").cloned().unwrap_or_else(|| json!([])),
              "locations": locations.get("locations").cloned().unwrap_or_else(|| json!([])),
              "voices": voices.get("voices").cloned().unwrap_or_else(|| json!([])),
            })))
        }
    }
}

async fn select_image(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<SelectImageBody>,
) -> Result<Json<Value>, AppError> {
    let asset_type = body.asset_type.trim().to_lowercase();

    if asset_type == "character" {
        ensure_character_belongs_to_user(&state, &body.id, &user.id).await?;
        let appearance_index = body.appearance_index.unwrap_or(0);

        let appearance = sqlx::query_as::<_, CharacterAppearanceSelectionRow>(
            "SELECT id, imageUrls, selectedIndex FROM global_character_appearances WHERE characterId = ? AND appearanceIndex = ? LIMIT 1",
        )
        .bind(&body.id)
        .bind(appearance_index)
        .fetch_optional(&state.mysql)
        .await?;

        let Some(appearance) = appearance else {
            return Err(AppError::not_found("appearance not found"));
        };

        if body.confirm.unwrap_or(false) {
            if let Some(selected_index) = appearance.selected_index {
                let image_urls = decode_image_urls(appearance.image_urls.as_deref());
                let selected_url = usize::try_from(selected_index)
                    .ok()
                    .and_then(|idx| image_urls.get(idx))
                    .cloned();

                if let Some(selected_url) = selected_url {
                    let selected_only_urls = vec![selected_url.clone()];
                    let encoded = encode_image_urls(&selected_only_urls).ok_or_else(|| {
                        AppError::internal("failed to encode selected image urls".to_string())
                    })?;
                    sqlx::query(
                        "UPDATE global_character_appearances SET imageUrl = ?, imageUrls = ?, selectedIndex = 0, updatedAt = NOW(3) WHERE id = ?",
                    )
                    .bind(selected_url)
                    .bind(encoded)
                    .bind(&appearance.id)
                    .execute(&state.mysql)
                    .await?;
                }
            }
        } else {
            sqlx::query(
                "UPDATE global_character_appearances SET selectedIndex = ?, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(body.image_index)
            .bind(&appearance.id)
            .execute(&state.mysql)
            .await?;
        }

        return Ok(Json(json!({ "success": true })));
    }

    if asset_type == "location" {
        ensure_location_belongs_to_user(&state, &body.id, &user.id).await?;
        let mut tx = state.mysql.begin().await?;
        sqlx::query("UPDATE global_location_images SET isSelected = false, updatedAt = NOW(3) WHERE locationId = ?")
            .bind(&body.id)
            .execute(&mut *tx)
            .await?;
        if body.image_index.is_some() {
            sqlx::query(
                "UPDATE global_location_images SET isSelected = true, updatedAt = NOW(3) WHERE locationId = ? AND imageIndex = ?",
            )
            .bind(&body.id)
            .bind(body.image_index)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        return Ok(Json(json!({ "success": true })));
    }

    Err(AppError::invalid_params(
        "unsupported type for select-image",
    ))
}

async fn undo_image(
    State(state): State<AppState>,
    user: AuthUser,
    headers: HeaderMap,
    Json(body): Json<UndoImageBody>,
) -> Result<Json<Value>, AppError> {
    let locale = read_task_locale_from_headers(&headers).unwrap_or("zh");
    let asset_type = body.asset_type.trim().to_lowercase();

    if asset_type == "character" {
        ensure_character_belongs_to_user(&state, &body.id, &user.id).await?;
        let appearance_index = body.appearance_index.unwrap_or(0);

        let appearance = sqlx::query_as::<_, CharacterAppearanceUndoRow>(
            "SELECT id, description, descriptions, previousImageUrl, previousImageUrls, previousDescription, previousDescriptions FROM global_character_appearances WHERE characterId = ? AND appearanceIndex = ? LIMIT 1",
        )
        .bind(&body.id)
        .bind(appearance_index)
        .fetch_optional(&state.mysql)
        .await?;

        let Some(appearance) = appearance else {
            return Err(AppError::not_found("appearance not found"));
        };

        let restored_image_urls = build_restored_image_urls(
            appearance.previous_image_url.as_deref(),
            appearance.previous_image_urls.as_deref(),
        );
        if restored_image_urls.is_empty() {
            return Err(AppError::invalid_params("no previous image to restore"));
        }

        let restored_image_url = appearance
            .previous_image_url
            .clone()
            .or_else(|| restored_image_urls.first().cloned());
        let restored_descriptions = appearance
            .previous_descriptions
            .clone()
            .or(appearance.descriptions.clone());
        let restored_description = appearance
            .previous_description
            .clone()
            .or(appearance.description.clone());

        let encoded_restored = encode_image_urls(&restored_image_urls).ok_or_else(|| {
            AppError::internal("failed to encode restored image urls".to_string())
        })?;
        let encoded_empty = encode_image_urls(&[])
            .ok_or_else(|| AppError::internal("failed to encode empty image urls".to_string()))?;

        sqlx::query(
            "UPDATE global_character_appearances SET imageUrl = ?, imageUrls = ?, previousImageUrl = NULL, previousImageUrls = ?, selectedIndex = NULL, descriptions = ?, previousDescriptions = NULL, description = ?, previousDescription = NULL, updatedAt = NOW(3) WHERE id = ?",
        )
        .bind(restored_image_url)
        .bind(encoded_restored)
        .bind(encoded_empty)
        .bind(restored_descriptions)
        .bind(restored_description)
        .bind(appearance.id)
        .execute(&state.mysql)
        .await?;

        return Ok(Json(json!({
            "success": true,
            "message": localized_msg(
                locale,
                "已撤回到上一版本（图片和描述词）",
                "Reverted to the previous version (image and prompt)"
            )
        })));
    }

    if asset_type == "location" {
        ensure_location_belongs_to_user(&state, &body.id, &user.id).await?;
        let _ = body.image_index;

        let rows = sqlx::query_as::<_, LocationUndoRow>(
            "SELECT id, previousImageUrl, description, previousDescription FROM global_location_images WHERE locationId = ?",
        )
        .bind(&body.id)
        .fetch_all(&state.mysql)
        .await?;

        let mut tx = state.mysql.begin().await?;
        for row in rows {
            let Some(previous_image_url) =
                normalize_optional_string(row.previous_image_url.clone())
            else {
                continue;
            };

            let restored_description = row.previous_description.or(row.description);
            sqlx::query(
                "UPDATE global_location_images SET imageUrl = ?, previousImageUrl = NULL, description = ?, previousDescription = NULL, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(previous_image_url)
            .bind(restored_description)
            .bind(row.id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        return Ok(Json(json!({
            "success": true,
            "message": localized_msg(
                locale,
                "已撤回到上一版本（图片和描述词）",
                "Reverted to the previous version (image and prompt)"
            )
        })));
    }

    Err(AppError::invalid_params("unsupported type for undo-image"))
}

async fn update_asset_label(
    State(state): State<AppState>,
    user: AuthUser,
    headers: HeaderMap,
    Json(body): Json<UpdateAssetLabelBody>,
) -> Result<Json<Value>, AppError> {
    let locale = read_task_locale_from_headers(&headers).unwrap_or("zh");
    let new_name = normalize_optional_string(body.new_name)
        .ok_or_else(|| AppError::invalid_params("newName is required"))?;

    let asset_type = body.asset_type.trim().to_lowercase();

    if asset_type == "character" {
        ensure_character_belongs_to_user(&state, &body.id, &user.id).await?;

        let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
            "SELECT id, appearanceIndex, changeReason, imageUrl, imageUrls FROM global_character_appearances WHERE characterId = ",
        );
        qb.push_bind(&body.id);
        if let Some(appearance_index) = body.appearance_index {
            qb.push(" AND appearanceIndex = ");
            qb.push_bind(appearance_index);
        }
        qb.push(" ORDER BY appearanceIndex ASC");

        let rows = qb
            .build_query_as::<CharacterLabelRow>()
            .fetch_all(&state.mysql)
            .await?;

        let mut results = Vec::<Value>::new();
        for row in rows {
            let mut image_urls = decode_image_urls(row.image_urls.as_deref());
            if image_urls.is_empty()
                && let Some(image_url) = normalize_optional_string(row.image_url.clone())
            {
                image_urls.push(image_url);
            }
            if image_urls.is_empty() {
                continue;
            }

            let label_text =
                build_character_image_label(&new_name, row.change_reason.as_deref(), locale);
            let mut updated_urls = Vec::with_capacity(image_urls.len());
            for image_url in image_urls {
                let updated = update_image_label(
                    &image_url,
                    &label_text,
                    Some(UpdateImageLabelOptions::with_new_key("labeled-rename")),
                )
                .await?;
                updated_urls.push(updated);
            }

            let first_url = updated_urls.iter().find(|value| !value.is_empty()).cloned();
            let encoded = encode_image_urls(&updated_urls).ok_or_else(|| {
                AppError::internal("failed to encode appearance image urls".to_string())
            })?;
            sqlx::query(
                "UPDATE global_character_appearances SET imageUrls = ?, imageUrl = ?, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(encoded)
            .bind(first_url)
            .bind(&row.id)
            .execute(&state.mysql)
            .await?;

            results.push(json!({
              "appearanceIndex": row.appearance_index,
              "imageUrls": updated_urls,
            }));
        }

        return Ok(Json(json!({ "success": true, "results": results })));
    }

    if asset_type == "location" {
        ensure_location_belongs_to_user(&state, &body.id, &user.id).await?;

        let rows = sqlx::query_as::<_, LocationLabelRow>(
            "SELECT id, imageIndex, imageUrl FROM global_location_images WHERE locationId = ? ORDER BY imageIndex ASC",
        )
        .bind(&body.id)
        .fetch_all(&state.mysql)
        .await?;

        let mut results = Vec::<Value>::new();
        for row in rows {
            let Some(image_url) = normalize_optional_string(row.image_url.clone()) else {
                continue;
            };

            let updated_image_url = update_image_label(
                &image_url,
                &new_name,
                Some(UpdateImageLabelOptions::with_new_key("labeled-rename")),
            )
            .await?;

            sqlx::query(
                "UPDATE global_location_images SET imageUrl = ?, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(&updated_image_url)
            .bind(&row.id)
            .execute(&state.mysql)
            .await?;

            results.push(json!({
              "imageIndex": row.image_index,
              "imageUrl": updated_image_url,
            }));
        }

        return Ok(Json(json!({ "success": true, "results": results })));
    }

    Err(AppError::invalid_params(
        "unsupported type for update-asset-label",
    ))
}

async fn upload_temp(
    _state: State<AppState>,
    _user: AuthUser,
    Json(body): Json<UploadTempBody>,
) -> Result<Json<Value>, AppError> {
    if let Some(image_url) = normalize_optional_string(body.image_url) {
        return Ok(Json(json!({
          "success": true,
          "key": image_url,
          "url": image_url,
        })));
    }

    let (bytes, ext) = if let Some(image_base64) = normalize_optional_string(body.image_base64) {
        let parts: Vec<&str> = image_base64.splitn(2, ',').collect();
        if parts.len() != 2 {
            return Err(AppError::invalid_params("invalid imageBase64 payload"));
        }

        let header = parts[0];
        let data = parts[1];
        let ext = if header.contains("image/png") {
            "png".to_string()
        } else if header.contains("image/jpeg") || header.contains("image/jpg") {
            "jpg".to_string()
        } else if header.contains("image/webp") {
            "webp".to_string()
        } else {
            return Err(AppError::invalid_params("unsupported data url image type"));
        };

        let bytes = STANDARD
            .decode(data)
            .map_err(|_| AppError::invalid_params("invalid base64 image data"))?;
        (bytes, ext)
    } else if let (Some(base64_payload), Some(extension_raw)) = (
        normalize_optional_string(body.base64),
        normalize_optional_string(body.extension),
    ) {
        let ext = normalize_file_extension(Some(extension_raw.as_str()))
            .ok_or_else(|| AppError::invalid_params("invalid extension"))?;
        let bytes = STANDARD
            .decode(base64_payload)
            .map_err(|_| AppError::invalid_params("invalid base64 data"))?;
        (bytes, ext)
    } else {
        return Err(AppError::invalid_params(
            "imageBase64 or {base64, extension} is required",
        ));
    };

    let dir = format!("{}/temp", upload_dir());
    fs::create_dir_all(&dir)
        .await
        .map_err(|err| AppError::internal(format!("failed to create temp upload dir: {err}")))?;

    let key = format!("temp/{}.{}", Uuid::new_v4(), ext);
    let path = format!("{}/{}", upload_dir(), key);
    fs::write(&path, bytes)
        .await
        .map_err(|err| AppError::internal(format!("failed to write temp image: {err}")))?;

    Ok(Json(json!({
      "success": true,
      "key": key,
      "url": key,
    })))
}

async fn parse_upload_image_multipart(
    mut multipart: Multipart,
) -> Result<UploadImageInput, AppError> {
    let mut asset_type: Option<String> = None;
    let mut id: Option<String> = None;
    let mut appearance_index_raw: Option<String> = None;
    let mut image_index_raw: Option<String> = None;
    let mut label_text: Option<String> = None;
    let mut image_url: Option<String> = None;
    let mut file_ext: Option<String> = None;
    let mut file_bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::invalid_params("invalid multipart payload"))?
    {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "type" => {
                asset_type = normalize_optional_string(Some(
                    field
                        .text()
                        .await
                        .map_err(|_| AppError::invalid_params("invalid type field"))?,
                ));
            }
            "id" => {
                id = normalize_optional_string(Some(
                    field
                        .text()
                        .await
                        .map_err(|_| AppError::invalid_params("invalid id field"))?,
                ));
            }
            "appearanceIndex" => {
                appearance_index_raw = normalize_optional_string(Some(
                    field
                        .text()
                        .await
                        .map_err(|_| AppError::invalid_params("invalid appearanceIndex field"))?,
                ));
            }
            "imageIndex" => {
                image_index_raw = normalize_optional_string(Some(
                    field
                        .text()
                        .await
                        .map_err(|_| AppError::invalid_params("invalid imageIndex field"))?,
                ));
            }
            "labelText" => {
                label_text = normalize_optional_string(Some(
                    field
                        .text()
                        .await
                        .map_err(|_| AppError::invalid_params("invalid labelText field"))?,
                ));
            }
            "imageUrl" => {
                image_url = normalize_optional_string(Some(
                    field
                        .text()
                        .await
                        .map_err(|_| AppError::invalid_params("invalid imageUrl field"))?,
                ));
            }
            "file" => {
                file_ext = field
                    .file_name()
                    .and_then(|name| name.rsplit('.').next())
                    .and_then(|ext| normalize_file_extension(Some(ext)));
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|_| AppError::invalid_params("invalid file field"))?;
                if !bytes.is_empty() {
                    file_bytes = Some(bytes.to_vec());
                }
            }
            _ => {}
        }
    }

    let asset_type = asset_type.ok_or_else(|| AppError::invalid_params("type is required"))?;
    let id = id.ok_or_else(|| AppError::invalid_params("id is required"))?;
    let label_text = label_text.ok_or_else(|| AppError::invalid_params("labelText is required"))?;
    let appearance_index = parse_optional_i32_text(appearance_index_raw, "appearanceIndex")?;
    let image_index = parse_optional_i32_text(image_index_raw, "imageIndex")?;

    let image_url = if let Some(url) = image_url {
        url
    } else {
        let bytes = file_bytes.ok_or_else(|| AppError::invalid_params("file is required"))?;
        let ext = file_ext.unwrap_or_else(|| "jpg".to_string());
        let mime_type = image_mime_type_by_extension(&ext);
        format!("data:{mime_type};base64,{}", STANDARD.encode(bytes))
    };

    Ok(UploadImageInput {
        asset_type,
        id,
        appearance_index,
        image_index,
        image_url,
        label_text: Some(label_text),
    })
}

async fn parse_upload_image_input(
    headers: &HeaderMap,
    state: &AppState,
    request: Request,
) -> Result<UploadImageInput, AppError> {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if content_type.starts_with("multipart/form-data") {
        let multipart = Multipart::from_request(request, state)
            .await
            .map_err(|_| AppError::invalid_params("invalid multipart payload"))?;
        return parse_upload_image_multipart(multipart).await;
    }

    let Json(body) = Json::<UploadImageBody>::from_request(request, state)
        .await
        .map_err(|_| AppError::invalid_params("invalid upload-image payload"))?;
    let image_url = body.image_url.trim().to_string();
    if image_url.is_empty() {
        return Err(AppError::invalid_params("imageUrl is required"));
    }

    Ok(UploadImageInput {
        asset_type: body.asset_type,
        id: body.id,
        appearance_index: body.appearance_index,
        image_index: body.image_index,
        image_url,
        label_text: normalize_optional_string(body.description),
    })
}

async fn upload_image(
    State(state): State<AppState>,
    user: AuthUser,
    headers: HeaderMap,
    request: Request,
) -> Result<Json<Value>, AppError> {
    let input = parse_upload_image_input(&headers, &state, request).await?;
    let asset_type = input.asset_type.trim().to_lowercase();
    let image_url = if let Some(label_text) = input.label_text.as_deref() {
        let key_prefix = match asset_type.as_str() {
            "character" => "asset-hub-character",
            "location" => "asset-hub-location",
            _ => "asset-hub-image",
        };
        update_image_label(
            &input.image_url,
            label_text,
            Some(UpdateImageLabelOptions::with_new_key(key_prefix)),
        )
        .await?
    } else {
        input.image_url.clone()
    };

    if asset_type == "character" {
        ensure_character_belongs_to_user(&state, &input.id, &user.id).await?;
        let appearance_index = input.appearance_index.unwrap_or(0);

        let appearance = sqlx::query_as::<_, CharacterUploadRow>(
            "SELECT id, imageUrl, imageUrls, selectedIndex FROM global_character_appearances WHERE characterId = ? AND appearanceIndex = ? LIMIT 1",
        )
        .bind(&input.id)
        .bind(appearance_index)
        .fetch_optional(&state.mysql)
        .await?;

        let Some(appearance) = appearance else {
            return Err(AppError::not_found("appearance not found"));
        };

        let mut image_urls = decode_image_urls(appearance.image_urls.as_deref());
        if image_urls.is_empty()
            && let Some(current_image) = normalize_optional_string(appearance.image_url.clone())
        {
            image_urls.push(current_image);
        }

        if appearance.image_url.is_some() || !image_urls.is_empty() {
            let previous_image_urls = appearance
                .image_urls
                .clone()
                .or_else(|| encode_image_urls(&image_urls));
            sqlx::query(
                "UPDATE global_character_appearances SET previousImageUrl = ?, previousImageUrls = ?, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(appearance.image_url.clone())
            .bind(previous_image_urls)
            .bind(&appearance.id)
            .execute(&state.mysql)
            .await?;
        }

        let target_index = if let Some(index) = input.image_index {
            index
        } else {
            i32::try_from(image_urls.len()).map_err(|_| {
                AppError::internal("too many character image candidates".to_string())
            })?
        };
        if target_index < 0 {
            return Err(AppError::invalid_params("imageIndex must be >= 0"));
        }
        let target_index_usize = usize::try_from(target_index)
            .map_err(|_| AppError::invalid_params("imageIndex must be >= 0"))?;

        while image_urls.len() <= target_index_usize {
            image_urls.push(String::new());
        }
        image_urls[target_index_usize] = image_url.clone();

        let should_update_image_url = appearance.selected_index == Some(target_index)
            || (appearance.selected_index.is_none() && target_index == 0)
            || image_urls.iter().filter(|value| !value.is_empty()).count() == 1;

        let encoded = encode_image_urls(&image_urls)
            .ok_or_else(|| AppError::internal("failed to encode image urls".to_string()))?;
        if should_update_image_url {
            sqlx::query(
                "UPDATE global_character_appearances SET imageUrls = ?, imageUrl = ?, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(encoded)
            .bind(&image_url)
            .bind(&appearance.id)
            .execute(&state.mysql)
            .await?;
        } else {
            sqlx::query(
                "UPDATE global_character_appearances SET imageUrls = ?, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(encoded)
            .bind(&appearance.id)
            .execute(&state.mysql)
            .await?;
        }

        return Ok(Json(
            json!({ "success": true, "imageKey": image_url, "imageIndex": target_index }),
        ));
    }

    if asset_type == "location" {
        ensure_location_belongs_to_user(&state, &input.id, &user.id).await?;

        let images = sqlx::query_as::<_, LocationUploadRow>(
            "SELECT id, imageIndex, imageUrl FROM global_location_images WHERE locationId = ? ORDER BY imageIndex ASC",
        )
        .bind(&input.id)
        .fetch_all(&state.mysql)
        .await?;

        let image_index = if let Some(index) = input.image_index {
            index
        } else {
            i32::try_from(images.len())
                .map_err(|_| AppError::internal("too many location images".to_string()))?
        };
        if image_index < 0 {
            return Err(AppError::invalid_params("imageIndex must be >= 0"));
        }

        if let Some(existing) = images.iter().find(|item| item.image_index == image_index) {
            if let Some(previous_image_url) = normalize_optional_string(existing.image_url.clone())
            {
                sqlx::query(
                    "UPDATE global_location_images SET previousImageUrl = ?, updatedAt = NOW(3) WHERE id = ?",
                )
                .bind(previous_image_url)
                .bind(&existing.id)
                .execute(&state.mysql)
                .await?;
            }

            sqlx::query(
                "UPDATE global_location_images SET imageUrl = ?, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(&image_url)
            .bind(&existing.id)
            .execute(&state.mysql)
            .await?;
        } else {
            let is_selected = image_index == 0;
            sqlx::query(
                "INSERT INTO global_location_images (id, locationId, imageIndex, description, imageUrl, isSelected, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&input.id)
            .bind(image_index)
            .bind(input.label_text.clone())
            .bind(&image_url)
            .bind(is_selected)
            .execute(&state.mysql)
            .await?;
        }

        return Ok(Json(
            json!({ "success": true, "imageKey": image_url, "imageIndex": image_index }),
        ));
    }

    Err(AppError::invalid_params(
        "unsupported type for upload-image",
    ))
}

fn read_target_info(
    body: &Value,
    fallback_target_type: &str,
    fallback_target_id: &str,
) -> (String, String) {
    let target_type = body
        .get("targetType")
        .and_then(Value::as_str)
        .or_else(|| body.get("type").and_then(Value::as_str))
        .unwrap_or(fallback_target_type)
        .trim()
        .to_string();
    let target_id = body
        .get("targetId")
        .and_then(Value::as_str)
        .or_else(|| body.get("id").and_then(Value::as_str))
        .or_else(|| body.get("characterId").and_then(Value::as_str))
        .or_else(|| body.get("locationId").and_then(Value::as_str))
        .or_else(|| body.get("voiceId").and_then(Value::as_str))
        .unwrap_or(fallback_target_id)
        .trim()
        .to_string();

    (
        if target_type.is_empty() {
            fallback_target_type.to_string()
        } else {
            target_type
        },
        if target_id.is_empty() {
            fallback_target_id.to_string()
        } else {
            target_id
        },
    )
}

fn body_string(body: &Value, key: &str) -> Option<String> {
    body.get(key)
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_locale_candidate(raw: &str) -> Option<&'static str> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }

    if normalized == "zh" || normalized.starts_with("zh-") {
        return Some("zh");
    }

    if normalized == "en" || normalized.starts_with("en-") {
        return Some("en");
    }

    None
}

fn read_task_locale_from_body(body: &Value) -> Option<&'static str> {
    let from_meta = body
        .get("meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get("locale"))
        .and_then(Value::as_str)
        .and_then(normalize_locale_candidate);
    if from_meta.is_some() {
        return from_meta;
    }

    body.get("locale")
        .and_then(Value::as_str)
        .and_then(normalize_locale_candidate)
}

fn read_task_locale_from_headers(headers: &HeaderMap) -> Option<&'static str> {
    headers
        .get("accept-language")
        .and_then(|raw| raw.to_str().ok())
        .and_then(|raw| raw.split(',').next())
        .and_then(normalize_locale_candidate)
}

fn require_task_locale(body: &Value, headers: &HeaderMap) -> Result<(), AppError> {
    if read_task_locale_from_body(body)
        .or_else(|| read_task_locale_from_headers(headers))
        .is_none()
    {
        return Err(AppError::invalid_params("meta.locale is required"));
    }

    Ok(())
}

fn validate_generate_image_payload(body: &Value) -> Result<(), AppError> {
    let asset_type =
        body_string(body, "type").ok_or_else(|| AppError::invalid_params("type is required"))?;
    if asset_type != "character" && asset_type != "location" {
        return Err(AppError::invalid_params(
            "type must be character or location",
        ));
    }

    if body_string(body, "id").is_none() {
        return Err(AppError::invalid_params("id is required"));
    }

    Ok(())
}

fn validate_ai_design_payload(body: &Value) -> Result<(), AppError> {
    if body_string(body, "userInstruction").is_none() {
        return Err(AppError::invalid_params("userInstruction is required"));
    }

    Ok(())
}

fn validate_voice_design_payload(body: &Value) -> Result<(), AppError> {
    let voice_prompt = body_string(body, "voicePrompt")
        .ok_or_else(|| AppError::invalid_params("voicePrompt is required"))?;
    if voice_prompt.chars().count() > 500 {
        return Err(AppError::invalid_params(
            "voicePrompt cannot exceed 500 characters",
        ));
    }

    let preview_text = body_string(body, "previewText")
        .ok_or_else(|| AppError::invalid_params("previewText is required"))?;
    let preview_len = preview_text.chars().count();
    if preview_len < 5 {
        return Err(AppError::invalid_params(
            "previewText must be at least 5 characters",
        ));
    }
    if preview_len > 200 {
        return Err(AppError::invalid_params(
            "previewText cannot exceed 200 characters",
        ));
    }

    Ok(())
}

async fn submit_asset_task(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<Value>,
    task_type: &'static str,
    fallback_target_type: &'static str,
    fallback_target_id: &'static str,
    accept_language: Option<&str>,
) -> Result<Json<Value>, AppError> {
    let (target_type, target_id) =
        read_target_info(&body, fallback_target_type, fallback_target_id);

    task_submit::submit_task(
        &state,
        &user,
        task_submit::SubmitTaskArgs {
            project_id: GLOBAL_ASSET_PROJECT_ID,
            episode_id: None,
            task_type,
            target_type: &target_type,
            target_id: &target_id,
            priority: None,
            max_attempts: None,
            accept_language,
            payload: body,
        },
    )
    .await
}

async fn generate_image(
    state: State<AppState>,
    user: AuthUser,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    require_task_locale(&body, &headers)?;
    validate_generate_image_payload(&body)?;
    let accept_language = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|raw| raw.to_str().ok());

    submit_asset_task(
        state,
        user,
        Json(body),
        "asset_hub_image",
        "character",
        "asset-hub-image",
        accept_language,
    )
    .await
}

async fn modify_image(
    state: State<AppState>,
    user: AuthUser,
    body: Json<Value>,
) -> Result<Json<Value>, AppError> {
    submit_asset_task(
        state,
        user,
        body,
        "asset_hub_modify",
        "character",
        "asset-hub-modify",
        None,
    )
    .await
}

async fn voice_design(
    state: State<AppState>,
    user: AuthUser,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    require_task_locale(&body, &headers)?;
    validate_voice_design_payload(&body)?;
    let accept_language = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|raw| raw.to_str().ok());

    submit_asset_task(
        state,
        user,
        Json(body),
        "asset_hub_voice_design",
        "voice",
        "asset-hub-voice-design",
        accept_language,
    )
    .await
}

async fn ai_design_character(
    state: State<AppState>,
    user: AuthUser,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    validate_ai_design_payload(&body)?;

    submit_asset_task(
        state,
        user,
        Json(body),
        "asset_hub_ai_design_character",
        "character",
        "new-character",
        None,
    )
    .await
}

async fn ai_design_location(
    state: State<AppState>,
    user: AuthUser,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    validate_ai_design_payload(&body)?;

    submit_asset_task(
        state,
        user,
        Json(body),
        "asset_hub_ai_design_location",
        "location",
        "new-location",
        None,
    )
    .await
}

async fn ai_modify_character(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let character_id = body
        .get("characterId")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::invalid_params("characterId is required"))?
        .trim()
        .to_string();
    if character_id.is_empty() {
        return Err(AppError::invalid_params("characterId is required"));
    }
    ensure_character_belongs_to_user(&state, &character_id, &user.id).await?;

    task_submit::submit_task(
        &state,
        &user,
        task_submit::SubmitTaskArgs {
            project_id: GLOBAL_ASSET_PROJECT_ID,
            episode_id: None,
            task_type: "asset_hub_ai_modify_character",
            target_type: "character",
            target_id: &character_id,
            priority: None,
            max_attempts: None,
            accept_language: None,
            payload: body,
        },
    )
    .await
}

async fn ai_modify_location(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let location_id = body
        .get("locationId")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::invalid_params("locationId is required"))?
        .trim()
        .to_string();
    if location_id.is_empty() {
        return Err(AppError::invalid_params("locationId is required"));
    }
    ensure_location_belongs_to_user(&state, &location_id, &user.id).await?;

    task_submit::submit_task(
        &state,
        &user,
        task_submit::SubmitTaskArgs {
            project_id: GLOBAL_ASSET_PROJECT_ID,
            episode_id: None,
            task_type: "asset_hub_ai_modify_location",
            target_type: "location",
            target_id: &location_id,
            priority: None,
            max_attempts: None,
            accept_language: None,
            payload: body,
        },
    )
    .await
}

async fn reference_to_character(
    state: State<AppState>,
    user: AuthUser,
    body: Json<Value>,
) -> Result<Json<Value>, AppError> {
    submit_asset_task(
        state,
        user,
        body,
        "asset_hub_reference_to_character",
        "character",
        "reference-to-character",
        None,
    )
    .await
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/asset-hub/folders",
            get(list_folders).post(create_folder),
        )
        .route(
            "/api/asset-hub/folders/{folderId}",
            patch(update_folder).delete(delete_folder),
        )
        .route(
            "/api/asset-hub/characters",
            get(list_characters).post(create_character),
        )
        .route(
            "/api/asset-hub/characters/{characterId}",
            get(get_character)
                .patch(update_character)
                .delete(delete_character),
        )
        .route(
            "/api/asset-hub/characters/{characterId}/appearances/{appearanceIndex}",
            patch(patch_character_appearance)
                .post(upsert_character_appearance)
                .delete(delete_character_appearance),
        )
        .route(
            "/api/asset-hub/appearances",
            post(create_appearance)
                .patch(update_appearance)
                .delete(remove_appearance),
        )
        .route(
            "/api/asset-hub/locations",
            get(list_locations).post(create_location),
        )
        .route(
            "/api/asset-hub/locations/{locationId}",
            get(get_location)
                .patch(update_location)
                .delete(delete_location),
        )
        .route("/api/asset-hub/voices", get(list_voices).post(create_voice))
        .route(
            "/api/asset-hub/voices/{id}",
            patch(update_voice).delete(delete_voice),
        )
        .route("/api/asset-hub/voices/upload", post(upload_voice))
        .route(
            "/api/asset-hub/character-voice",
            post(post_character_voice).patch(patch_character_voice),
        )
        .route("/api/asset-hub/picker", get(picker))
        .route("/api/asset-hub/select-image", post(select_image))
        .route("/api/asset-hub/undo-image", post(undo_image))
        .route(
            "/api/asset-hub/update-asset-label",
            post(update_asset_label),
        )
        .route("/api/asset-hub/upload-temp", post(upload_temp))
        .route("/api/asset-hub/upload-image", post(upload_image))
        .route("/api/asset-hub/generate-image", post(generate_image))
        .route("/api/asset-hub/modify-image", post(modify_image))
        .route("/api/asset-hub/voice-design", post(voice_design))
        .route(
            "/api/asset-hub/ai-design-character",
            post(ai_design_character),
        )
        .route(
            "/api/asset-hub/ai-design-location",
            post(ai_design_location),
        )
        .route(
            "/api/asset-hub/ai-modify-character",
            post(ai_modify_character),
        )
        .route(
            "/api/asset-hub/ai-modify-location",
            post(ai_modify_location),
        )
        .route(
            "/api/asset-hub/reference-to-character",
            post(reference_to_character),
        )
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};
    use serde_json::json;

    use super::*;

    #[test]
    fn require_task_locale_accepts_payload_meta_locale() {
        let body = json!({ "meta": { "locale": "zh-CN" } });
        let headers = HeaderMap::new();
        assert!(require_task_locale(&body, &headers).is_ok());
    }

    #[test]
    fn require_task_locale_accepts_accept_language_header() {
        let body = json!({});
        let mut headers = HeaderMap::new();
        headers.insert(
            "accept-language",
            HeaderValue::from_static("en-US,en;q=0.9"),
        );
        assert!(require_task_locale(&body, &headers).is_ok());
    }

    #[test]
    fn validate_generate_image_payload_rejects_invalid_type() {
        let body = json!({ "type": "panel", "id": "abc" });
        assert!(validate_generate_image_payload(&body).is_err());
    }

    #[test]
    fn validate_voice_design_payload_rejects_missing_preview_text() {
        let body = json!({ "voicePrompt": "warm narrator" });
        assert!(validate_voice_design_payload(&body).is_err());
    }

    #[test]
    fn validate_ai_design_payload_requires_user_instruction() {
        let body = json!({ "characterId": "char-1" });
        assert!(validate_ai_design_payload(&body).is_err());
    }

    #[test]
    fn build_restored_image_urls_prefers_previous_urls_array() {
        let restored =
            build_restored_image_urls(Some("legacy.jpg"), Some(r#"["a.jpg", "b.jpg", ""]"#));
        assert_eq!(restored, vec!["a.jpg".to_string(), "b.jpg".to_string()]);
    }

    #[test]
    fn build_restored_image_urls_falls_back_to_single_previous_url() {
        let restored = build_restored_image_urls(Some("legacy.jpg"), Some("[]"));
        assert_eq!(restored, vec!["legacy.jpg".to_string()]);
    }

    #[test]
    fn parse_optional_i32_text_rejects_invalid_number() {
        let result = parse_optional_i32_text(Some("abc".to_string()), "imageIndex");
        assert!(result.is_err());
    }

    #[test]
    fn normalize_file_extension_rejects_invalid_chars() {
        assert_eq!(normalize_file_extension(Some("jp*g")), None);
    }
}
