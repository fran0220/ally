use std::{collections::HashMap, io::Write};

use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{HeaderMap, Method, StatusCode, header},
    response::Response,
    routing::{any, get, post},
};
use chrono::{Duration as ChronoDuration, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{MySql, QueryBuilder};
use tracing::warn;
use uuid::Uuid;
use waoowaoo_core::media;
use zip::write::SimpleFileOptions;

use crate::{
    app_state::AppState, error::AppError, extractors::auth::AuthUser, routes::task_submit,
};

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct NovelProjectRow {
    id: String,
    #[sqlx(rename = "projectId")]
    project_id: String,
    #[sqlx(rename = "analysisModel")]
    analysis_model: Option<String>,
    #[sqlx(rename = "imageModel")]
    image_model: Option<String>,
    #[sqlx(rename = "videoModel")]
    video_model: Option<String>,
    #[sqlx(rename = "videoRatio")]
    video_ratio: String,
    #[sqlx(rename = "ttsRate")]
    tts_rate: String,
    #[sqlx(rename = "globalAssetText")]
    global_asset_text: Option<String>,
    #[sqlx(rename = "artStyle")]
    art_style: String,
    #[sqlx(rename = "artStylePrompt")]
    art_style_prompt: Option<String>,
    #[sqlx(rename = "characterModel")]
    character_model: Option<String>,
    #[sqlx(rename = "locationModel")]
    location_model: Option<String>,
    #[sqlx(rename = "storyboardModel")]
    storyboard_model: Option<String>,
    #[sqlx(rename = "editModel")]
    edit_model: Option<String>,
    #[sqlx(rename = "videoResolution")]
    video_resolution: String,
    #[sqlx(rename = "workflowMode")]
    workflow_mode: String,
    #[sqlx(rename = "lastEpisodeId")]
    last_episode_id: Option<String>,
    #[sqlx(rename = "imageResolution")]
    image_resolution: String,
    #[sqlx(rename = "importStatus")]
    import_status: Option<String>,
    #[sqlx(rename = "capabilityOverrides")]
    capability_overrides: Option<String>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct EpisodeRow {
    id: String,
    #[sqlx(rename = "novelPromotionProjectId")]
    novel_promotion_project_id: String,
    #[sqlx(rename = "episodeNumber")]
    episode_number: i32,
    name: String,
    description: Option<String>,
    #[sqlx(rename = "novelText")]
    novel_text: Option<String>,
    #[sqlx(rename = "audioUrl")]
    audio_url: Option<String>,
    #[sqlx(rename = "audioMediaId")]
    audio_media_id: Option<String>,
    #[sqlx(rename = "srtContent")]
    srt_content: Option<String>,
    #[sqlx(rename = "speakerVoices")]
    speaker_voices: Option<String>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct CharacterRow {
    id: String,
    #[sqlx(rename = "novelPromotionProjectId")]
    novel_promotion_project_id: String,
    name: String,
    aliases: Option<String>,
    #[sqlx(rename = "profileData")]
    profile_data: Option<String>,
    #[sqlx(rename = "profileConfirmed")]
    profile_confirmed: bool,
    #[sqlx(rename = "customVoiceUrl")]
    custom_voice_url: Option<String>,
    #[sqlx(rename = "customVoiceMediaId")]
    custom_voice_media_id: Option<String>,
    #[sqlx(rename = "voiceId")]
    voice_id: Option<String>,
    #[sqlx(rename = "voiceType")]
    voice_type: Option<String>,
    introduction: Option<String>,
    #[sqlx(rename = "sourceGlobalCharacterId")]
    source_global_character_id: Option<String>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct CharacterAppearanceRow {
    id: String,
    #[sqlx(rename = "characterId")]
    character_id: String,
    #[sqlx(rename = "appearanceIndex")]
    appearance_index: i32,
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
    #[sqlx(rename = "novelPromotionProjectId")]
    novel_promotion_project_id: String,
    name: String,
    summary: Option<String>,
    #[sqlx(rename = "sourceGlobalLocationId")]
    source_global_location_id: Option<String>,
    #[sqlx(rename = "selectedImageId")]
    selected_image_id: Option<String>,
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
    #[sqlx(rename = "previousDescription")]
    previous_description: Option<String>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct ClipRow {
    id: String,
    #[sqlx(rename = "episodeId")]
    episode_id: String,
    start: Option<i32>,
    end: Option<i32>,
    duration: Option<i32>,
    summary: String,
    location: Option<String>,
    content: String,
    characters: Option<String>,
    #[sqlx(rename = "endText")]
    end_text: Option<String>,
    #[sqlx(rename = "shotCount")]
    shot_count: Option<i32>,
    #[sqlx(rename = "startText")]
    start_text: Option<String>,
    screenplay: Option<String>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct StoryboardRow {
    id: String,
    #[sqlx(rename = "episodeId")]
    episode_id: String,
    #[sqlx(rename = "clipId")]
    clip_id: String,
    #[sqlx(rename = "storyboardImageUrl")]
    storyboard_image_url: Option<String>,
    #[sqlx(rename = "panelCount")]
    panel_count: i32,
    #[sqlx(rename = "storyboardTextJson")]
    storyboard_text_json: Option<String>,
    #[sqlx(rename = "imageHistory")]
    image_history: Option<String>,
    #[sqlx(rename = "candidateImages")]
    candidate_images: Option<String>,
    #[sqlx(rename = "lastError")]
    last_error: Option<String>,
    #[sqlx(rename = "photographyPlan")]
    photography_plan: Option<String>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct StoryboardPanelDetailRow {
    id: String,
    #[sqlx(rename = "storyboardId")]
    storyboard_id: String,
    #[sqlx(rename = "panelIndex")]
    panel_index: i32,
    #[sqlx(rename = "panelNumber")]
    panel_number: Option<i32>,
    #[sqlx(rename = "shotType")]
    shot_type: Option<String>,
    #[sqlx(rename = "cameraMove")]
    camera_move: Option<String>,
    description: Option<String>,
    location: Option<String>,
    characters: Option<String>,
    #[sqlx(rename = "srtStart")]
    srt_start: Option<f64>,
    #[sqlx(rename = "srtEnd")]
    srt_end: Option<f64>,
    duration: Option<f64>,
    #[sqlx(rename = "videoPrompt")]
    video_prompt: Option<String>,
    #[sqlx(rename = "firstLastFramePrompt")]
    first_last_frame_prompt: Option<String>,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
    #[sqlx(rename = "candidateImages")]
    candidate_images: Option<String>,
    #[sqlx(rename = "linkedToNextPanel")]
    linked_to_next_panel: bool,
    #[sqlx(rename = "actingNotes")]
    acting_notes: Option<String>,
    #[sqlx(rename = "photographyRules")]
    photography_rules: Option<String>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct StoryboardPanelMediaRow {
    id: String,
    #[sqlx(rename = "storyboardId")]
    storyboard_id: String,
    #[sqlx(rename = "panelIndex")]
    panel_index: i32,
    #[sqlx(rename = "panelNumber")]
    panel_number: Option<i32>,
    #[sqlx(rename = "shotType")]
    shot_type: Option<String>,
    #[sqlx(rename = "cameraMove")]
    camera_move: Option<String>,
    description: Option<String>,
    location: Option<String>,
    characters: Option<String>,
    #[sqlx(rename = "srtStart")]
    srt_start: Option<f64>,
    #[sqlx(rename = "srtEnd")]
    srt_end: Option<f64>,
    duration: Option<f64>,
    #[sqlx(rename = "videoPrompt")]
    video_prompt: Option<String>,
    #[sqlx(rename = "firstLastFramePrompt")]
    first_last_frame_prompt: Option<String>,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
    #[sqlx(rename = "imageMediaId")]
    image_media_id: Option<String>,
    #[sqlx(rename = "candidateImages")]
    candidate_images: Option<String>,
    #[sqlx(rename = "videoUrl")]
    video_url: Option<String>,
    #[sqlx(rename = "videoMediaId")]
    video_media_id: Option<String>,
    #[sqlx(rename = "lipSyncVideoUrl")]
    lip_sync_video_url: Option<String>,
    #[sqlx(rename = "sketchImageUrl")]
    sketch_image_url: Option<String>,
    #[sqlx(rename = "previousImageUrl")]
    previous_image_url: Option<String>,
    #[sqlx(rename = "previousImageMediaId")]
    previous_image_media_id: Option<String>,
    #[sqlx(rename = "linkedToNextPanel")]
    linked_to_next_panel: bool,
    #[sqlx(rename = "actingNotes")]
    acting_notes: Option<String>,
    #[sqlx(rename = "photographyRules")]
    photography_rules: Option<String>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct ProjectSummaryRow {
    id: String,
    name: String,
    description: Option<String>,
    mode: String,
    #[sqlx(rename = "userId")]
    user_id: String,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
    #[sqlx(rename = "lastAccessedAt")]
    last_accessed_at: Option<NaiveDateTime>,
}

#[derive(Debug, sqlx::FromRow)]
struct GlobalCharacterSourceRow {
    name: String,
    aliases: Option<String>,
    #[sqlx(rename = "profileData")]
    profile_data: Option<String>,
    #[sqlx(rename = "voiceId")]
    voice_id: Option<String>,
    #[sqlx(rename = "voiceType")]
    voice_type: Option<String>,
    #[sqlx(rename = "customVoiceUrl")]
    custom_voice_url: Option<String>,
    #[sqlx(rename = "customVoiceMediaId")]
    custom_voice_media_id: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct GlobalCharacterAppearanceSourceRow {
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
    #[sqlx(rename = "previousDescription")]
    previous_description: Option<String>,
    #[sqlx(rename = "previousDescriptions")]
    previous_descriptions: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct GlobalLocationSourceRow {
    name: String,
    summary: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct GlobalLocationImageSourceRow {
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
    #[sqlx(rename = "previousDescription")]
    previous_description: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct GlobalVoiceSourceRow {
    name: String,
    #[sqlx(rename = "voiceId")]
    voice_id: Option<String>,
    #[sqlx(rename = "voiceType")]
    voice_type: Option<String>,
    #[sqlx(rename = "customVoiceUrl")]
    custom_voice_url: Option<String>,
    #[sqlx(rename = "customVoiceMediaId")]
    custom_voice_media_id: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct VoiceLineRow {
    id: String,
    #[sqlx(rename = "episodeId")]
    episode_id: String,
    #[sqlx(rename = "lineIndex")]
    line_index: i32,
    speaker: String,
    content: String,
    #[sqlx(rename = "voicePresetId")]
    voice_preset_id: Option<String>,
    #[sqlx(rename = "audioUrl")]
    audio_url: Option<String>,
    #[sqlx(rename = "audioMediaId")]
    audio_media_id: Option<String>,
    #[sqlx(rename = "emotionPrompt")]
    emotion_prompt: Option<String>,
    #[sqlx(rename = "emotionStrength")]
    emotion_strength: Option<f64>,
    #[sqlx(rename = "matchedPanelIndex")]
    matched_panel_index: Option<i32>,
    #[sqlx(rename = "matchedStoryboardId")]
    matched_storyboard_id: Option<String>,
    #[sqlx(rename = "audioDuration")]
    audio_duration: Option<i32>,
    #[sqlx(rename = "matchedPanelId")]
    matched_panel_id: Option<String>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, sqlx::FromRow)]
struct MediaPanelDownloadRow {
    id: String,
    #[sqlx(rename = "storyboardId")]
    storyboard_id: String,
    #[sqlx(rename = "panelIndex")]
    panel_index: i32,
    description: Option<String>,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
    #[sqlx(rename = "videoUrl")]
    video_url: Option<String>,
    #[sqlx(rename = "lipSyncVideoUrl")]
    lip_sync_video_url: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct EditorRow {
    id: String,
    #[sqlx(rename = "episodeId")]
    episode_id: String,
    #[sqlx(rename = "projectData")]
    project_data: String,
    #[sqlx(rename = "renderStatus")]
    render_status: Option<String>,
    #[sqlx(rename = "renderTaskId")]
    render_task_id: Option<String>,
    #[sqlx(rename = "outputUrl")]
    output_url: Option<String>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct MediaObjectRow {
    id: String,
    #[sqlx(rename = "publicId")]
    public_id: String,
    #[sqlx(rename = "storageKey")]
    storage_key: String,
    #[sqlx(rename = "mimeType")]
    mime_type: Option<String>,
    #[sqlx(rename = "sizeBytes")]
    size_bytes: Option<i64>,
    width: Option<i32>,
    height: Option<i32>,
    #[sqlx(rename = "durationMs")]
    duration_ms: Option<i32>,
    sha256: Option<String>,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn normalize_optional_json(value: Option<Value>) -> Option<String> {
    value.and_then(|item| serde_json::to_string(&item).ok())
}

fn parse_json_str(value: Option<&str>) -> Option<Value> {
    value
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .filter(|item| !item.is_null())
}

fn parse_json_string_array(value: Option<&str>) -> Vec<String> {
    let Some(raw) = value else {
        return Vec::new();
    };

    let Ok(parsed) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };

    parsed
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|item| item.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn normalize_voice_line_audio_url(value: &mut Value) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    let Some(raw_audio_url) = object.get("audioUrl").and_then(Value::as_str) else {
        return;
    };
    if let Some(public_url) = media::to_public_media_url(Some(raw_audio_url)) {
        object.insert("audioUrl".to_string(), Value::String(public_url));
    }
}

fn normalize_location_description(input: &str) -> String {
    input.trim().trim_end_matches('，').trim().to_string()
}

fn localized_msg<'a>(locale: &str, zh: &'a str, en: &'a str) -> &'a str {
    if locale == "en" { en } else { zh }
}

fn request_locale(body: Option<&Value>, headers: Option<&HeaderMap>) -> &'static str {
    body.and_then(read_task_locale_from_body)
        .or_else(|| headers.and_then(read_task_locale_from_headers))
        .unwrap_or("zh")
}

fn map_art_style_prompt(style: &str, locale: &str) -> Option<&'static str> {
    match style.trim() {
        "american-comic" => Some(localized_msg(
            locale,
            "美式漫画风格",
            "American comic style",
        )),
        "chinese-comic" => Some(localized_msg(
            locale,
            "精致国漫风格",
            "Premium Chinese comic style",
        )),
        "anime" => Some(localized_msg(
            locale,
            "日系动漫风格",
            "Japanese anime style",
        )),
        "realistic" => Some(localized_msg(
            locale,
            "真人照片写实风格",
            "Photorealistic style",
        )),
        _ => None,
    }
}

fn body_string_array(body: &Value, key: &str, max_len: usize) -> Vec<String> {
    body.get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .take(max_len)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn media_ref_json(media_row: MediaObjectRow) -> Value {
    let relative_url = format!("/m/{}", media_row.public_id);
    let resolved_url =
        media::to_public_media_url(Some(&relative_url)).unwrap_or_else(|| relative_url.clone());
    json!({
      "id": media_row.id,
      "publicId": media_row.public_id,
      "url": resolved_url,
      "mimeType": media_row.mime_type,
      "sizeBytes": media_row.size_bytes,
      "width": media_row.width,
      "height": media_row.height,
      "durationMs": media_row.duration_ms,
      "sha256": media_row.sha256,
      "updatedAt": media_row.updated_at,
      "storageKey": media_row.storage_key,
    })
}

async fn resolve_media_row_for_voice_line(
    state: &AppState,
    audio_media_id: Option<&str>,
    audio_url: Option<&str>,
) -> Result<Option<MediaObjectRow>, AppError> {
    if let Some(media_id) = audio_media_id
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        let by_id = sqlx::query_as::<_, MediaObjectRow>(
            "SELECT id, publicId, storageKey, mimeType, sizeBytes, width, height, durationMs, sha256, updatedAt
             FROM media_objects
             WHERE id = ?
             LIMIT 1",
        )
        .bind(media_id)
        .fetch_optional(&state.mysql)
        .await?;
        if by_id.is_some() {
            return Ok(by_id);
        }
    }

    let Some(audio_url) = audio_url else {
        return Ok(None);
    };
    let Some((resolved_media_id, _)) = resolve_media_object_from_value(state, audio_url).await?
    else {
        return Ok(None);
    };

    let row = sqlx::query_as::<_, MediaObjectRow>(
        "SELECT id, publicId, storageKey, mimeType, sizeBytes, width, height, durationMs, sha256, updatedAt
         FROM media_objects
         WHERE id = ?
         LIMIT 1",
    )
    .bind(resolved_media_id)
    .fetch_optional(&state.mysql)
    .await?;

    Ok(row)
}

async fn with_voice_line_media(state: &AppState, line: &VoiceLineRow) -> Result<Value, AppError> {
    let mut value = serde_json::to_value(line)
        .map_err(|err| AppError::internal(format!("failed to encode voice line: {err}")))?;
    let media_row = resolve_media_row_for_voice_line(
        state,
        line.audio_media_id.as_deref(),
        line.audio_url.as_deref(),
    )
    .await?;

    let Some(object) = value.as_object_mut() else {
        return Ok(value);
    };
    let matched_panel = line.matched_panel_id.as_ref().map(|panel_id| {
        json!({
          "id": panel_id,
          "storyboardId": line.matched_storyboard_id,
          "panelIndex": line.matched_panel_index,
        })
    });
    object.insert(
        "matchedPanel".to_string(),
        matched_panel.unwrap_or(Value::Null),
    );

    if let Some(media_row) = media_row {
        let media_json = media_ref_json(media_row);
        let media_url = media_json
            .get("url")
            .and_then(Value::as_str)
            .map(|item| item.to_string());
        object.insert("media".to_string(), media_json.clone());
        object.insert("audioMedia".to_string(), media_json);
        if let Some(media_url) = media_url {
            object.insert("audioUrl".to_string(), Value::String(media_url));
        }
    } else {
        object.insert("media".to_string(), Value::Null);
        object.insert("audioMedia".to_string(), Value::Null);
        normalize_voice_line_audio_url(&mut value);
    }

    Ok(value)
}

fn episode_to_json(episode: EpisodeRow) -> Value {
    json!({
      "id": episode.id,
      "novelPromotionProjectId": episode.novel_promotion_project_id,
      "episodeNumber": episode.episode_number,
      "name": episode.name,
      "description": episode.description,
      "novelText": episode.novel_text,
      "audioUrl": episode.audio_url,
      "audioMediaId": episode.audio_media_id,
      "srtContent": episode.srt_content,
      "speakerVoices": parse_json_str(episode.speaker_voices.as_deref()),
      "createdAt": episode.created_at,
      "updatedAt": episode.updated_at,
    })
}

fn body_string(body: &Value, key: &str) -> Option<String> {
    body.get(key)
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn query_string(query: &HashMap<String, String>, key: &str) -> Option<String> {
    query
        .get(key)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn body_i32(body: &Value, key: &str) -> Option<i32> {
    body.get(key)
        .and_then(Value::as_i64)
        .and_then(|item| i32::try_from(item).ok())
}

fn body_f64(body: &Value, key: &str) -> Option<f64> {
    body.get(key).and_then(Value::as_f64)
}

fn body_bool(body: &Value, key: &str) -> Option<bool> {
    body.get(key).and_then(Value::as_bool)
}

fn parse_nullable_f64_field(value: &Value, field: &str) -> Result<Option<f64>, AppError> {
    if value.is_null() {
        return Ok(None);
    }

    if let Some(number) = value.as_f64() {
        return Ok(Some(number));
    }

    if let Some(number) = value.as_i64() {
        return Ok(Some(number as f64));
    }

    if let Some(raw) = value.as_str() {
        let normalized = raw.trim();
        if normalized.is_empty() {
            return Ok(None);
        }
        let parsed = normalized
            .parse::<f64>()
            .map_err(|_| AppError::invalid_params(format!("{field} must be a number or null")))?;
        return Ok(Some(parsed));
    }

    Err(AppError::invalid_params(format!(
        "{field} must be a number or null"
    )))
}

fn parse_nullable_i32_field(value: &Value, field: &str) -> Result<Option<i32>, AppError> {
    if value.is_null() {
        return Ok(None);
    }

    if let Some(number) = value.as_i64() {
        return i32::try_from(number)
            .map(Some)
            .map_err(|_| AppError::invalid_params(format!("{field} must be an integer or null")));
    }

    if let Some(raw) = value.as_str() {
        let normalized = raw.trim();
        if normalized.is_empty() {
            return Ok(None);
        }
        let parsed = normalized
            .parse::<i32>()
            .map_err(|_| AppError::invalid_params(format!("{field} must be an integer or null")))?;
        return Ok(Some(parsed));
    }

    Err(AppError::invalid_params(format!(
        "{field} must be an integer or null"
    )))
}

fn parse_body_json(raw: &Bytes) -> Result<Value, AppError> {
    if raw.is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_slice::<Value>(raw)
        .map_err(|err| AppError::invalid_params(format!("invalid json body: {err}")))
}

fn extract_media_public_id(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let candidate = if let Some(rest) = trimmed.strip_prefix("/m/") {
        rest
    } else if let Some(index) = trimmed.find("/m/") {
        &trimmed[index + 3..]
    } else {
        return None;
    };

    candidate
        .split(['/', '?', '#'])
        .next()
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
}

fn extract_storage_key_from_files_route(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let marker = "/api/files/";
    let index = trimmed.find(marker)?;
    let encoded = trimmed[index + marker.len()..]
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .trim_matches('/');
    if encoded.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    for seg in encoded.split('/') {
        let decoded = urlencoding::decode(seg).ok()?.into_owned();
        parts.push(decoded);
    }
    Some(parts.join("/"))
}

async fn resolve_media_object_from_value(
    state: &AppState,
    raw: &str,
) -> Result<Option<(String, String)>, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    if let Some(public_id) = extract_media_public_id(trimmed) {
        let row: Option<(String, String)> =
            sqlx::query_as("SELECT id, storageKey FROM media_objects WHERE publicId = ? LIMIT 1")
                .bind(&public_id)
                .fetch_optional(&state.mysql)
                .await?;
        return Ok(row);
    }

    if let Some(storage_key) = extract_storage_key_from_files_route(trimmed) {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT id FROM media_objects WHERE storageKey = ? LIMIT 1")
                .bind(&storage_key)
                .fetch_optional(&state.mysql)
                .await?;
        if let Some((media_id,)) = row {
            return Ok(Some((media_id, storage_key)));
        }
    }

    let row: Option<(String, String)> =
        sqlx::query_as("SELECT id, storageKey FROM media_objects WHERE storageKey = ? LIMIT 1")
            .bind(trimmed)
            .fetch_optional(&state.mysql)
            .await?;
    if row.is_some() {
        return Ok(row);
    }

    Ok(None)
}

fn normalized_storage_type() -> String {
    std::env::var("STORAGE_TYPE")
        .ok()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "cos".to_string())
}

fn encode_storage_key_for_url(raw_key: &str) -> String {
    raw_key
        .trim_matches('/')
        .split('/')
        .filter(|part| !part.is_empty())
        .map(urlencoding::encode)
        .collect::<Vec<_>>()
        .join("/")
}

fn cos_public_media_url(storage_key: &str) -> Option<String> {
    let encoded = encode_storage_key_for_url(storage_key);
    if encoded.is_empty() {
        return None;
    }

    if let Ok(base_url) = std::env::var("COS_PUBLIC_BASE_URL") {
        let normalized = base_url.trim().trim_end_matches('/');
        if !normalized.is_empty() {
            return Some(format!("{normalized}/{encoded}"));
        }
    }

    if let (Ok(bucket), Ok(region)) = (std::env::var("COS_BUCKET"), std::env::var("COS_REGION")) {
        let bucket = bucket.trim();
        let region = region.trim();
        if !bucket.is_empty() && !region.is_empty() {
            return Some(format!(
                "https://{bucket}.cos.{region}.myqcloud.com/{encoded}"
            ));
        }
    }

    None
}

async fn resolve_storage_key_from_media_source(
    state: &AppState,
    source: &str,
) -> Result<Option<String>, AppError> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    if let Some(storage_key) = extract_storage_key_from_files_route(trimmed) {
        return Ok(Some(storage_key));
    }

    if let Some((_, storage_key)) = resolve_media_object_from_value(state, trimmed).await? {
        return Ok(Some(storage_key));
    }

    if !trimmed.starts_with("http://")
        && !trimmed.starts_with("https://")
        && !trimmed.starts_with("data:")
        && !trimmed.starts_with('/')
    {
        return Ok(Some(trimmed.to_string()));
    }

    Ok(None)
}

async fn resolve_media_fetch_url(state: &AppState, source: &str) -> Result<String, AppError> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid_params("media source is required"));
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Ok(trimmed.to_string());
    }

    if let Some(storage_key) = resolve_storage_key_from_media_source(state, trimmed).await? {
        if normalized_storage_type() == "local" {
            return Ok(media::to_fetchable_url(&storage_key));
        }

        if let Some(url) = cos_public_media_url(&storage_key) {
            return Ok(url);
        }

        return Err(AppError::internal(
            "COS download requires COS_PUBLIC_BASE_URL or COS_BUCKET/COS_REGION",
        ));
    }

    if trimmed.starts_with('/') {
        return Ok(media::to_fetchable_url(trimmed));
    }

    Ok(trimmed.to_string())
}

async fn download_binary_from_media_source(
    state: &AppState,
    source: &str,
) -> Result<(Vec<u8>, String), AppError> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid_params("media source is required"));
    }

    if trimmed.starts_with("data:") {
        return media::download_source_bytes(trimmed)
            .await
            .map_err(AppError::from);
    }

    let fetch_url = resolve_media_fetch_url(state, trimmed).await?;
    media::download_source_bytes(&fetch_url)
        .await
        .map_err(AppError::from)
}

fn sanitize_filename_segment(raw: &str, fallback: &str) -> String {
    let value = raw.trim();
    let normalized = if value.is_empty() { fallback } else { value };
    let mut safe = String::with_capacity(normalized.len());
    for ch in normalized.chars() {
        if matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
            safe.push('_');
        } else {
            safe.push(ch);
        }
    }
    let compact = safe.trim().to_string();
    if compact.is_empty() {
        fallback.to_string()
    } else {
        compact
    }
}

fn sanitize_voice_content_segment(raw: &str) -> String {
    let slice = raw.chars().take(15).collect::<String>();
    let mut value = sanitize_filename_segment(&slice, "line");
    value = value
        .chars()
        .map(|ch| if ch.is_whitespace() { '_' } else { ch })
        .collect::<String>();
    if value.trim_matches('_').is_empty() {
        "line".to_string()
    } else {
        value
    }
}

fn infer_extension_from_content_type(content_type: &str, default_ext: &str) -> String {
    let normalized = content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase();
    if normalized.contains("jpeg") || normalized.contains("jpg") {
        return "jpg".to_string();
    }
    if normalized.contains("webp") {
        return "webp".to_string();
    }
    if normalized.contains("gif") {
        return "gif".to_string();
    }
    if normalized.contains("png") {
        return "png".to_string();
    }
    if normalized.contains("wav") {
        return "wav".to_string();
    }
    if normalized.contains("mpeg") || normalized.contains("mp3") {
        return "mp3".to_string();
    }
    if normalized.contains("mp4") {
        return "mp4".to_string();
    }
    default_ext.to_string()
}

fn infer_extension_from_source(source: &str, default_ext: &str) -> String {
    let trimmed = source.trim();
    let path = trimmed
        .split('?')
        .next()
        .unwrap_or(trimmed)
        .split('#')
        .next()
        .unwrap_or(trimmed)
        .to_ascii_lowercase();
    for ext in [
        "jpeg", "jpg", "png", "webp", "gif", "wav", "mp3", "ogg", "mp4",
    ] {
        if path.ends_with(&format!(".{ext}")) {
            return if ext == "jpeg" {
                "jpg".to_string()
            } else {
                ext.to_string()
            };
        }
    }
    default_ext.to_string()
}

fn build_zip_archive(entries: Vec<(String, Vec<u8>)>) -> Result<Vec<u8>, AppError> {
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut zip = zip::ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for (name, bytes) in entries {
        zip.start_file(name, options)
            .map_err(|err| AppError::internal(format!("failed to open zip entry: {err}")))?;
        zip.write_all(&bytes)
            .map_err(|err| AppError::internal(format!("failed to write zip entry: {err}")))?;
    }

    let cursor = zip
        .finish()
        .map_err(|err| AppError::internal(format!("failed to finalize zip archive: {err}")))?;
    Ok(cursor.into_inner())
}

fn zip_attachment_response(bytes: Vec<u8>, file_name: &str) -> Result<Response, AppError> {
    let disposition = format!(
        "attachment; filename=\"{}\"",
        urlencoding::encode(file_name)
    );
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/zip")
        .header(header::CONTENT_DISPOSITION, disposition)
        .body(Body::from(bytes))
        .map_err(|err| AppError::internal(format!("failed to build zip response: {err}")))
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

fn is_valid_model_key(raw: &str) -> bool {
    let value = raw.trim();
    if value.is_empty() {
        return false;
    }

    let Some(marker_index) = value.find("::") else {
        return false;
    };

    let provider = value[..marker_index].trim();
    let model_id = value[marker_index + 2..].trim();
    !provider.is_empty() && !model_id.is_empty()
}

async fn verify_project_owner(
    state: &AppState,
    project_id: &str,
    user_id: &str,
) -> Result<(), AppError> {
    task_submit::verify_project_access(state, project_id, user_id).await
}

async fn get_novel_id(state: &AppState, project_id: &str) -> Result<String, AppError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT id FROM novel_promotion_projects WHERE projectId = ? LIMIT 1")
            .bind(project_id)
            .fetch_optional(&state.mysql)
            .await?;
    row.map(|item| item.0)
        .ok_or_else(|| AppError::not_found("novel promotion project not found"))
}

async fn build_patch_project_response(
    state: &AppState,
    project_id: &str,
) -> Result<Value, AppError> {
    let project = sqlx::query_as::<_, ProjectSummaryRow>(
        "SELECT id, name, description, mode, userId, createdAt, updatedAt, lastAccessedAt FROM projects WHERE id = ? LIMIT 1",
    )
    .bind(project_id)
    .fetch_one(&state.mysql)
    .await?;

    let novel = sqlx::query_as::<_, NovelProjectRow>(
        "SELECT id, projectId, analysisModel, imageModel, videoModel, videoRatio, ttsRate, globalAssetText, artStyle, artStylePrompt, characterModel, locationModel, storyboardModel, editModel, videoResolution, workflowMode, lastEpisodeId, imageResolution, importStatus, capabilityOverrides, createdAt, updatedAt FROM novel_promotion_projects WHERE projectId = ? LIMIT 1",
    )
    .bind(project_id)
    .fetch_one(&state.mysql)
    .await?;

    let capability_overrides =
        parse_json_str(novel.capability_overrides.as_deref()).unwrap_or_else(|| json!({}));

    Ok(json!({
      "id": project.id,
      "name": project.name,
      "description": project.description,
      "mode": project.mode,
      "userId": project.user_id,
      "createdAt": project.created_at,
      "updatedAt": project.updated_at,
      "lastAccessedAt": project.last_accessed_at,
      "novelPromotionData": {
        "id": novel.id,
        "projectId": novel.project_id,
        "analysisModel": novel.analysis_model,
        "imageModel": novel.image_model,
        "videoModel": novel.video_model,
        "videoRatio": novel.video_ratio,
        "ttsRate": novel.tts_rate,
        "globalAssetText": novel.global_asset_text,
        "artStyle": novel.art_style,
        "artStylePrompt": novel.art_style_prompt,
        "characterModel": novel.character_model,
        "locationModel": novel.location_model,
        "storyboardModel": novel.storyboard_model,
        "editModel": novel.edit_model,
        "videoResolution": novel.video_resolution,
        "workflowMode": novel.workflow_mode,
        "lastEpisodeId": novel.last_episode_id,
        "imageResolution": novel.image_resolution,
        "importStatus": novel.import_status,
        "capabilityOverrides": capability_overrides,
        "createdAt": novel.created_at,
        "updatedAt": novel.updated_at,
      }
    }))
}

async fn get_root(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(&state, &project_id, &user.id).await?;

    let project_name: Option<(String,)> =
        sqlx::query_as("SELECT name FROM projects WHERE id = ? LIMIT 1")
            .bind(&project_id)
            .fetch_optional(&state.mysql)
            .await?;

    let novel = sqlx::query_as::<_, NovelProjectRow>(
        "SELECT id, projectId, analysisModel, imageModel, videoModel, videoRatio, ttsRate, globalAssetText, artStyle, artStylePrompt, characterModel, locationModel, storyboardModel, editModel, videoResolution, workflowMode, lastEpisodeId, imageResolution, importStatus, capabilityOverrides, createdAt, updatedAt FROM novel_promotion_projects WHERE projectId = ? LIMIT 1",
    )
    .bind(&project_id)
    .fetch_optional(&state.mysql)
    .await?
    .ok_or_else(|| AppError::not_found("novel promotion project not found"))?;

    let episodes = sqlx::query_as::<_, EpisodeRow>(
        "SELECT id, novelPromotionProjectId, episodeNumber, name, description, novelText, audioUrl, audioMediaId, srtContent, speakerVoices, createdAt, updatedAt FROM novel_promotion_episodes WHERE novelPromotionProjectId = ? ORDER BY episodeNumber ASC",
    )
    .bind(&novel.id)
    .fetch_all(&state.mysql)
    .await?;
    let episodes_json = episodes
        .into_iter()
        .map(episode_to_json)
        .collect::<Vec<_>>();

    let characters = sqlx::query_as::<_, CharacterRow>(
        "SELECT id, novelPromotionProjectId, name, aliases, profileData, profileConfirmed, customVoiceUrl, customVoiceMediaId, voiceId, voiceType, introduction, sourceGlobalCharacterId, createdAt, updatedAt FROM novel_promotion_characters WHERE novelPromotionProjectId = ? ORDER BY createdAt ASC",
    )
    .bind(&novel.id)
    .fetch_all(&state.mysql)
    .await?;

    let locations = sqlx::query_as::<_, LocationRow>(
        "SELECT id, novelPromotionProjectId, name, summary, sourceGlobalLocationId, selectedImageId, createdAt, updatedAt FROM novel_promotion_locations WHERE novelPromotionProjectId = ? ORDER BY createdAt ASC",
    )
    .bind(&novel.id)
    .fetch_all(&state.mysql)
    .await?;

    let character_ids = characters
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    let location_ids = locations
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();

    let appearances = if character_ids.is_empty() {
        Vec::new()
    } else {
        let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
            "SELECT id, characterId, appearanceIndex, description, descriptions, imageUrl, imageMediaId, imageUrls, selectedIndex, previousImageUrl, previousDescription, previousDescriptions, createdAt, updatedAt FROM character_appearances WHERE characterId IN (",
        );
        let mut separated = qb.separated(",");
        for id in &character_ids {
            separated.push_bind(id);
        }
        separated.push_unseparated(") ORDER BY appearanceIndex ASC, createdAt ASC");
        qb.build_query_as::<CharacterAppearanceRow>()
            .fetch_all(&state.mysql)
            .await?
    };

    let images = if location_ids.is_empty() {
        Vec::new()
    } else {
        let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
            "SELECT id, locationId, imageIndex, description, imageUrl, imageMediaId, isSelected, previousImageUrl, previousDescription, createdAt, updatedAt FROM location_images WHERE locationId IN (",
        );
        let mut separated = qb.separated(",");
        for id in &location_ids {
            separated.push_bind(id);
        }
        separated.push_unseparated(") ORDER BY imageIndex ASC, createdAt ASC");
        qb.build_query_as::<LocationImageRow>()
            .fetch_all(&state.mysql)
            .await?
    };

    let mut app_map: HashMap<String, Vec<CharacterAppearanceRow>> = HashMap::new();
    for item in appearances {
        app_map
            .entry(item.character_id.clone())
            .or_default()
            .push(item);
    }

    let mut img_map: HashMap<String, Vec<LocationImageRow>> = HashMap::new();
    for item in images {
        img_map
            .entry(item.location_id.clone())
            .or_default()
            .push(item);
    }

    let character_json = characters
        .into_iter()
        .map(|item| {
            json!({
              "id": item.id,
              "novelPromotionProjectId": item.novel_promotion_project_id,
              "name": item.name,
              "aliases": parse_json_str(item.aliases.as_deref()),
              "profileData": parse_json_str(item.profile_data.as_deref()),
              "profileConfirmed": item.profile_confirmed,
              "customVoiceUrl": item.custom_voice_url,
              "customVoiceMediaId": item.custom_voice_media_id,
              "voiceId": item.voice_id,
              "voiceType": item.voice_type,
              "introduction": item.introduction,
              "sourceGlobalCharacterId": item.source_global_character_id,
              "appearances": app_map.remove(&item.id).unwrap_or_default(),
              "createdAt": item.created_at,
              "updatedAt": item.updated_at,
            })
        })
        .collect::<Vec<_>>();

    let location_json = locations
        .into_iter()
        .map(|item| {
            json!({
              "id": item.id,
              "novelPromotionProjectId": item.novel_promotion_project_id,
              "name": item.name,
              "summary": item.summary,
              "sourceGlobalLocationId": item.source_global_location_id,
              "selectedImageId": item.selected_image_id,
              "images": img_map.remove(&item.id).unwrap_or_default(),
              "createdAt": item.created_at,
              "updatedAt": item.updated_at,
            })
        })
        .collect::<Vec<_>>();

    let capability_overrides =
        parse_json_str(novel.capability_overrides.as_deref()).unwrap_or_else(|| json!({}));
    let project = json!({
      "id": project_id,
      "name": project_name.map(|r| r.0),
      "novelPromotionData": {
        "id": novel.id,
        "projectId": novel.project_id,
        "analysisModel": novel.analysis_model,
        "imageModel": novel.image_model,
        "videoModel": novel.video_model,
        "videoRatio": novel.video_ratio,
        "ttsRate": novel.tts_rate,
        "globalAssetText": novel.global_asset_text,
        "artStyle": novel.art_style,
        "artStylePrompt": novel.art_style_prompt,
        "characterModel": novel.character_model,
        "locationModel": novel.location_model,
        "storyboardModel": novel.storyboard_model,
        "editModel": novel.edit_model,
        "videoResolution": novel.video_resolution,
        "workflowMode": novel.workflow_mode,
        "lastEpisodeId": novel.last_episode_id,
        "imageResolution": novel.image_resolution,
        "importStatus": novel.import_status,
        "episodes": episodes_json,
        "characters": character_json,
        "locations": location_json,
        "createdAt": novel.created_at,
        "updatedAt": novel.updated_at
      }
    });

    Ok(Json(json!({
      "project": project,
      "capabilityOverrides": capability_overrides,
    })))
}

async fn patch_root(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(&state, &project_id, &user.id).await?;

    let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new("UPDATE novel_promotion_projects SET ");
    let mut separated = qb.separated(", ");
    let mut touched = false;

    for (db_col, body_key) in [
        ("analysisModel", "analysisModel"),
        ("imageModel", "imageModel"),
        ("videoModel", "videoModel"),
        ("videoRatio", "videoRatio"),
        ("ttsRate", "ttsRate"),
        ("globalAssetText", "globalAssetText"),
        ("artStyle", "artStyle"),
        ("artStylePrompt", "artStylePrompt"),
        ("characterModel", "characterModel"),
        ("locationModel", "locationModel"),
        ("storyboardModel", "storyboardModel"),
        ("editModel", "editModel"),
        ("videoResolution", "videoResolution"),
        ("workflowMode", "workflowMode"),
        ("lastEpisodeId", "lastEpisodeId"),
        ("imageResolution", "imageResolution"),
        ("importStatus", "importStatus"),
    ] {
        if body.get(body_key).is_some() {
            touched = true;
            separated
                .push(format!("{db_col} = "))
                .push_bind_unseparated(normalize_optional_string(
                    body.get(body_key)
                        .and_then(Value::as_str)
                        .map(|item| item.to_string()),
                ));
        }
    }

    if let Some(value) = body.get("capabilityOverrides") {
        touched = true;
        separated
            .push("capabilityOverrides = ")
            .push_bind_unseparated(normalize_optional_json(Some(value.clone())));
    }

    if !touched {
        return Err(AppError::invalid_params("empty update payload"));
    }

    separated.push("updatedAt = NOW(3)");
    qb.push(" WHERE projectId = ");
    qb.push_bind(&project_id);

    qb.build().execute(&state.mysql).await?;

    let project = build_patch_project_response(&state, &project_id).await?;
    Ok(Json(json!({ "success": true, "project": project })))
}

#[allow(clippy::too_many_arguments)]
async fn submit_novel_task(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
    task_type: &str,
    target_type: &str,
    target_id: &str,
    body: Value,
    priority: Option<i32>,
    accept_language: Option<&str>,
) -> Result<Json<Value>, AppError> {
    let episode_id = body_string(&body, "episodeId");
    task_submit::submit_task(
        state,
        user,
        task_submit::SubmitTaskArgs {
            project_id,
            episode_id: episode_id.as_deref(),
            task_type,
            target_type,
            target_id,
            priority,
            max_attempts: None,
            accept_language,
            payload: body,
        },
    )
    .await
}

fn task_submission_priority(path: &str) -> Option<i32> {
    match path {
        "analyze" | "voice-analyze" => Some(1),
        "story-to-script-stream" | "script-to-storyboard-stream" | "screenplay-conversion" => {
            Some(2)
        }
        _ => None,
    }
}

fn normalize_task_target(
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
        .or_else(|| body.get("storyboardId").and_then(Value::as_str))
        .or_else(|| body.get("panelId").and_then(Value::as_str))
        .or_else(|| body.get("clipId").and_then(Value::as_str))
        .or_else(|| body.get("episodeId").and_then(Value::as_str))
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

fn validate_generate_image_submission(body: &Value) -> Result<(), AppError> {
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

fn validate_generate_character_image_submission(body: &Value) -> Result<(), AppError> {
    if body_string(body, "characterId").is_none() {
        return Err(AppError::invalid_params("characterId is required"));
    }

    Ok(())
}

fn validate_generate_video_submission(body: &Value) -> Result<(), AppError> {
    let video_model = body_string(body, "videoModel")
        .ok_or_else(|| AppError::invalid_params("videoModel is required"))?;
    if !is_valid_model_key(&video_model) {
        return Err(AppError::invalid_params(
            "videoModel must be a valid model key",
        ));
    }

    if body_bool(body, "all") == Some(true) {
        if body_string(body, "episodeId").is_none() {
            return Err(AppError::invalid_params(
                "episodeId is required when all is true",
            ));
        }
        return Ok(());
    }

    if body_string(body, "storyboardId").is_none() {
        return Err(AppError::invalid_params("storyboardId is required"));
    }
    if body.get("panelIndex").is_none() {
        return Err(AppError::invalid_params("panelIndex is required"));
    }

    Ok(())
}

fn validate_task_submission_payload(
    path: &str,
    body: &Value,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    match path {
        "generate-image" => {
            require_task_locale(body, headers)?;
            validate_generate_image_submission(body)?;
        }
        "generate-character-image" => {
            require_task_locale(body, headers)?;
            validate_generate_character_image_submission(body)?;
        }
        "generate-video" => {
            require_task_locale(body, headers)?;
            validate_generate_video_submission(body)?;
        }
        _ => {}
    }

    Ok(())
}

async fn handle_assets(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;
    let novel_id = get_novel_id(state, project_id).await?;

    let characters = sqlx::query_as::<_, CharacterRow>(
        "SELECT id, novelPromotionProjectId, name, aliases, profileData, profileConfirmed, customVoiceUrl, customVoiceMediaId, voiceId, voiceType, introduction, sourceGlobalCharacterId, createdAt, updatedAt FROM novel_promotion_characters WHERE novelPromotionProjectId = ? ORDER BY createdAt ASC",
    )
    .bind(&novel_id)
    .fetch_all(&state.mysql)
    .await?;

    let locations = sqlx::query_as::<_, LocationRow>(
        "SELECT id, novelPromotionProjectId, name, summary, sourceGlobalLocationId, selectedImageId, createdAt, updatedAt FROM novel_promotion_locations WHERE novelPromotionProjectId = ? ORDER BY createdAt ASC",
    )
    .bind(&novel_id)
    .fetch_all(&state.mysql)
    .await?;

    let character_ids = characters
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    let location_ids = locations
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();

    let appearances = if character_ids.is_empty() {
        Vec::new()
    } else {
        let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
            "SELECT id, characterId, appearanceIndex, description, descriptions, imageUrl, imageMediaId, imageUrls, selectedIndex, previousImageUrl, previousDescription, previousDescriptions, createdAt, updatedAt FROM character_appearances WHERE characterId IN (",
        );
        let mut separated = qb.separated(",");
        for id in &character_ids {
            separated.push_bind(id);
        }
        separated.push_unseparated(") ORDER BY appearanceIndex ASC, createdAt ASC");
        qb.build_query_as::<CharacterAppearanceRow>()
            .fetch_all(&state.mysql)
            .await?
    };

    let images = if location_ids.is_empty() {
        Vec::new()
    } else {
        let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
            "SELECT id, locationId, imageIndex, description, imageUrl, imageMediaId, isSelected, previousImageUrl, previousDescription, createdAt, updatedAt FROM location_images WHERE locationId IN (",
        );
        let mut separated = qb.separated(",");
        for id in &location_ids {
            separated.push_bind(id);
        }
        separated.push_unseparated(") ORDER BY imageIndex ASC, createdAt ASC");
        qb.build_query_as::<LocationImageRow>()
            .fetch_all(&state.mysql)
            .await?
    };

    let mut app_map: HashMap<String, Vec<Value>> = HashMap::new();
    for item in appearances {
        let image_url = media::to_public_media_url(item.image_url.as_deref()).or(item.image_url);
        let previous_image_url = media::to_public_media_url(item.previous_image_url.as_deref())
            .or(item.previous_image_url);
        let value = json!({
          "id": item.id,
          "characterId": item.character_id,
          "appearanceIndex": item.appearance_index,
          "description": item.description,
          "descriptions": parse_json_str(item.descriptions.as_deref()),
          "imageUrl": image_url,
          "imageMediaId": item.image_media_id,
          "imageUrls": parse_json_str(item.image_urls.as_deref()).unwrap_or_else(|| json!([])),
          "selectedIndex": item.selected_index,
          "previousImageUrl": previous_image_url,
          "previousDescription": item.previous_description,
          "previousDescriptions": parse_json_str(item.previous_descriptions.as_deref()),
          "createdAt": item.created_at,
          "updatedAt": item.updated_at,
        });
        app_map.entry(item.character_id).or_default().push(value);
    }

    let mut img_map: HashMap<String, Vec<Value>> = HashMap::new();
    for item in images {
        let image_url = media::to_public_media_url(item.image_url.as_deref()).or(item.image_url);
        let previous_image_url = media::to_public_media_url(item.previous_image_url.as_deref())
            .or(item.previous_image_url);
        let value = json!({
          "id": item.id,
          "locationId": item.location_id,
          "imageIndex": item.image_index,
          "description": item.description,
          "imageUrl": image_url,
          "imageMediaId": item.image_media_id,
          "isSelected": item.is_selected,
          "previousImageUrl": previous_image_url,
          "previousDescription": item.previous_description,
          "createdAt": item.created_at,
          "updatedAt": item.updated_at,
        });
        img_map.entry(item.location_id).or_default().push(value);
    }

    let characters = characters
        .into_iter()
        .map(|item| {
            let custom_voice_url = media::to_public_media_url(item.custom_voice_url.as_deref())
                .or(item.custom_voice_url);
            json!({
              "id": item.id,
              "novelPromotionProjectId": item.novel_promotion_project_id,
              "name": item.name,
              "aliases": parse_json_str(item.aliases.as_deref()),
              "profileData": parse_json_str(item.profile_data.as_deref()),
              "profileConfirmed": item.profile_confirmed,
              "customVoiceUrl": custom_voice_url,
              "customVoiceMediaId": item.custom_voice_media_id,
              "voiceId": item.voice_id,
              "voiceType": item.voice_type,
              "introduction": item.introduction,
              "sourceGlobalCharacterId": item.source_global_character_id,
              "appearances": app_map.remove(&item.id).unwrap_or_default(),
              "createdAt": item.created_at,
              "updatedAt": item.updated_at,
            })
        })
        .collect::<Vec<_>>();

    let locations = locations
        .into_iter()
        .map(|item| {
            json!({
              "id": item.id,
              "novelPromotionProjectId": item.novel_promotion_project_id,
              "name": item.name,
              "summary": item.summary,
              "sourceGlobalLocationId": item.source_global_location_id,
              "selectedImageId": item.selected_image_id,
              "images": img_map.remove(&item.id).unwrap_or_default(),
              "createdAt": item.created_at,
              "updatedAt": item.updated_at,
            })
        })
        .collect::<Vec<_>>();

    Ok(Json(json!({
      "characters": characters,
      "locations": locations,
    })))
}

async fn handle_episodes_get(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;
    let novel_id = get_novel_id(state, project_id).await?;

    let rows = sqlx::query_as::<_, EpisodeRow>(
        "SELECT id, novelPromotionProjectId, episodeNumber, name, description, novelText, audioUrl, audioMediaId, srtContent, speakerVoices, createdAt, updatedAt FROM novel_promotion_episodes WHERE novelPromotionProjectId = ? ORDER BY episodeNumber ASC",
    )
    .bind(&novel_id)
    .fetch_all(&state.mysql)
    .await?;
    let episodes = rows.into_iter().map(episode_to_json).collect::<Vec<_>>();

    Ok(Json(json!({ "episodes": episodes })))
}

async fn handle_episodes_post(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
    body: Value,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;
    let novel_id = get_novel_id(state, project_id).await?;

    let episode_number: i32 = if let Some(value) = body_i32(&body, "episodeNumber") {
        value
    } else {
        let last: Option<(i32,)> = sqlx::query_as(
            "SELECT episodeNumber FROM novel_promotion_episodes WHERE novelPromotionProjectId = ? ORDER BY episodeNumber DESC LIMIT 1",
        )
        .bind(&novel_id)
        .fetch_optional(&state.mysql)
        .await?;
        last.map(|item| item.0 + 1).unwrap_or(1)
    };

    let name =
        body_string(&body, "name").ok_or_else(|| AppError::invalid_params("name is required"))?;

    let episode_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO novel_promotion_episodes (id, novelPromotionProjectId, episodeNumber, name, description, novelText, audioUrl, srtContent, speakerVoices, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
    )
    .bind(&episode_id)
    .bind(&novel_id)
    .bind(episode_number)
    .bind(name)
    .bind(body_string(&body, "description"))
    .bind(body_string(&body, "novelText"))
    .bind(body_string(&body, "audioUrl"))
    .bind(body_string(&body, "srtContent"))
    .bind(normalize_optional_json(body.get("speakerVoices").cloned()))
    .execute(&state.mysql)
    .await?;

    sqlx::query(
        "UPDATE novel_promotion_projects SET lastEpisodeId = ?, updatedAt = NOW(3) WHERE id = ?",
    )
    .bind(&episode_id)
    .bind(&novel_id)
    .execute(&state.mysql)
    .await?;

    let row = sqlx::query_as::<_, EpisodeRow>(
        "SELECT id, novelPromotionProjectId, episodeNumber, name, description, novelText, audioUrl, audioMediaId, srtContent, speakerVoices, createdAt, updatedAt FROM novel_promotion_episodes WHERE id = ? LIMIT 1",
    )
    .bind(&episode_id)
    .fetch_one(&state.mysql)
    .await?;

    Ok(Json(json!({ "episode": episode_to_json(row) })))
}

async fn handle_episode_get(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
    episode_id: &str,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;
    let novel_id = get_novel_id(state, project_id).await?;

    let row = sqlx::query_as::<_, EpisodeRow>(
        "SELECT id, novelPromotionProjectId, episodeNumber, name, description, novelText, audioUrl, audioMediaId, srtContent, speakerVoices, createdAt, updatedAt FROM novel_promotion_episodes WHERE id = ? AND novelPromotionProjectId = ? LIMIT 1",
    )
    .bind(episode_id)
    .bind(&novel_id)
    .fetch_optional(&state.mysql)
    .await?
    .ok_or_else(|| AppError::not_found("episode not found"))?;

    sqlx::query(
        "UPDATE novel_promotion_projects SET lastEpisodeId = ?, updatedAt = NOW(3) WHERE id = ?",
    )
    .bind(episode_id)
    .bind(&novel_id)
    .execute(&state.mysql)
    .await?;

    Ok(Json(json!({ "episode": episode_to_json(row) })))
}

async fn handle_episode_patch(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
    episode_id: &str,
    body: Value,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;
    let novel_id = get_novel_id(state, project_id).await?;

    let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new("UPDATE novel_promotion_episodes SET ");
    let mut separated = qb.separated(", ");
    let mut touched = false;

    for (db_col, body_key) in [
        ("name", "name"),
        ("description", "description"),
        ("novelText", "novelText"),
        ("audioUrl", "audioUrl"),
        ("srtContent", "srtContent"),
    ] {
        if body.get(body_key).is_some() {
            touched = true;
            separated
                .push(format!("{db_col} = "))
                .push_bind_unseparated(body_string(&body, body_key));
        }
    }

    if body.get("speakerVoices").is_some() {
        touched = true;
        separated
            .push("speakerVoices = ")
            .push_bind_unseparated(normalize_optional_json(body.get("speakerVoices").cloned()));
    }

    if body.get("episodeNumber").is_some() {
        touched = true;
        separated
            .push("episodeNumber = ")
            .push_bind_unseparated(body_i32(&body, "episodeNumber"));
    }

    if !touched {
        return Err(AppError::invalid_params("empty update payload"));
    }

    separated.push("updatedAt = NOW(3)");
    qb.push(" WHERE id = ");
    qb.push_bind(episode_id);
    qb.push(" AND novelPromotionProjectId = ");
    qb.push_bind(&novel_id);
    qb.build().execute(&state.mysql).await?;

    handle_episode_get(state, user, project_id, episode_id).await
}

async fn handle_episode_delete(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
    episode_id: &str,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;
    let novel_id = get_novel_id(state, project_id).await?;

    sqlx::query(
        "DELETE FROM novel_promotion_episodes WHERE id = ? AND novelPromotionProjectId = ?",
    )
    .bind(episode_id)
    .bind(&novel_id)
    .execute(&state.mysql)
    .await?;

    Ok(Json(json!({ "success": true })))
}

async fn handle_character_route(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
    method: &str,
    query: &HashMap<String, String>,
    headers: &HeaderMap,
    body: Value,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;
    let novel_id = get_novel_id(state, project_id).await?;

    match method {
        "POST" => {
            let locale = request_locale(Some(&body), Some(headers));
            let name = body_string(&body, "name")
                .ok_or_else(|| AppError::invalid_params("name is required"))?;
            let introduction = body_string(&body, "introduction");
            let character_id = Uuid::new_v4().to_string();
            let description =
                body_string(&body, "description").unwrap_or_else(|| format!("{name} 的角色设定"));
            let initial_image_url =
                body_string(&body, "initialImageUrl").or_else(|| body_string(&body, "imageUrl"));
            let descriptions = json!([description.clone()]);
            let image_urls = if let Some(url) = initial_image_url.clone() {
                json!([url])
            } else {
                json!([])
            };
            let appearance_id = Uuid::new_v4().to_string();

            sqlx::query(
                "INSERT INTO novel_promotion_characters (id, novelPromotionProjectId, name, aliases, profileData, introduction, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
            )
            .bind(&character_id)
            .bind(&novel_id)
            .bind(&name)
            .bind(normalize_optional_json(body.get("aliases").cloned()))
            .bind(normalize_optional_json(body.get("profileData").cloned()))
            .bind(introduction)
            .execute(&state.mysql)
            .await?;

            sqlx::query(
                "INSERT INTO character_appearances (id, characterId, appearanceIndex, changeReason, description, descriptions, imageUrl, imageUrls, previousImageUrls, createdAt, updatedAt) VALUES (?, ?, 0, ?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
            )
            .bind(&appearance_id)
            .bind(&character_id)
            .bind(localized_msg(locale, "初始形象", "Default Appearance"))
            .bind(description)
            .bind(normalize_optional_json(Some(descriptions)))
            .bind(initial_image_url)
            .bind(normalize_optional_json(Some(image_urls)))
            .bind(normalize_optional_json(Some(json!([]))))
            .execute(&state.mysql)
            .await?;

            let accept_language = headers
                .get(header::ACCEPT_LANGUAGE)
                .and_then(|raw| raw.to_str().ok());

            let mut background_meta = body
                .get("meta")
                .cloned()
                .unwrap_or_else(|| json!({ "source": "character-post" }));
            if !background_meta.is_object() {
                background_meta = json!({ "source": "character-post" });
            }
            if let Some(meta) = background_meta.as_object_mut() {
                meta.entry("locale".to_string())
                    .or_insert_with(|| Value::String(locale.to_string()));
            }

            let mut reference_image_urls = body_string_array(&body, "referenceImageUrls", 5);
            if reference_image_urls.is_empty()
                && let Some(single_reference) = body_string(&body, "referenceImageUrl")
            {
                reference_image_urls.push(single_reference);
            }
            let generate_from_reference =
                body_bool(&body, "generateFromReference").unwrap_or(false);

            if generate_from_reference && !reference_image_urls.is_empty() {
                let mut payload = json!({
                  "referenceImageUrls": reference_image_urls,
                  "characterName": name,
                  "characterId": character_id,
                  "appearanceId": appearance_id,
                  "isBackgroundJob": true,
                  "meta": background_meta.clone(),
                });
                if let Some(object) = payload.as_object_mut() {
                    object.insert("locale".to_string(), Value::String(locale.to_string()));
                }
                if let Some(art_style) = body_string(&body, "artStyle")
                    && let Some(object) = payload.as_object_mut()
                {
                    object.insert("artStyle".to_string(), Value::String(art_style));
                }
                if let Some(custom_description) = body_string(&body, "customDescription")
                    && let Some(object) = payload.as_object_mut()
                {
                    object.insert(
                        "customDescription".to_string(),
                        Value::String(custom_description),
                    );
                }

                if let Err(error) = submit_novel_task(
                    state,
                    user,
                    project_id,
                    "reference_to_character",
                    "character",
                    &character_id,
                    payload,
                    None,
                    accept_language,
                )
                .await
                {
                    warn!(
                        project_id,
                        character_id,
                        error = %error,
                        "failed to submit background reference_to_character task"
                    );
                }
            } else if body
                .get("description")
                .and_then(Value::as_str)
                .map(|item| !item.trim().is_empty())
                .unwrap_or(false)
            {
                let mut payload = json!({
                  "characterId": character_id,
                  "appearanceId": appearance_id,
                  "appearanceIndex": 0,
                  "type": "character",
                  "id": character_id,
                  "meta": background_meta,
                });
                if let Some(object) = payload.as_object_mut() {
                    object.insert("locale".to_string(), Value::String(locale.to_string()));
                }
                if let Some(art_style) = body_string(&body, "artStyle")
                    && let Some(object) = payload.as_object_mut()
                {
                    object.insert("artStyle".to_string(), Value::String(art_style));
                }

                if let Err(error) = submit_novel_task(
                    state,
                    user,
                    project_id,
                    "image_character",
                    "character",
                    &character_id,
                    payload,
                    None,
                    accept_language,
                )
                .await
                {
                    warn!(
                        project_id,
                        character_id,
                        error = %error,
                        "failed to submit background image_character task"
                    );
                }
            }

            Ok(Json(json!({
              "success": true,
              "character": { "id": character_id },
            })))
        }
        "PATCH" => {
            let character_id = body_string(&body, "characterId")
                .ok_or_else(|| AppError::invalid_params("characterId is required"))?;
            let mut qb: QueryBuilder<'_, MySql> =
                QueryBuilder::new("UPDATE novel_promotion_characters SET ");
            let mut separated = qb.separated(", ");
            let mut touched = false;

            for (db_col, key) in [
                ("name", "name"),
                ("introduction", "introduction"),
                ("voiceId", "voiceId"),
                ("voiceType", "voiceType"),
                ("customVoiceUrl", "customVoiceUrl"),
                ("customVoiceMediaId", "customVoiceMediaId"),
            ] {
                if body.get(key).is_some() {
                    touched = true;
                    separated
                        .push(format!("{db_col} = "))
                        .push_bind_unseparated(body_string(&body, key));
                }
            }

            if body.get("aliases").is_some() {
                touched = true;
                separated
                    .push("aliases = ")
                    .push_bind_unseparated(normalize_optional_json(body.get("aliases").cloned()));
            }
            if body.get("profileData").is_some() {
                touched = true;
                separated
                    .push("profileData = ")
                    .push_bind_unseparated(normalize_optional_json(
                        body.get("profileData").cloned(),
                    ));
            }

            if !touched {
                return Err(AppError::invalid_params("empty update payload"));
            }

            separated.push("updatedAt = NOW(3)");
            qb.push(" WHERE id = ");
            qb.push_bind(&character_id);
            qb.push(" AND novelPromotionProjectId = ");
            qb.push_bind(&novel_id);
            qb.build().execute(&state.mysql).await?;

            Ok(Json(
                json!({ "success": true, "character": { "id": character_id } }),
            ))
        }
        "DELETE" => {
            let character_id = body_string(&body, "characterId")
                .or_else(|| query_string(query, "characterId"))
                .or_else(|| query_string(query, "id"))
                .ok_or_else(|| AppError::invalid_params("characterId is required"))?;
            sqlx::query("DELETE FROM novel_promotion_characters WHERE id = ? AND novelPromotionProjectId = ?")
                .bind(&character_id)
                .bind(&novel_id)
                .execute(&state.mysql)
                .await?;
            Ok(Json(json!({ "success": true })))
        }
        _ => Err(AppError::invalid_params(
            "unsupported method for /character",
        )),
    }
}

async fn handle_location_route(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
    method: &str,
    query: &HashMap<String, String>,
    headers: &HeaderMap,
    body: Value,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;
    let novel_id = get_novel_id(state, project_id).await?;

    match method {
        "POST" => {
            let locale = request_locale(Some(&body), Some(headers));
            let name = body_string(&body, "name")
                .ok_or_else(|| AppError::invalid_params("name is required"))?;
            let description = body_string(&body, "description")
                .ok_or_else(|| AppError::invalid_params("description is required"))?;
            let location_id = Uuid::new_v4().to_string();
            let initial_image_id = Uuid::new_v4().to_string();

            if let Some(art_style) = body_string(&body, "artStyle")
                && let Some(prompt) = map_art_style_prompt(&art_style, locale)
            {
                sqlx::query(
                    "UPDATE novel_promotion_projects SET artStylePrompt = ?, updatedAt = NOW(3) WHERE id = ?",
                )
                .bind(prompt)
                .bind(&novel_id)
                .execute(&state.mysql)
                .await?;
            }

            sqlx::query(
                "INSERT INTO novel_promotion_locations (id, novelPromotionProjectId, name, summary, createdAt, updatedAt) VALUES (?, ?, ?, ?, NOW(3), NOW(3))",
            )
            .bind(&location_id)
            .bind(&novel_id)
            .bind(name)
            .bind(body_string(&body, "summary"))
            .execute(&state.mysql)
            .await?;

            let clean_description = normalize_location_description(&description);
            sqlx::query(
                "INSERT INTO location_images (id, locationId, imageIndex, description, imageUrl, isSelected, createdAt, updatedAt) VALUES (?, ?, 0, ?, ?, true, NOW(3), NOW(3))",
            )
            .bind(&initial_image_id)
            .bind(&location_id)
            .bind(clean_description)
            .bind(body_string(&body, "imageUrl"))
            .execute(&state.mysql)
            .await?;
            sqlx::query(
                "UPDATE novel_promotion_locations SET selectedImageId = ?, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(&initial_image_id)
            .bind(&location_id)
            .execute(&state.mysql)
            .await?;

            let accept_language = headers
                .get(header::ACCEPT_LANGUAGE)
                .and_then(|raw| raw.to_str().ok());
            let mut payload = json!({
              "type": "location",
              "id": location_id,
              "locationId": location_id,
              "meta": body.get("meta").cloned().unwrap_or_else(|| json!({ "source": "location-post" })),
            });
            if let Some(locale) = read_task_locale_from_body(&body)
                .or_else(|| read_task_locale_from_headers(headers))
                .map(str::to_string)
                .or_else(|| Some("zh".to_string()))
                && let Some(object) = payload.as_object_mut()
            {
                object.insert("locale".to_string(), Value::String(locale.clone()));
                if let Some(meta) = object.get_mut("meta").and_then(Value::as_object_mut) {
                    meta.entry("locale".to_string())
                        .or_insert_with(|| Value::String(locale));
                }
            }

            if let Err(error) = submit_novel_task(
                state,
                user,
                project_id,
                "image_location",
                "location",
                &location_id,
                payload,
                None,
                accept_language,
            )
            .await
            {
                warn!(
                    project_id,
                    location_id,
                    error = %error,
                    "failed to submit background image_location task"
                );
            }

            Ok(Json(json!({
              "success": true,
              "location": { "id": location_id },
            })))
        }
        "PATCH" => {
            if body.get("locationId").is_some()
                && body.get("imageIndex").is_some()
                && body.get("description").is_some()
            {
                let location_id = body_string(&body, "locationId").unwrap_or_default();
                let image_index = body_i32(&body, "imageIndex").unwrap_or(0);
                let description = body_string(&body, "description")
                    .map(|item| normalize_location_description(&item));
                sqlx::query(
                    "UPDATE location_images SET description = ?, updatedAt = NOW(3) WHERE locationId = ? AND imageIndex = ?",
                )
                .bind(description)
                .bind(&location_id)
                .bind(image_index)
                .execute(&state.mysql)
                .await?;
                return Ok(Json(json!({ "success": true })));
            }

            let location_id = body_string(&body, "locationId")
                .ok_or_else(|| AppError::invalid_params("locationId is required"))?;
            let mut qb: QueryBuilder<'_, MySql> =
                QueryBuilder::new("UPDATE novel_promotion_locations SET ");
            let mut separated = qb.separated(", ");
            let mut touched = false;

            if body.get("name").is_some() {
                touched = true;
                separated
                    .push("name = ")
                    .push_bind_unseparated(body_string(&body, "name"));
            }
            if body.get("summary").is_some() {
                touched = true;
                separated
                    .push("summary = ")
                    .push_bind_unseparated(body_string(&body, "summary"));
            }
            if body.get("selectedImageId").is_some() {
                touched = true;
                separated
                    .push("selectedImageId = ")
                    .push_bind_unseparated(body_string(&body, "selectedImageId"));
            }

            if !touched {
                return Err(AppError::invalid_params("empty update payload"));
            }

            separated.push("updatedAt = NOW(3)");
            qb.push(" WHERE id = ");
            qb.push_bind(&location_id);
            qb.push(" AND novelPromotionProjectId = ");
            qb.push_bind(&novel_id);
            qb.build().execute(&state.mysql).await?;

            Ok(Json(json!({
              "success": true,
              "location": { "id": location_id },
              "image": Value::Null,
            })))
        }
        "DELETE" => {
            let location_id = body_string(&body, "locationId")
                .or_else(|| query_string(query, "locationId"))
                .or_else(|| query_string(query, "id"))
                .ok_or_else(|| AppError::invalid_params("locationId is required"))?;
            sqlx::query("DELETE FROM novel_promotion_locations WHERE id = ? AND novelPromotionProjectId = ?")
                .bind(&location_id)
                .bind(&novel_id)
                .execute(&state.mysql)
                .await?;
            Ok(Json(json!({ "success": true })))
        }
        _ => Err(AppError::invalid_params("unsupported method for /location")),
    }
}

async fn handle_voice_lines(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
    method: &str,
    query: &HashMap<String, String>,
    body: Value,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;
    let novel_id = get_novel_id(state, project_id).await?;

    match method {
        "GET" => {
            if query.get("speakersOnly").is_some_and(|value| value == "1") {
                let rows: Vec<(String,)> = sqlx::query_as(
                    "SELECT DISTINCT speaker
                     FROM novel_promotion_voice_lines
                     WHERE episodeId IN (
                       SELECT id FROM novel_promotion_episodes WHERE novelPromotionProjectId = ?
                     )
                     ORDER BY speaker ASC",
                )
                .bind(&novel_id)
                .fetch_all(&state.mysql)
                .await?;
                let speakers = rows.into_iter().map(|item| item.0).collect::<Vec<_>>();
                return Ok(Json(json!({ "speakers": speakers })));
            }

            let episode_id = query_string(query, "episodeId")
                .ok_or_else(|| AppError::invalid_params("episodeId is required"))?;

            let episode_exists: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM novel_promotion_episodes WHERE id = ? AND novelPromotionProjectId = ? LIMIT 1",
            )
            .bind(&episode_id)
            .bind(&novel_id)
            .fetch_optional(&state.mysql)
            .await?;
            if episode_exists.is_none() {
                return Err(AppError::not_found("episode not found"));
            }

            let rows = sqlx::query_as::<_, VoiceLineRow>(
                "SELECT id, episodeId, lineIndex, speaker, content, voicePresetId, audioUrl, audioMediaId, emotionPrompt, emotionStrength, matchedPanelIndex, matchedStoryboardId, audioDuration, matchedPanelId, createdAt, updatedAt FROM novel_promotion_voice_lines WHERE episodeId = ? ORDER BY lineIndex ASC",
            )
            .bind(&episode_id)
            .fetch_all(&state.mysql)
            .await?;
            let mut voice_lines = Vec::with_capacity(rows.len());
            for row in &rows {
                voice_lines.push(with_voice_line_media(state, row).await?);
            }
            let mut speaker_counts: HashMap<String, usize> = HashMap::new();
            for row in &rows {
                let entry = speaker_counts.entry(row.speaker.clone()).or_insert(0);
                *entry += 1;
            }
            let speakers = speaker_counts.keys().cloned().collect::<Vec<_>>();

            Ok(Json(json!({
              "voiceLines": voice_lines,
              "count": speaker_counts.values().sum::<usize>(),
              "speakers": speakers,
              "speakerStats": speaker_counts,
            })))
        }
        "POST" => {
            let episode_id = body_string(&body, "episodeId")
                .ok_or_else(|| AppError::invalid_params("episodeId is required"))?;
            let speaker = body_string(&body, "speaker")
                .ok_or_else(|| AppError::invalid_params("speaker is required"))?;
            let content = body_string(&body, "content")
                .ok_or_else(|| AppError::invalid_params("content is required"))?;
            let matched_panel_id = body_string(&body, "matchedPanelId");
            let mut matched_storyboard_id = body_string(&body, "matchedStoryboardId");
            let mut matched_panel_index = body_i32(&body, "matchedPanelIndex");

            let episode_exists: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM novel_promotion_episodes WHERE id = ? AND novelPromotionProjectId = ? LIMIT 1",
            )
            .bind(&episode_id)
            .bind(&novel_id)
            .fetch_optional(&state.mysql)
            .await?;
            if episode_exists.is_none() {
                return Err(AppError::not_found("episode not found"));
            }

            if let Some(panel_id) = matched_panel_id.clone() {
                let matched_panel: Option<(String, i32, String)> = sqlx::query_as(
                    "SELECT p.storyboardId, p.panelIndex, s.episodeId
                     FROM novel_promotion_panels p
                     INNER JOIN novel_promotion_storyboards s ON s.id = p.storyboardId
                     WHERE p.id = ?
                     LIMIT 1",
                )
                .bind(&panel_id)
                .fetch_optional(&state.mysql)
                .await?;
                let Some((storyboard_id, panel_index, panel_episode_id)) = matched_panel else {
                    return Err(AppError::not_found("matched panel not found"));
                };
                if panel_episode_id != episode_id {
                    return Err(AppError::invalid_params(
                        "matchedPanelId does not belong to episodeId",
                    ));
                }
                matched_storyboard_id = Some(storyboard_id);
                matched_panel_index = Some(panel_index);
            }

            let max: Option<(i32,)> = sqlx::query_as(
                "SELECT lineIndex FROM novel_promotion_voice_lines WHERE episodeId = ? ORDER BY lineIndex DESC LIMIT 1",
            )
            .bind(&episode_id)
            .fetch_optional(&state.mysql)
            .await?;

            let line_index = max.map(|item| item.0 + 1).unwrap_or(1);
            let line_id = Uuid::new_v4().to_string();
            let audio_url = body_string(&body, "audioUrl");
            let audio_media_id = if let Some(media_id) = body_string(&body, "audioMediaId") {
                Some(media_id)
            } else if let Some(url) = audio_url.as_deref() {
                resolve_media_object_from_value(state, url)
                    .await?
                    .map(|item| item.0)
            } else {
                None
            };
            sqlx::query(
                "INSERT INTO novel_promotion_voice_lines (id, episodeId, lineIndex, speaker, content, voicePresetId, audioUrl, audioMediaId, emotionPrompt, emotionStrength, matchedPanelIndex, matchedStoryboardId, audioDuration, matchedPanelId, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
            )
            .bind(&line_id)
            .bind(&episode_id)
            .bind(line_index)
            .bind(speaker)
            .bind(content)
            .bind(body_string(&body, "voicePresetId"))
            .bind(audio_url)
            .bind(audio_media_id)
            .bind(body_string(&body, "emotionPrompt"))
            .bind(body_f64(&body, "emotionStrength"))
            .bind(matched_panel_index)
            .bind(matched_storyboard_id)
            .bind(body_i32(&body, "audioDuration"))
            .bind(matched_panel_id)
            .execute(&state.mysql)
            .await?;
            let voice_line = sqlx::query_as::<_, VoiceLineRow>(
                "SELECT id, episodeId, lineIndex, speaker, content, voicePresetId, audioUrl, audioMediaId, emotionPrompt, emotionStrength, matchedPanelIndex, matchedStoryboardId, audioDuration, matchedPanelId, createdAt, updatedAt FROM novel_promotion_voice_lines WHERE id = ? LIMIT 1",
            )
            .bind(&line_id)
            .fetch_one(&state.mysql)
            .await?;
            let voice_line_json = with_voice_line_media(state, &voice_line).await?;

            Ok(Json(json!({
              "success": true,
              "voiceLine": voice_line_json,
            })))
        }
        "PATCH" => {
            if let Some(line_id) = body_string(&body, "lineId") {
                let line_scope: Option<(String,)> = sqlx::query_as(
                    "SELECT l.episodeId
                     FROM novel_promotion_voice_lines l
                     INNER JOIN novel_promotion_episodes e ON e.id = l.episodeId
                     WHERE l.id = ? AND e.novelPromotionProjectId = ?
                     LIMIT 1",
                )
                .bind(&line_id)
                .bind(&novel_id)
                .fetch_optional(&state.mysql)
                .await?;
                let Some((line_episode_id,)) = line_scope else {
                    return Err(AppError::not_found("voice line not found"));
                };

                let mut qb: QueryBuilder<'_, MySql> =
                    QueryBuilder::new("UPDATE novel_promotion_voice_lines SET ");
                let mut separated = qb.separated(", ");
                let mut touched = false;

                if let Some(value) = body.get("speaker") {
                    let Some(speaker) = value
                        .as_str()
                        .map(|item| item.trim().to_string())
                        .filter(|item| !item.is_empty())
                    else {
                        return Err(AppError::invalid_params("speaker cannot be empty"));
                    };
                    touched = true;
                    separated.push("speaker = ").push_bind_unseparated(speaker);
                }

                if let Some(value) = body.get("content") {
                    let Some(content) = value
                        .as_str()
                        .map(|item| item.trim().to_string())
                        .filter(|item| !item.is_empty())
                    else {
                        return Err(AppError::invalid_params("content cannot be empty"));
                    };
                    touched = true;
                    separated.push("content = ").push_bind_unseparated(content);
                }

                for key in ["voicePresetId", "emotionPrompt"] {
                    if let Some(value) = body.get(key) {
                        touched = true;
                        separated.push(format!("{key} = ")).push_bind_unseparated(
                            value
                                .as_str()
                                .map(|item| item.trim().to_string())
                                .filter(|item| !item.is_empty()),
                        );
                    }
                }

                if let Some(value) = body.get("audioUrl") {
                    let audio_url = value
                        .as_str()
                        .map(|item| item.trim().to_string())
                        .filter(|item| !item.is_empty());
                    let resolved_media_id = if let Some(url) = audio_url.as_deref() {
                        resolve_media_object_from_value(state, url)
                            .await?
                            .map(|item| item.0)
                    } else {
                        None
                    };

                    touched = true;
                    separated
                        .push("audioUrl = ")
                        .push_bind_unseparated(audio_url);
                    separated
                        .push("audioMediaId = ")
                        .push_bind_unseparated(resolved_media_id);
                }

                for key in ["lineIndex", "audioDuration"] {
                    if let Some(value) = body.get(key) {
                        touched = true;
                        separated
                            .push(format!("{key} = "))
                            .push_bind_unseparated(parse_nullable_i32_field(value, key)?);
                    }
                }

                if let Some(value) = body.get("emotionStrength") {
                    touched = true;
                    separated
                        .push("emotionStrength = ")
                        .push_bind_unseparated(parse_nullable_f64_field(value, "emotionStrength")?);
                }

                if body.get("matchedPanelId").is_some() {
                    let matched_panel_id = body_string(&body, "matchedPanelId");
                    let mut matched_storyboard_id = body_string(&body, "matchedStoryboardId");
                    let mut matched_panel_index = match body.get("matchedPanelIndex") {
                        Some(value) => parse_nullable_i32_field(value, "matchedPanelIndex")?,
                        None => None,
                    };

                    if let Some(panel_id) = matched_panel_id.clone() {
                        let matched_panel: Option<(String, i32, String)> = sqlx::query_as(
                            "SELECT p.storyboardId, p.panelIndex, s.episodeId
                             FROM novel_promotion_panels p
                             INNER JOIN novel_promotion_storyboards s ON s.id = p.storyboardId
                             WHERE p.id = ?
                             LIMIT 1",
                        )
                        .bind(&panel_id)
                        .fetch_optional(&state.mysql)
                        .await?;
                        let Some((storyboard_id, panel_index, panel_episode_id)) = matched_panel
                        else {
                            return Err(AppError::not_found("matched panel not found"));
                        };
                        if panel_episode_id != line_episode_id {
                            return Err(AppError::invalid_params(
                                "matchedPanelId does not belong to voice line episode",
                            ));
                        }
                        matched_storyboard_id = Some(storyboard_id);
                        matched_panel_index = Some(panel_index);
                    }

                    touched = true;
                    separated
                        .push("matchedPanelId = ")
                        .push_bind_unseparated(matched_panel_id);
                    separated
                        .push("matchedStoryboardId = ")
                        .push_bind_unseparated(matched_storyboard_id);
                    separated
                        .push("matchedPanelIndex = ")
                        .push_bind_unseparated(matched_panel_index);
                } else if let Some(value) = body.get("matchedStoryboardId") {
                    touched = true;
                    separated
                        .push("matchedStoryboardId = ")
                        .push_bind_unseparated(
                            value
                                .as_str()
                                .map(|item| item.trim().to_string())
                                .filter(|item| !item.is_empty()),
                        );
                } else if let Some(value) = body.get("matchedPanelIndex") {
                    touched = true;
                    separated
                        .push("matchedPanelIndex = ")
                        .push_bind_unseparated(parse_nullable_i32_field(
                            value,
                            "matchedPanelIndex",
                        )?);
                }

                if !touched {
                    return Err(AppError::invalid_params("empty update payload"));
                }

                separated.push("updatedAt = NOW(3)");
                qb.push(" WHERE id = ");
                qb.push_bind(&line_id);
                let updated = qb.build().execute(&state.mysql).await?.rows_affected();

                let voice_line = sqlx::query_as::<_, VoiceLineRow>(
                    "SELECT id, episodeId, lineIndex, speaker, content, voicePresetId, audioUrl, audioMediaId, emotionPrompt, emotionStrength, matchedPanelIndex, matchedStoryboardId, audioDuration, matchedPanelId, createdAt, updatedAt FROM novel_promotion_voice_lines WHERE id = ? LIMIT 1",
                )
                .bind(&line_id)
                .fetch_one(&state.mysql)
                .await?;
                let voice_line_json = with_voice_line_media(state, &voice_line).await?;
                let speaker = voice_line.speaker.clone();
                let voice_preset_id = voice_line.voice_preset_id.clone();
                return Ok(Json(json!({
                  "success": true,
                  "updatedCount": updated,
                  "voiceLine": voice_line_json,
                  "speaker": speaker,
                  "voicePresetId": voice_preset_id,
                })));
            }

            let speaker = body_string(&body, "speaker");
            let episode_id = body_string(&body, "episodeId");
            if let (Some(speaker), Some(episode_id)) = (speaker, episode_id) {
                let episode_exists: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM novel_promotion_episodes WHERE id = ? AND novelPromotionProjectId = ? LIMIT 1",
                )
                .bind(&episode_id)
                .bind(&novel_id)
                .fetch_optional(&state.mysql)
                .await?;
                if episode_exists.is_none() {
                    return Err(AppError::not_found("episode not found"));
                }

                let updated = sqlx::query(
                    "UPDATE novel_promotion_voice_lines SET voicePresetId = ?, updatedAt = NOW(3) WHERE episodeId = ? AND speaker = ?",
                )
                .bind(body_string(&body, "voicePresetId"))
                .bind(&episode_id)
                .bind(&speaker)
                .execute(&state.mysql)
                .await?
                .rows_affected();

                return Ok(Json(json!({
                  "success": true,
                  "updatedCount": updated,
                  "speaker": speaker,
                  "voicePresetId": body_string(&body, "voicePresetId"),
                })));
            }

            Err(AppError::invalid_params(
                "lineId or (speaker + episodeId) is required",
            ))
        }
        "DELETE" => {
            let line_id = body_string(&body, "lineId")
                .or_else(|| query_string(query, "lineId"))
                .ok_or_else(|| AppError::invalid_params("lineId is required"))?;
            let line: Option<(String, i32)> = sqlx::query_as(
                "SELECT episodeId, lineIndex FROM novel_promotion_voice_lines WHERE id = ? LIMIT 1",
            )
            .bind(&line_id)
            .fetch_optional(&state.mysql)
            .await?;
            let Some((episode_id, deleted_line_index)) = line else {
                return Err(AppError::not_found("voice line not found"));
            };

            let episode_exists: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM novel_promotion_episodes WHERE id = ? AND novelPromotionProjectId = ? LIMIT 1",
            )
            .bind(&episode_id)
            .bind(&novel_id)
            .fetch_optional(&state.mysql)
            .await?;
            if episode_exists.is_none() {
                return Err(AppError::not_found("episode not found"));
            }

            sqlx::query("DELETE FROM novel_promotion_voice_lines WHERE id = ?")
                .bind(&line_id)
                .execute(&state.mysql)
                .await?;

            sqlx::query(
                "UPDATE novel_promotion_voice_lines
                 SET lineIndex = lineIndex - 1, updatedAt = NOW(3)
                 WHERE episodeId = ? AND lineIndex > ?",
            )
            .bind(&episode_id)
            .bind(deleted_line_index)
            .execute(&state.mysql)
            .await?;

            let remaining: Option<(i64,)> = sqlx::query_as(
                "SELECT COUNT(*) FROM novel_promotion_voice_lines WHERE episodeId = ?",
            )
            .bind(&episode_id)
            .fetch_optional(&state.mysql)
            .await?;
            Ok(Json(json!({
              "success": true,
              "deletedId": line_id,
              "remainingCount": remaining.map(|item| item.0).unwrap_or(0),
            })))
        }
        _ => Err(AppError::invalid_params(
            "unsupported method for /voice-lines",
        )),
    }
}

async fn handle_editor(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
    method: &str,
    query: &HashMap<String, String>,
    body: Value,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;
    let novel_id = get_novel_id(state, project_id).await?;

    let episode_id = query
        .get("episodeId")
        .cloned()
        .or_else(|| body_string(&body, "episodeId"))
        .ok_or_else(|| AppError::invalid_params("episodeId is required"))?;

    let episode_exists: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM novel_promotion_episodes WHERE id = ? AND novelPromotionProjectId = ? LIMIT 1",
    )
    .bind(&episode_id)
    .bind(&novel_id)
    .fetch_optional(&state.mysql)
    .await?;
    if episode_exists.is_none() {
        return Err(AppError::not_found("episode not found"));
    }

    match method {
        "GET" => {
            let row = sqlx::query_as::<_, EditorRow>(
                "SELECT id, episodeId, projectData, renderStatus, renderTaskId, outputUrl, createdAt, updatedAt FROM video_editor_projects WHERE episodeId = ? LIMIT 1",
            )
            .bind(&episode_id)
            .fetch_optional(&state.mysql)
            .await?;

            if let Some(item) = row {
                let project_data =
                    parse_json_str(Some(&item.project_data)).unwrap_or_else(|| json!({}));
                Ok(Json(json!({
                  "projectData": project_data,
                  "id": item.id,
                  "episodeId": item.episode_id,
                  "renderStatus": item.render_status,
                  "outputUrl": item.output_url,
                  "updatedAt": item.updated_at,
                })))
            } else {
                Ok(Json(json!({
                  "projectData": null,
                  "id": null,
                  "episodeId": episode_id,
                  "renderStatus": null,
                  "outputUrl": null,
                  "updatedAt": null,
                })))
            }
        }
        "PUT" => {
            let project_data = body
                .get("projectData")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let serialized = serde_json::to_string(&project_data)
                .map_err(|err| AppError::invalid_params(format!("invalid projectData: {err}")))?;

            let existing: Option<(String,)> =
                sqlx::query_as("SELECT id FROM video_editor_projects WHERE episodeId = ? LIMIT 1")
                    .bind(&episode_id)
                    .fetch_optional(&state.mysql)
                    .await?;

            if let Some((id,)) = existing {
                sqlx::query(
                    "UPDATE video_editor_projects SET projectData = ?, renderStatus = ?, renderTaskId = ?, outputUrl = ?, updatedAt = NOW(3) WHERE id = ?",
                )
                .bind(&serialized)
                .bind(body_string(&body, "renderStatus"))
                .bind(body_string(&body, "renderTaskId"))
                .bind(body_string(&body, "outputUrl"))
                .bind(&id)
                .execute(&state.mysql)
                .await?;
            } else {
                sqlx::query(
                    "INSERT INTO video_editor_projects (id, episodeId, projectData, renderStatus, renderTaskId, outputUrl, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
                )
                .bind(Uuid::new_v4().to_string())
                .bind(&episode_id)
                .bind(&serialized)
                .bind(body_string(&body, "renderStatus"))
                .bind(body_string(&body, "renderTaskId"))
                .bind(body_string(&body, "outputUrl"))
                .execute(&state.mysql)
                .await?;
            }

            let saved: (String, NaiveDateTime) = sqlx::query_as(
                "SELECT id, updatedAt FROM video_editor_projects WHERE episodeId = ? LIMIT 1",
            )
            .bind(&episode_id)
            .fetch_one(&state.mysql)
            .await?;
            Ok(Json(json!({
              "success": true,
              "id": saved.0,
              "updatedAt": saved.1,
            })))
        }
        "DELETE" => {
            sqlx::query("DELETE FROM video_editor_projects WHERE episodeId = ?")
                .bind(&episode_id)
                .execute(&state.mysql)
                .await?;
            Ok(Json(json!({ "success": true })))
        }
        _ => Err(AppError::invalid_params("unsupported method for /editor")),
    }
}

async fn handle_storyboards(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
    method: &str,
    query: &HashMap<String, String>,
    body: Value,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;
    let novel_id = get_novel_id(state, project_id).await?;

    match method {
        "GET" => {
            let episode_id = query_string(query, "episodeId")
                .ok_or_else(|| AppError::invalid_params("episodeId is required"))?;
            let episode_exists: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM novel_promotion_episodes WHERE id = ? AND novelPromotionProjectId = ? LIMIT 1",
            )
            .bind(&episode_id)
            .bind(&novel_id)
            .fetch_optional(&state.mysql)
            .await?;
            if episode_exists.is_none() {
                return Err(AppError::not_found("episode not found"));
            }

            let storyboards = sqlx::query_as::<_, StoryboardRow>(
                "SELECT id, episodeId, clipId, storyboardImageUrl, panelCount, storyboardTextJson, imageHistory, candidateImages, lastError, photographyPlan, createdAt, updatedAt FROM novel_promotion_storyboards WHERE episodeId = ? ORDER BY createdAt ASC",
            )
            .bind(&episode_id)
            .fetch_all(&state.mysql)
            .await?;

            let mut enriched = Vec::with_capacity(storyboards.len());
            for item in storyboards {
                let clip = sqlx::query_as::<_, ClipRow>(
                    "SELECT id, episodeId, start, end, duration, summary, location, content, characters, endText, shotCount, startText, screenplay, createdAt, updatedAt FROM novel_promotion_clips WHERE id = ? LIMIT 1",
                )
                .bind(&item.clip_id)
                .fetch_optional(&state.mysql)
                .await?;

                let panels = sqlx::query_as::<_, StoryboardPanelMediaRow>(
                    "SELECT id, storyboardId, panelIndex, panelNumber, shotType, cameraMove, description, location, characters, srtStart, srtEnd, duration, videoPrompt, firstLastFramePrompt, imageUrl, imageMediaId, candidateImages, videoUrl, videoMediaId, lipSyncVideoUrl, sketchImageUrl, previousImageUrl, previousImageMediaId, linkedToNextPanel, actingNotes, photographyRules, createdAt, updatedAt
                     FROM novel_promotion_panels
                     WHERE storyboardId = ?
                     ORDER BY panelIndex ASC",
                )
                .bind(&item.id)
                .fetch_all(&state.mysql)
                .await?;

                let mut value = serde_json::to_value(item).map_err(|err| {
                    AppError::internal(format!("failed to encode storyboard: {err}"))
                })?;
                if let Some(object) = value.as_object_mut() {
                    if let Some(raw_storyboard_image) =
                        object.get("storyboardImageUrl").and_then(Value::as_str)
                        && let Some(public_url) =
                            media::to_public_media_url(Some(raw_storyboard_image))
                    {
                        object.insert("storyboardImageUrl".to_string(), Value::String(public_url));
                    }

                    let mut panel_values = serde_json::to_value(panels).map_err(|err| {
                        AppError::internal(format!("failed to encode panels: {err}"))
                    })?;
                    if let Some(items) = panel_values.as_array_mut() {
                        for panel in items {
                            if let Some(panel_object) = panel.as_object_mut() {
                                for field in [
                                    "imageUrl",
                                    "videoUrl",
                                    "lipSyncVideoUrl",
                                    "sketchImageUrl",
                                    "previousImageUrl",
                                ] {
                                    if let Some(raw_value) =
                                        panel_object.get(field).and_then(Value::as_str)
                                        && let Some(public_url) =
                                            media::to_public_media_url(Some(raw_value))
                                    {
                                        panel_object
                                            .insert(field.to_string(), Value::String(public_url));
                                    }
                                }

                                if let Some(raw_candidates) =
                                    panel_object.get("candidateImages").and_then(Value::as_str)
                                    && let Some(candidate_array) =
                                        parse_json_str(Some(raw_candidates))
                                            .and_then(|value| value.as_array().cloned())
                                {
                                    let normalized = candidate_array
                                        .into_iter()
                                        .map(|item| {
                                            item.as_str()
                                                .and_then(|raw| {
                                                    media::to_public_media_url(Some(raw))
                                                })
                                                .map(Value::String)
                                                .unwrap_or(item)
                                        })
                                        .collect::<Vec<_>>();
                                    if let Ok(serialized) = serde_json::to_string(&normalized) {
                                        panel_object.insert(
                                            "candidateImages".to_string(),
                                            Value::String(serialized),
                                        );
                                    }
                                }
                            }
                        }
                    }

                    object.insert(
                        "clip".to_string(),
                        serde_json::to_value(clip).map_err(|err| {
                            AppError::internal(format!("failed to encode clip: {err}"))
                        })?,
                    );
                    object.insert("panels".to_string(), panel_values);
                }
                enriched.push(value);
            }

            Ok(Json(json!({ "storyboards": enriched })))
        }
        "PATCH" => {
            let storyboard_id = body_string(&body, "storyboardId")
                .ok_or_else(|| AppError::invalid_params("storyboardId is required"))?;
            let updated = sqlx::query(
                "UPDATE novel_promotion_storyboards
                 SET lastError = NULL, updatedAt = NOW(3)
                 WHERE id = ?
                   AND episodeId IN (
                     SELECT id FROM novel_promotion_episodes WHERE novelPromotionProjectId = ?
                   )",
            )
            .bind(&storyboard_id)
            .bind(&novel_id)
            .execute(&state.mysql)
            .await?
            .rows_affected();
            if updated == 0 {
                return Err(AppError::not_found("storyboard not found"));
            }
            Ok(Json(json!({ "success": true })))
        }
        _ => Err(AppError::invalid_params(
            "unsupported method for /storyboards",
        )),
    }
}

async fn handle_panel(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
    method: &str,
    query: &HashMap<String, String>,
    body: Value,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;
    let novel_id = get_novel_id(state, project_id).await?;

    match method {
        "POST" => {
            let storyboard_id = body_string(&body, "storyboardId")
                .ok_or_else(|| AppError::invalid_params("storyboardId is required"))?;
            let storyboard_exists: Option<(String,)> = sqlx::query_as(
                "SELECT s.id
                     FROM novel_promotion_storyboards s
                     INNER JOIN novel_promotion_episodes e ON e.id = s.episodeId
                     WHERE s.id = ? AND e.novelPromotionProjectId = ?
                     LIMIT 1",
            )
            .bind(&storyboard_id)
            .bind(&novel_id)
            .fetch_optional(&state.mysql)
            .await?;
            if storyboard_exists.is_none() {
                return Err(AppError::not_found("storyboard not found"));
            }

            let panel_index = if let Some(index) = body_i32(&body, "panelIndex") {
                index
            } else {
                let max_index: Option<(i32,)> = sqlx::query_as(
                    "SELECT panelIndex FROM novel_promotion_panels WHERE storyboardId = ? ORDER BY panelIndex DESC LIMIT 1",
                )
                .bind(&storyboard_id)
                .fetch_optional(&state.mysql)
                .await?;
                max_index.map(|item| item.0 + 1).unwrap_or(0)
            };
            let panel_number = body_i32(&body, "panelNumber").or(Some(panel_index + 1));
            let srt_start = match body.get("srtStart") {
                Some(value) => parse_nullable_f64_field(value, "srtStart")?,
                None => None,
            };
            let srt_end = match body.get("srtEnd") {
                Some(value) => parse_nullable_f64_field(value, "srtEnd")?,
                None => None,
            };
            let duration = match body.get("duration") {
                Some(value) => parse_nullable_f64_field(value, "duration")?,
                None => None,
            };

            let panel_id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO novel_promotion_panels (id, storyboardId, panelIndex, panelNumber, shotType, cameraMove, description, location, characters, srtStart, srtEnd, duration, imagePrompt, imageUrl, candidateImages, videoPrompt, firstLastFramePrompt, videoUrl, actingNotes, photographyRules, linkedToNextPanel, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
            )
            .bind(&panel_id)
            .bind(&storyboard_id)
            .bind(panel_index)
            .bind(panel_number)
            .bind(body_string(&body, "shotType"))
            .bind(body_string(&body, "cameraMove"))
            .bind(body_string(&body, "description"))
            .bind(body_string(&body, "location"))
            .bind(normalize_optional_json(body.get("characters").cloned()))
            .bind(srt_start)
            .bind(srt_end)
            .bind(duration)
            .bind(body_string(&body, "imagePrompt"))
            .bind(body_string(&body, "imageUrl"))
            .bind(normalize_optional_json(body.get("candidateImages").cloned()))
            .bind(body_string(&body, "videoPrompt"))
            .bind(body_string(&body, "firstLastFramePrompt"))
            .bind(body_string(&body, "videoUrl"))
            .bind(normalize_optional_json(body.get("actingNotes").cloned()))
            .bind(normalize_optional_json(body.get("photographyRules").cloned()))
            .bind(body_bool(&body, "linkedToNextPanel").unwrap_or(false))
            .execute(&state.mysql)
            .await?;

            sqlx::query(
                "UPDATE novel_promotion_storyboards SET panelCount = (SELECT COUNT(*) FROM novel_promotion_panels WHERE storyboardId = ?), updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(&storyboard_id)
            .bind(&storyboard_id)
            .execute(&state.mysql)
            .await?;
            let panel = sqlx::query_as::<_, StoryboardPanelDetailRow>(
                "SELECT id, storyboardId, panelIndex, panelNumber, shotType, cameraMove, description, location, characters, srtStart, srtEnd, duration, videoPrompt, firstLastFramePrompt, imageUrl, candidateImages, linkedToNextPanel, actingNotes, photographyRules, createdAt, updatedAt FROM novel_promotion_panels WHERE id = ? LIMIT 1",
            )
            .bind(&panel_id)
            .fetch_one(&state.mysql)
            .await?;

            Ok(Json(json!({
              "success": true,
              "panel": panel,
            })))
        }
        "PATCH" | "PUT" => {
            let storyboard_id = body_string(&body, "storyboardId");
            let panel_index = match body.get("panelIndex") {
                Some(value) => parse_nullable_i32_field(value, "panelIndex")?,
                None => None,
            };
            let provided_panel_id = body_string(&body, "panelId");

            let panel_id = if let Some(panel_id) = provided_panel_id.clone() {
                let exists: Option<(String,)> = sqlx::query_as(
                    "SELECT p.id
                     FROM novel_promotion_panels p
                     INNER JOIN novel_promotion_storyboards s ON s.id = p.storyboardId
                     INNER JOIN novel_promotion_episodes e ON e.id = s.episodeId
                     WHERE p.id = ? AND e.novelPromotionProjectId = ?
                     LIMIT 1",
                )
                .bind(&panel_id)
                .bind(&novel_id)
                .fetch_optional(&state.mysql)
                .await?;
                exists.map(|item| item.0)
            } else if let (Some(storyboard_id), Some(panel_index)) =
                (storyboard_id.clone(), panel_index)
            {
                let row: Option<(String,)> = sqlx::query_as(
                    "SELECT p.id
                     FROM novel_promotion_panels p
                     INNER JOIN novel_promotion_storyboards s ON s.id = p.storyboardId
                     INNER JOIN novel_promotion_episodes e ON e.id = s.episodeId
                     WHERE p.storyboardId = ? AND p.panelIndex = ? AND e.novelPromotionProjectId = ?
                     LIMIT 1",
                )
                .bind(storyboard_id)
                .bind(panel_index)
                .bind(&novel_id)
                .fetch_optional(&state.mysql)
                .await?;
                row.map(|item| item.0)
            } else {
                None
            };

            let panel_id = if let Some(panel_id) = panel_id {
                panel_id
            } else {
                if provided_panel_id.is_some() {
                    return Err(AppError::not_found("panel not found"));
                }
                if method == "PATCH" && (storyboard_id.is_none() || panel_index.is_none()) {
                    return Err(AppError::invalid_params("panelId is required"));
                }
                let storyboard_id = storyboard_id
                    .ok_or_else(|| AppError::invalid_params("storyboardId is required"))?;
                let panel_index = panel_index
                    .ok_or_else(|| AppError::invalid_params("panelIndex is required"))?;
                let storyboard_exists: Option<(String,)> = sqlx::query_as(
                    "SELECT s.id
                     FROM novel_promotion_storyboards s
                     INNER JOIN novel_promotion_episodes e ON e.id = s.episodeId
                     WHERE s.id = ? AND e.novelPromotionProjectId = ?
                     LIMIT 1",
                )
                .bind(&storyboard_id)
                .bind(&novel_id)
                .fetch_optional(&state.mysql)
                .await?;
                if storyboard_exists.is_none() {
                    return Err(AppError::not_found("storyboard not found"));
                }
                let panel_id = Uuid::new_v4().to_string();
                let panel_number = body_i32(&body, "panelNumber").or(Some(panel_index + 1));
                let srt_start = match body.get("srtStart") {
                    Some(value) => parse_nullable_f64_field(value, "srtStart")?,
                    None => None,
                };
                let srt_end = match body.get("srtEnd") {
                    Some(value) => parse_nullable_f64_field(value, "srtEnd")?,
                    None => None,
                };
                let duration = match body.get("duration") {
                    Some(value) => parse_nullable_f64_field(value, "duration")?,
                    None => None,
                };

                sqlx::query(
                    "INSERT INTO novel_promotion_panels (id, storyboardId, panelIndex, panelNumber, shotType, cameraMove, description, location, characters, srtStart, srtEnd, duration, imagePrompt, imageUrl, candidateImages, videoPrompt, firstLastFramePrompt, videoUrl, actingNotes, photographyRules, linkedToNextPanel, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
                )
                .bind(&panel_id)
                .bind(&storyboard_id)
                .bind(panel_index)
                .bind(panel_number)
                .bind(body_string(&body, "shotType"))
                .bind(body_string(&body, "cameraMove"))
                .bind(body_string(&body, "description"))
                .bind(body_string(&body, "location"))
                .bind(normalize_optional_json(body.get("characters").cloned()))
                .bind(srt_start)
                .bind(srt_end)
                .bind(duration)
                .bind(body_string(&body, "imagePrompt"))
                .bind(body_string(&body, "imageUrl"))
                .bind(normalize_optional_json(body.get("candidateImages").cloned()))
                .bind(body_string(&body, "videoPrompt"))
                .bind(body_string(&body, "firstLastFramePrompt"))
                .bind(body_string(&body, "videoUrl"))
                .bind(normalize_optional_json(body.get("actingNotes").cloned()))
                .bind(normalize_optional_json(body.get("photographyRules").cloned()))
                .bind(body_bool(&body, "linkedToNextPanel").unwrap_or(false))
                .execute(&state.mysql)
                .await?;

                sqlx::query(
                    "UPDATE novel_promotion_storyboards SET panelCount = (SELECT COUNT(*) FROM novel_promotion_panels WHERE storyboardId = ?), updatedAt = NOW(3) WHERE id = ?",
                )
                .bind(&storyboard_id)
                .bind(&storyboard_id)
                .execute(&state.mysql)
                .await?;

                return Ok(Json(json!({ "success": true })));
            };
            let mut qb: QueryBuilder<'_, MySql> =
                QueryBuilder::new("UPDATE novel_promotion_panels SET ");
            let mut separated = qb.separated(", ");
            let mut touched = false;

            for key in [
                "shotType",
                "cameraMove",
                "description",
                "location",
                "imagePrompt",
                "imageUrl",
                "videoPrompt",
                "videoUrl",
                "firstLastFramePrompt",
                "previousImageUrl",
            ] {
                if let Some(value) = body.get(key) {
                    touched = true;
                    separated.push(format!("{key} = ")).push_bind_unseparated(
                        value
                            .as_str()
                            .map(|item| item.trim().to_string())
                            .filter(|item| !item.is_empty()),
                    );
                }
            }

            for key in ["panelIndex", "panelNumber"] {
                if let Some(value) = body.get(key) {
                    touched = true;
                    separated
                        .push(format!("{key} = "))
                        .push_bind_unseparated(parse_nullable_i32_field(value, key)?);
                }
            }
            for key in ["srtStart", "srtEnd", "duration"] {
                if let Some(value) = body.get(key) {
                    touched = true;
                    separated
                        .push(format!("{key} = "))
                        .push_bind_unseparated(parse_nullable_f64_field(value, key)?);
                }
            }

            if body.get("linkedToNextPanel").is_some() {
                touched = true;
                separated
                    .push("linkedToNextPanel = ")
                    .push_bind_unseparated(body_bool(&body, "linkedToNextPanel"));
            }

            if body.get("characters").is_some() {
                touched = true;
                separated
                    .push("characters = ")
                    .push_bind_unseparated(normalize_optional_json(
                        body.get("characters").cloned(),
                    ));
            }
            if body.get("candidateImages").is_some() {
                touched = true;
                separated.push("candidateImages = ").push_bind_unseparated(
                    normalize_optional_json(body.get("candidateImages").cloned()),
                );
            }
            if body.get("actingNotes").is_some() {
                touched = true;
                separated
                    .push("actingNotes = ")
                    .push_bind_unseparated(normalize_optional_json(
                        body.get("actingNotes").cloned(),
                    ));
            }
            if body.get("photographyRules").is_some() {
                touched = true;
                separated.push("photographyRules = ").push_bind_unseparated(
                    normalize_optional_json(body.get("photographyRules").cloned()),
                );
            }

            if !touched {
                return Err(AppError::invalid_params("empty update payload"));
            }

            separated.push("updatedAt = NOW(3)");
            qb.push(" WHERE id = ");
            qb.push_bind(&panel_id);
            qb.build().execute(&state.mysql).await?;
            Ok(Json(json!({ "success": true })))
        }
        "DELETE" => {
            let panel_id = body_string(&body, "panelId")
                .or_else(|| query_string(query, "panelId"))
                .ok_or_else(|| AppError::invalid_params("panelId is required"))?;
            let panel: Option<(String, i32)> = sqlx::query_as(
                "SELECT p.storyboardId, p.panelIndex
                 FROM novel_promotion_panels p
                 INNER JOIN novel_promotion_storyboards s ON s.id = p.storyboardId
                 INNER JOIN novel_promotion_episodes e ON e.id = s.episodeId
                 WHERE p.id = ? AND e.novelPromotionProjectId = ?
                 LIMIT 1",
            )
            .bind(&panel_id)
            .bind(&novel_id)
            .fetch_optional(&state.mysql)
            .await?;
            let Some((storyboard_id, deleted_panel_index)) = panel else {
                return Err(AppError::not_found("panel not found"));
            };

            let mut tx = state.mysql.begin().await?;
            let max_panel: Option<(i32,)> = sqlx::query_as(
                "SELECT panelIndex FROM novel_promotion_panels WHERE storyboardId = ? ORDER BY panelIndex DESC LIMIT 1",
            )
            .bind(&storyboard_id)
            .fetch_optional(&mut *tx)
            .await?;
            let max_panel_index = max_panel.map(|item| item.0).unwrap_or(0);
            let offset = max_panel_index + 1000;

            sqlx::query("DELETE FROM novel_promotion_panels WHERE id = ?")
                .bind(&panel_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "UPDATE novel_promotion_panels
                 SET panelIndex = panelIndex + ?,
                     panelNumber = CASE WHEN panelNumber IS NULL THEN NULL ELSE panelNumber + ? END,
                     updatedAt = NOW(3)
                 WHERE storyboardId = ? AND panelIndex > ?",
            )
            .bind(offset)
            .bind(offset)
            .bind(&storyboard_id)
            .bind(deleted_panel_index)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "UPDATE novel_promotion_panels
                 SET panelIndex = panelIndex - ?,
                     panelNumber = CASE WHEN panelNumber IS NULL THEN NULL ELSE panelNumber - ? END,
                     updatedAt = NOW(3)
                 WHERE storyboardId = ? AND panelIndex > ?",
            )
            .bind(offset + 1)
            .bind(offset + 1)
            .bind(&storyboard_id)
            .bind(deleted_panel_index + offset)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "UPDATE novel_promotion_storyboards
                 SET panelCount = (SELECT COUNT(*) FROM novel_promotion_panels WHERE storyboardId = ?),
                     updatedAt = NOW(3)
                 WHERE id = ?",
            )
            .bind(&storyboard_id)
            .bind(&storyboard_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok(Json(json!({ "success": true })))
        }
        _ => Err(AppError::invalid_params("unsupported method for /panel")),
    }
}

async fn handle_storyboard_group(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
    method: &str,
    query: &HashMap<String, String>,
    headers: &HeaderMap,
    body: Value,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;
    let novel_id = get_novel_id(state, project_id).await?;

    match method {
        "POST" => {
            let locale = request_locale(Some(&body), Some(headers));
            let episode_id = body_string(&body, "episodeId")
                .ok_or_else(|| AppError::invalid_params("episodeId is required"))?;
            let episode_exists: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM novel_promotion_episodes WHERE id = ? AND novelPromotionProjectId = ? LIMIT 1",
            )
            .bind(&episode_id)
            .bind(&novel_id)
            .fetch_optional(&state.mysql)
            .await?;
            if episode_exists.is_none() {
                return Err(AppError::not_found("episode not found"));
            }

            let clip_id = Uuid::new_v4().to_string();
            let storyboard_id = Uuid::new_v4().to_string();
            let panel_id = Uuid::new_v4().to_string();
            let insert_index = body_i32(&body, "insertIndex")
                .and_then(|value| usize::try_from(value).ok())
                .unwrap_or(usize::MAX);

            let existing_clips: Vec<(NaiveDateTime,)> = sqlx::query_as(
                "SELECT createdAt FROM novel_promotion_clips WHERE episodeId = ? ORDER BY createdAt ASC",
            )
            .bind(&episode_id)
            .fetch_all(&state.mysql)
            .await?;

            let insert_at = insert_index.min(existing_clips.len());
            let created_at = if existing_clips.is_empty() {
                Utc::now().naive_utc()
            } else if insert_at == 0 {
                existing_clips[0].0 - ChronoDuration::seconds(1)
            } else if insert_at >= existing_clips.len() {
                existing_clips[existing_clips.len() - 1].0 + ChronoDuration::seconds(1)
            } else {
                let prev = existing_clips[insert_at - 1].0;
                let next = existing_clips[insert_at].0;
                let prev_ms = prev.and_utc().timestamp_millis();
                let next_ms = next.and_utc().timestamp_millis();
                let mut mid_ms = prev_ms + (next_ms - prev_ms) / 2;
                if mid_ms <= prev_ms {
                    mid_ms = prev_ms + 1;
                }
                if mid_ms >= next_ms {
                    mid_ms = next_ms - 1;
                }
                chrono::DateTime::<Utc>::from_timestamp_millis(mid_ms)
                    .map(|value| value.naive_utc())
                    .unwrap_or_else(|| prev + ChronoDuration::milliseconds(1))
            };

            let mut tx = state.mysql.begin().await?;
            sqlx::query(
                "INSERT INTO novel_promotion_clips (id, episodeId, summary, content, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, NOW(3))",
            )
            .bind(&clip_id)
            .bind(&episode_id)
            .bind(
                body_string(&body, "summary")
                    .unwrap_or_else(|| {
                        localized_msg(
                            locale,
                            "手动添加的分镜组",
                            "Manually added storyboard group",
                        )
                        .to_string()
                    }),
            )
            .bind(body_string(&body, "content").unwrap_or_default())
            .bind(created_at)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO novel_promotion_storyboards (id, episodeId, clipId, panelCount, createdAt, updatedAt) VALUES (?, ?, ?, 1, NOW(3), NOW(3))",
            )
            .bind(&storyboard_id)
            .bind(&episode_id)
            .bind(&clip_id)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO novel_promotion_panels (id, storyboardId, panelIndex, panelNumber, shotType, cameraMove, description, characters, linkedToNextPanel, createdAt, updatedAt) VALUES (?, ?, 0, 1, ?, ?, ?, ?, false, NOW(3), NOW(3))",
            )
            .bind(&panel_id)
            .bind(&storyboard_id)
            .bind(localized_msg(locale, "中景", "Medium shot"))
            .bind(localized_msg(locale, "固定", "Static"))
            .bind(localized_msg(locale, "新镜头描述", "New shot description"))
            .bind(normalize_optional_json(Some(json!([]))))
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            let clip = sqlx::query_as::<_, ClipRow>(
                "SELECT id, episodeId, start, end, duration, summary, location, content, characters, endText, shotCount, startText, screenplay, createdAt, updatedAt FROM novel_promotion_clips WHERE id = ? LIMIT 1",
            )
            .bind(&clip_id)
            .fetch_one(&state.mysql)
            .await?;
            let storyboard = sqlx::query_as::<_, StoryboardRow>(
                "SELECT id, episodeId, clipId, storyboardImageUrl, panelCount, storyboardTextJson, imageHistory, candidateImages, lastError, photographyPlan, createdAt, updatedAt FROM novel_promotion_storyboards WHERE id = ? LIMIT 1",
            )
            .bind(&storyboard_id)
            .fetch_one(&state.mysql)
            .await?;
            let panel = sqlx::query_as::<_, StoryboardPanelDetailRow>(
                "SELECT id, storyboardId, panelIndex, panelNumber, shotType, cameraMove, description, location, characters, srtStart, srtEnd, duration, videoPrompt, firstLastFramePrompt, imageUrl, candidateImages, linkedToNextPanel, actingNotes, photographyRules, createdAt, updatedAt FROM novel_promotion_panels WHERE id = ? LIMIT 1",
            )
            .bind(&panel_id)
            .fetch_one(&state.mysql)
            .await?;

            Ok(Json(json!({
              "success": true,
              "clip": clip,
              "panel": panel,
              "storyboard": storyboard,
            })))
        }
        "PUT" => {
            let (current_clip_id, target_clip_id) = if let (
                Some(episode_id),
                Some(clip_id),
                Some(direction),
            ) = (
                body_string(&body, "episodeId"),
                body_string(&body, "clipId"),
                body_string(&body, "direction"),
            ) {
                let episode_exists: Option<(String,)> = sqlx::query_as(
                        "SELECT id FROM novel_promotion_episodes WHERE id = ? AND novelPromotionProjectId = ? LIMIT 1",
                    )
                    .bind(&episode_id)
                    .bind(&novel_id)
                    .fetch_optional(&state.mysql)
                    .await?;
                if episode_exists.is_none() {
                    return Err(AppError::not_found("episode not found"));
                }

                let clips = sqlx::query_as::<_, ClipRow>(
                        "SELECT id, episodeId, start, end, duration, summary, location, content, characters, endText, shotCount, startText, screenplay, createdAt, updatedAt FROM novel_promotion_clips WHERE episodeId = ? ORDER BY createdAt ASC",
                    )
                    .bind(&episode_id)
                    .fetch_all(&state.mysql)
                    .await?;
                let current_index = clips
                    .iter()
                    .position(|item| item.id == clip_id)
                    .ok_or_else(|| AppError::not_found("clip not found"))?;
                let target_index = if direction == "up" {
                    current_index.saturating_sub(1)
                } else if direction == "down" {
                    current_index + 1
                } else {
                    return Err(AppError::invalid_params("direction must be up or down"));
                };
                if target_index >= clips.len() || target_index == current_index {
                    return Err(AppError::invalid_params("invalid clip move target"));
                }
                (
                    clips[current_index].id.clone(),
                    clips[target_index].id.clone(),
                )
            } else {
                let current_clip_id = body_string(&body, "currentClipId")
                    .ok_or_else(|| AppError::invalid_params("currentClipId is required"))?;
                let target_clip_id = body_string(&body, "targetClipId")
                    .ok_or_else(|| AppError::invalid_params("targetClipId is required"))?;

                let current_scope: Option<(String,)> = sqlx::query_as(
                    "SELECT c.episodeId
                     FROM novel_promotion_clips c
                     INNER JOIN novel_promotion_episodes e ON e.id = c.episodeId
                     WHERE c.id = ? AND e.novelPromotionProjectId = ?
                     LIMIT 1",
                )
                .bind(&current_clip_id)
                .bind(&novel_id)
                .fetch_optional(&state.mysql)
                .await?;
                let target_scope: Option<(String,)> = sqlx::query_as(
                    "SELECT c.episodeId
                     FROM novel_promotion_clips c
                     INNER JOIN novel_promotion_episodes e ON e.id = c.episodeId
                     WHERE c.id = ? AND e.novelPromotionProjectId = ?
                     LIMIT 1",
                )
                .bind(&target_clip_id)
                .bind(&novel_id)
                .fetch_optional(&state.mysql)
                .await?;

                let (Some((current_episode_id,)), Some((target_episode_id,))) =
                    (current_scope, target_scope)
                else {
                    return Err(AppError::not_found("clip not found"));
                };
                if current_episode_id != target_episode_id {
                    return Err(AppError::invalid_params(
                        "clips must belong to the same episode",
                    ));
                }

                (current_clip_id, target_clip_id)
            };

            let current: Option<(NaiveDateTime,)> =
                sqlx::query_as("SELECT createdAt FROM novel_promotion_clips WHERE id = ? LIMIT 1")
                    .bind(&current_clip_id)
                    .fetch_optional(&state.mysql)
                    .await?;
            let target: Option<(NaiveDateTime,)> =
                sqlx::query_as("SELECT createdAt FROM novel_promotion_clips WHERE id = ? LIMIT 1")
                    .bind(&target_clip_id)
                    .fetch_optional(&state.mysql)
                    .await?;

            let (current_ts, target_ts) = match (current, target) {
                (Some(c), Some(t)) => (c.0, t.0),
                _ => return Err(AppError::not_found("clip not found")),
            };

            let mut tx = state.mysql.begin().await?;
            sqlx::query("UPDATE novel_promotion_clips SET createdAt = ? WHERE id = ?")
                .bind(target_ts)
                .bind(&current_clip_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE novel_promotion_clips SET createdAt = ? WHERE id = ?")
                .bind(current_ts)
                .bind(&target_clip_id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;

            Ok(Json(json!({ "success": true })))
        }
        "DELETE" => {
            let storyboard_id = body_string(&body, "storyboardId")
                .or_else(|| query_string(query, "storyboardId"))
                .ok_or_else(|| AppError::invalid_params("storyboardId is required"))?;

            let clip: Option<(String,)> = sqlx::query_as(
                "SELECT s.clipId
                 FROM novel_promotion_storyboards s
                 INNER JOIN novel_promotion_episodes e ON e.id = s.episodeId
                 WHERE s.id = ? AND e.novelPromotionProjectId = ?
                 LIMIT 1",
            )
            .bind(&storyboard_id)
            .bind(&novel_id)
            .fetch_optional(&state.mysql)
            .await?;

            let Some((clip_id,)) = clip else {
                return Err(AppError::not_found("storyboard not found"));
            };

            let mut tx = state.mysql.begin().await?;
            sqlx::query("DELETE FROM novel_promotion_panels WHERE storyboardId = ?")
                .bind(&storyboard_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM novel_promotion_storyboards WHERE id = ?")
                .bind(&storyboard_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM novel_promotion_clips WHERE id = ?")
                .bind(&clip_id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;

            Ok(Json(json!({ "success": true })))
        }
        _ => Err(AppError::invalid_params(
            "unsupported method for /storyboard-group",
        )),
    }
}

async fn handle_clips(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
    method: &str,
    clip_id: Option<&str>,
    body: Value,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;

    match (method, clip_id) {
        ("POST", None) => {
            let (target_type, target_id) = normalize_task_target(&body, "episode", "clips-build");
            submit_novel_task(
                state,
                user,
                project_id,
                "clips_build",
                &target_type,
                &target_id,
                body,
                Some(1),
                None,
            )
            .await
        }
        ("PATCH", Some(clip_id)) => {
            let mut qb: QueryBuilder<'_, MySql> =
                QueryBuilder::new("UPDATE novel_promotion_clips SET ");
            let mut separated = qb.separated(", ");
            let mut touched = false;

            for key in [
                "summary",
                "content",
                "location",
                "characters",
                "endText",
                "startText",
                "screenplay",
            ] {
                if body.get(key).is_some() {
                    touched = true;
                    separated
                        .push(format!("{key} = "))
                        .push_bind_unseparated(body_string(&body, key));
                }
            }
            for key in ["start", "end", "duration", "shotCount"] {
                if body.get(key).is_some() {
                    touched = true;
                    separated
                        .push(format!("{key} = "))
                        .push_bind_unseparated(body_i32(&body, key));
                }
            }

            if !touched {
                return Err(AppError::invalid_params("empty clip update payload"));
            }

            separated.push("updatedAt = NOW(3)");
            qb.push(" WHERE id = ");
            qb.push_bind(clip_id);
            qb.build().execute(&state.mysql).await?;

            let clip = sqlx::query_as::<_, ClipRow>(
                "SELECT id, episodeId, start, end, duration, summary, location, content, characters, endText, shotCount, startText, screenplay, createdAt, updatedAt FROM novel_promotion_clips WHERE id = ? LIMIT 1",
            )
            .bind(clip_id)
            .fetch_one(&state.mysql)
            .await?;

            Ok(Json(json!({ "success": true, "clip": clip })))
        }
        _ => Err(AppError::invalid_params("unsupported clips operation")),
    }
}

async fn handle_speaker_voice(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
    method: &str,
    query: &HashMap<String, String>,
    body: Value,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;
    let novel_id = get_novel_id(state, project_id).await?;

    let episode_id = query
        .get("episodeId")
        .cloned()
        .or_else(|| body_string(&body, "episodeId"))
        .ok_or_else(|| AppError::invalid_params("episodeId is required"))?;

    let row: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT speakerVoices FROM novel_promotion_episodes WHERE id = ? AND novelPromotionProjectId = ? LIMIT 1",
    )
    .bind(&episode_id)
    .bind(&novel_id)
    .fetch_optional(&state.mysql)
    .await?;
    let Some((speaker_voices_raw,)) = row else {
        return Err(AppError::not_found("episode not found"));
    };

    match method {
        "GET" => {
            let mut speaker_voices = speaker_voices_raw
                .and_then(|item| serde_json::from_str::<Value>(&item).ok())
                .unwrap_or_else(|| json!({}));
            if let Some(object) = speaker_voices.as_object_mut() {
                for value in object.values_mut() {
                    if let Some(entry) = value.as_object_mut()
                        && let Some(audio_value) = entry.get("audioUrl").and_then(Value::as_str)
                        && let Some(public_url) = media::to_public_media_url(Some(audio_value))
                    {
                        entry.insert("audioUrl".to_string(), Value::String(public_url));
                    }
                }
            }
            Ok(Json(json!({ "speakerVoices": speaker_voices })))
        }
        "PATCH" => {
            let mut speaker_voices = speaker_voices_raw
                .and_then(|item| serde_json::from_str::<Value>(&item).ok())
                .unwrap_or_else(|| json!({}));
            if !speaker_voices.is_object() {
                speaker_voices = json!({});
            }
            let mut touched = false;

            if let (Some(speaker), Some(audio_url)) = (
                body_string(&body, "speaker"),
                body_string(&body, "audioUrl"),
            ) {
                let voice_type =
                    body_string(&body, "voiceType").unwrap_or_else(|| "uploaded".to_string());
                let voice_id = body_string(&body, "voiceId");
                let normalized_audio_url = if let Some((_, storage_key)) =
                    resolve_media_object_from_value(state, &audio_url).await?
                {
                    storage_key
                } else {
                    audio_url
                };
                let mut value = serde_json::Map::new();
                value.insert("voiceType".to_string(), json!(voice_type));
                if let Some(voice_id) = voice_id {
                    value.insert("voiceId".to_string(), json!(voice_id));
                }
                value.insert("audioUrl".to_string(), json!(normalized_audio_url));
                if let Some(object) = speaker_voices.as_object_mut() {
                    object.insert(speaker, Value::Object(value));
                }
                touched = true;
            } else if let Some(value) = body.get("speakerVoices") {
                speaker_voices = value.clone();
                touched = true;
            }

            if !touched {
                return Err(AppError::invalid_params(
                    "speaker + audioUrl or speakerVoices is required",
                ));
            }

            sqlx::query(
                "UPDATE novel_promotion_episodes SET speakerVoices = ?, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(
                serde_json::to_string(&speaker_voices).map_err(|err| {
                    AppError::invalid_params(format!("invalid speakerVoices: {err}"))
                })?,
            )
            .bind(&episode_id)
            .execute(&state.mysql)
            .await?;
            Ok(Json(json!({ "success": true })))
        }
        _ => Err(AppError::invalid_params(
            "unsupported method for /speaker-voice",
        )),
    }
}

async fn handle_video_urls(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
    headers: &HeaderMap,
    body: Value,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;
    let locale = request_locale(Some(&body), Some(headers));
    let episode_id = body_string(&body, "episodeId");
    let panel_preferences = body
        .get("panelPreferences")
        .and_then(Value::as_object)
        .map(|value| {
            value
                .iter()
                .filter_map(|(key, item)| item.as_bool().map(|flag| (key.to_string(), flag)))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let novel_id = get_novel_id(state, project_id).await?;

    let rows = if let Some(ref episode_id) = episode_id {
        sqlx::query_as::<_, MediaPanelDownloadRow>(
            "SELECT p.id, p.storyboardId, p.panelIndex, p.description, p.imageUrl, p.videoUrl, p.lipSyncVideoUrl
             FROM novel_promotion_panels p
             INNER JOIN novel_promotion_storyboards s ON s.id = p.storyboardId
             INNER JOIN novel_promotion_clips c ON c.id = s.clipId
             INNER JOIN novel_promotion_episodes e ON e.id = s.episodeId
             WHERE e.novelPromotionProjectId = ? AND e.id = ?
             ORDER BY e.episodeNumber ASC, c.createdAt ASC, p.panelIndex ASC",
        )
        .bind(&novel_id)
        .bind(episode_id)
        .fetch_all(&state.mysql)
        .await?
    } else {
        sqlx::query_as::<_, MediaPanelDownloadRow>(
            "SELECT p.id, p.storyboardId, p.panelIndex, p.description, p.imageUrl, p.videoUrl, p.lipSyncVideoUrl
             FROM novel_promotion_panels p
             INNER JOIN novel_promotion_storyboards s ON s.id = p.storyboardId
             INNER JOIN novel_promotion_clips c ON c.id = s.clipId
             INNER JOIN novel_promotion_episodes e ON e.id = s.episodeId
             WHERE e.novelPromotionProjectId = ?
             ORDER BY e.episodeNumber ASC, c.createdAt ASC, p.panelIndex ASC",
        )
        .bind(&novel_id)
        .fetch_all(&state.mysql)
        .await?
    };

    let mut videos = Vec::new();
    for row in rows {
        let panel_key = format!("{}-{}", row.storyboard_id, row.panel_index);
        let prefer_lip_sync = panel_preferences.get(&panel_key).copied().unwrap_or(true);
        let selected_video_url = if prefer_lip_sync {
            row.lip_sync_video_url.as_ref().or(row.video_url.as_ref())
        } else {
            row.video_url.as_ref().or(row.lip_sync_video_url.as_ref())
        };

        let Some(video_url) = selected_video_url else {
            continue;
        };
        let description = row
            .description
            .as_deref()
            .filter(|item| !item.trim().is_empty())
            .unwrap_or(localized_msg(locale, "镜头", "Shot"));
        let safe_desc = sanitize_filename_segment(
            &description.chars().take(50).collect::<String>(),
            localized_msg(locale, "镜头", "Shot"),
        );
        let index = videos.len() + 1;
        let file_name = format!("{index:03}_{safe_desc}.mp4");
        let proxy_url = format!(
            "/api/novel-promotion/{project_id}/video-proxy?key={}",
            urlencoding::encode(video_url)
        );
        let is_lip_sync = row
            .lip_sync_video_url
            .as_deref()
            .is_some_and(|value| value == video_url);
        videos.push(json!({
          "index": index,
          "fileName": file_name,
          "panelId": row.id,
          "storyboardId": row.storyboard_id,
          "panelIndex": row.panel_index,
          "isLipSync": is_lip_sync,
          "sourceVideoUrl": video_url,
          "videoUrl": proxy_url,
        }));
    }

    if videos.is_empty() {
        return Err(AppError::invalid_params("no videos available for download"));
    }

    let project_name: Option<(String,)> =
        sqlx::query_as("SELECT name FROM projects WHERE id = ? LIMIT 1")
            .bind(project_id)
            .fetch_optional(&state.mysql)
            .await?;

    Ok(Json(json!({
      "projectName": project_name.map(|item| item.0).unwrap_or_default(),
      "videos": videos,
    })))
}

async fn route_assets(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    handle_assets(&state, &user, &project_id).await
}

async fn route_episodes_get(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    handle_episodes_get(&state, &user, &project_id).await
}

async fn route_episodes_post(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    handle_episodes_post(&state, &user, &project_id, body).await
}

async fn route_video_urls(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    handle_video_urls(&state, &user, &project_id, &headers, body).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownloadVideosBody {
    episode_id: Option<String>,
    panel_preferences: Option<HashMap<String, bool>>,
}

#[derive(Debug, Deserialize)]
struct DownloadQuery {
    #[serde(rename = "episodeId")]
    episode_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VideoProxyQuery {
    key: Option<String>,
    url: Option<String>,
}

async fn route_download_images(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
    headers: HeaderMap,
    Query(query): Query<DownloadQuery>,
) -> Result<Response, AppError> {
    verify_project_owner(&state, &project_id, &user.id).await?;
    let locale = request_locale(None, Some(&headers));
    let novel_id = get_novel_id(&state, &project_id).await?;
    let episode_id = query
        .episode_id
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty());

    let rows = if let Some(ref episode_id) = episode_id {
        sqlx::query_as::<_, MediaPanelDownloadRow>(
            "SELECT p.id, p.storyboardId, p.panelIndex, p.description, p.imageUrl, p.videoUrl, p.lipSyncVideoUrl
             FROM novel_promotion_panels p
             INNER JOIN novel_promotion_storyboards s ON s.id = p.storyboardId
             INNER JOIN novel_promotion_clips c ON c.id = s.clipId
             INNER JOIN novel_promotion_episodes e ON e.id = s.episodeId
             WHERE e.novelPromotionProjectId = ? AND e.id = ?
             ORDER BY e.episodeNumber ASC, c.createdAt ASC, p.panelIndex ASC",
        )
        .bind(&novel_id)
        .bind(episode_id)
        .fetch_all(&state.mysql)
        .await?
    } else {
        sqlx::query_as::<_, MediaPanelDownloadRow>(
            "SELECT p.id, p.storyboardId, p.panelIndex, p.description, p.imageUrl, p.videoUrl, p.lipSyncVideoUrl
             FROM novel_promotion_panels p
             INNER JOIN novel_promotion_storyboards s ON s.id = p.storyboardId
             INNER JOIN novel_promotion_clips c ON c.id = s.clipId
             INNER JOIN novel_promotion_episodes e ON e.id = s.episodeId
             WHERE e.novelPromotionProjectId = ?
             ORDER BY e.episodeNumber ASC, c.createdAt ASC, p.panelIndex ASC",
        )
        .bind(&novel_id)
        .fetch_all(&state.mysql)
        .await?
    };

    let mut entries = Vec::new();
    for row in rows {
        let Some(image_url) = row.image_url.as_deref() else {
            continue;
        };
        let index = entries.len() + 1;
        let desc = row
            .description
            .as_deref()
            .filter(|item| !item.trim().is_empty())
            .unwrap_or(localized_msg(locale, "镜头", "Shot"));
        let safe_desc = sanitize_filename_segment(
            &desc.chars().take(50).collect::<String>(),
            localized_msg(locale, "镜头", "Shot"),
        );
        let fallback_ext = infer_extension_from_source(image_url, "png");
        let download_result = download_binary_from_media_source(&state, image_url).await;
        let (bytes, content_type) = match download_result {
            Ok(value) => value,
            Err(err) => {
                warn!(
                    project_id = %project_id,
                    panel_id = %row.id,
                    panel_index = row.panel_index,
                    error = %err,
                    "failed to download panel image for zip archive"
                );
                continue;
            }
        };
        let ext = infer_extension_from_content_type(&content_type, &fallback_ext);
        let file_name = format!("{index:03}_{safe_desc}.{ext}");
        entries.push((file_name, bytes));
    }

    if entries.is_empty() {
        return Err(AppError::invalid_params("no downloadable images"));
    }

    let bytes = build_zip_archive(entries)?;
    let project_name: Option<(String,)> =
        sqlx::query_as("SELECT name FROM projects WHERE id = ? LIMIT 1")
            .bind(&project_id)
            .fetch_optional(&state.mysql)
            .await?;
    let resolved_project_name = project_name
        .map(|item| item.0)
        .filter(|item| !item.trim().is_empty())
        .unwrap_or_else(|| "project".to_string());
    let zip_name = format!("{}_images.zip", resolved_project_name.trim());
    zip_attachment_response(bytes, &zip_name)
}

async fn route_download_videos(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<DownloadVideosBody>,
) -> Result<Response, AppError> {
    verify_project_owner(&state, &project_id, &user.id).await?;
    let locale = request_locale(None, Some(&headers));
    let novel_id = get_novel_id(&state, &project_id).await?;
    let episode_id = body
        .episode_id
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty());
    let panel_preferences = body.panel_preferences.unwrap_or_default();

    let rows = if let Some(ref episode_id) = episode_id {
        sqlx::query_as::<_, MediaPanelDownloadRow>(
            "SELECT p.id, p.storyboardId, p.panelIndex, p.description, p.imageUrl, p.videoUrl, p.lipSyncVideoUrl
             FROM novel_promotion_panels p
             INNER JOIN novel_promotion_storyboards s ON s.id = p.storyboardId
             INNER JOIN novel_promotion_clips c ON c.id = s.clipId
             INNER JOIN novel_promotion_episodes e ON e.id = s.episodeId
             WHERE e.novelPromotionProjectId = ? AND e.id = ?
             ORDER BY e.episodeNumber ASC, c.createdAt ASC, p.panelIndex ASC",
        )
        .bind(&novel_id)
        .bind(episode_id)
        .fetch_all(&state.mysql)
        .await?
    } else {
        sqlx::query_as::<_, MediaPanelDownloadRow>(
            "SELECT p.id, p.storyboardId, p.panelIndex, p.description, p.imageUrl, p.videoUrl, p.lipSyncVideoUrl
             FROM novel_promotion_panels p
             INNER JOIN novel_promotion_storyboards s ON s.id = p.storyboardId
             INNER JOIN novel_promotion_clips c ON c.id = s.clipId
             INNER JOIN novel_promotion_episodes e ON e.id = s.episodeId
             WHERE e.novelPromotionProjectId = ?
             ORDER BY e.episodeNumber ASC, c.createdAt ASC, p.panelIndex ASC",
        )
        .bind(&novel_id)
        .fetch_all(&state.mysql)
        .await?
    };

    let mut entries = Vec::new();
    for row in rows {
        let panel_key = format!("{}-{}", row.storyboard_id, row.panel_index);
        let prefer_lip_sync = panel_preferences.get(&panel_key).copied().unwrap_or(true);
        let selected_video = if prefer_lip_sync {
            row.lip_sync_video_url
                .as_deref()
                .or(row.video_url.as_deref())
        } else {
            row.video_url
                .as_deref()
                .or(row.lip_sync_video_url.as_deref())
        };
        let Some(video_url) = selected_video else {
            continue;
        };

        let index = entries.len() + 1;
        let desc = row
            .description
            .as_deref()
            .filter(|item| !item.trim().is_empty())
            .unwrap_or(localized_msg(locale, "镜头", "Shot"));
        let safe_desc = sanitize_filename_segment(
            &desc.chars().take(50).collect::<String>(),
            localized_msg(locale, "镜头", "Shot"),
        );
        let download_result = download_binary_from_media_source(&state, video_url).await;
        let (bytes, content_type) = match download_result {
            Ok(value) => value,
            Err(err) => {
                warn!(
                    project_id = %project_id,
                    panel_id = %row.id,
                    panel_index = row.panel_index,
                    error = %err,
                    "failed to download panel video for zip archive"
                );
                continue;
            }
        };
        let ext = infer_extension_from_content_type(&content_type, "mp4");
        let file_name = format!("{index:03}_{safe_desc}.{ext}");
        entries.push((file_name, bytes));
    }

    if entries.is_empty() {
        return Err(AppError::invalid_params("no downloadable videos"));
    }

    let bytes = build_zip_archive(entries)?;
    let project_name: Option<(String,)> =
        sqlx::query_as("SELECT name FROM projects WHERE id = ? LIMIT 1")
            .bind(&project_id)
            .fetch_optional(&state.mysql)
            .await?;
    let resolved_project_name = project_name
        .map(|item| item.0)
        .filter(|item| !item.trim().is_empty())
        .unwrap_or_else(|| "project".to_string());
    let zip_name = format!("{}_videos.zip", resolved_project_name.trim());
    zip_attachment_response(bytes, &zip_name)
}

async fn route_download_voices(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
    Query(query): Query<DownloadQuery>,
) -> Result<Response, AppError> {
    verify_project_owner(&state, &project_id, &user.id).await?;
    let novel_id = get_novel_id(&state, &project_id).await?;
    let episode_id = query
        .episode_id
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty());

    let rows = if let Some(ref episode_id) = episode_id {
        sqlx::query_as::<_, VoiceLineRow>(
            "SELECT id, episodeId, lineIndex, speaker, content, voicePresetId, audioUrl, audioMediaId, emotionPrompt, emotionStrength, matchedPanelIndex, matchedStoryboardId, audioDuration, matchedPanelId, createdAt, updatedAt
             FROM novel_promotion_voice_lines
             WHERE episodeId = ? AND audioUrl IS NOT NULL
             ORDER BY lineIndex ASC",
        )
        .bind(episode_id)
        .fetch_all(&state.mysql)
        .await?
    } else {
        sqlx::query_as::<_, VoiceLineRow>(
            "SELECT id, episodeId, lineIndex, speaker, content, voicePresetId, audioUrl, audioMediaId, emotionPrompt, emotionStrength, matchedPanelIndex, matchedStoryboardId, audioDuration, matchedPanelId, createdAt, updatedAt
             FROM novel_promotion_voice_lines
             WHERE episodeId IN (SELECT id FROM novel_promotion_episodes WHERE novelPromotionProjectId = ?)
               AND audioUrl IS NOT NULL
             ORDER BY lineIndex ASC",
        )
        .bind(&novel_id)
        .fetch_all(&state.mysql)
        .await?
    };

    let mut entries = Vec::new();
    for row in rows {
        let Some(audio_url) = row.audio_url.as_deref() else {
            continue;
        };
        let download_result = download_binary_from_media_source(&state, audio_url).await;
        let (bytes, content_type) = match download_result {
            Ok(value) => value,
            Err(err) => {
                warn!(
                    project_id = %project_id,
                    line_id = %row.id,
                    line_index = row.line_index,
                    error = %err,
                    "failed to download voice line for zip archive"
                );
                continue;
            }
        };

        let safe_speaker = sanitize_filename_segment(&row.speaker, "speaker");
        let safe_content = sanitize_voice_content_segment(&row.content);
        let fallback_ext = infer_extension_from_source(audio_url, "mp3");
        let ext = infer_extension_from_content_type(&content_type, &fallback_ext);
        let file_name = format!(
            "{:03}_{}_{}.{}",
            row.line_index, safe_speaker, safe_content, ext
        );
        entries.push((file_name, bytes));
    }

    if entries.is_empty() {
        return Err(AppError::invalid_params("no downloadable voice lines"));
    }

    let bytes = build_zip_archive(entries)?;
    let project_name: Option<(String,)> =
        sqlx::query_as("SELECT name FROM projects WHERE id = ? LIMIT 1")
            .bind(&project_id)
            .fetch_optional(&state.mysql)
            .await?;
    let resolved_project_name = project_name
        .map(|item| item.0)
        .filter(|item| !item.trim().is_empty())
        .unwrap_or_else(|| "project".to_string());
    let zip_name = format!("{}_voices.zip", resolved_project_name.trim());
    zip_attachment_response(bytes, &zip_name)
}

async fn route_video_proxy(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
    Query(query): Query<VideoProxyQuery>,
) -> Result<Response, AppError> {
    verify_project_owner(&state, &project_id, &user.id).await?;
    let source = query
        .key
        .or(query.url)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .ok_or_else(|| AppError::invalid_params("key is required"))?;

    let fetch_url = resolve_media_fetch_url(&state, &source).await?;
    let upstream = reqwest::get(&fetch_url)
        .await
        .map_err(|err| AppError::internal(format!("failed to fetch proxy source: {err}")))?;

    if !upstream.status().is_success() {
        return Err(AppError::internal(format!(
            "failed to fetch proxy source: status {}",
            upstream.status()
        )));
    }

    let content_type = upstream
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|item| item.to_string())
        .unwrap_or_else(|| "video/mp4".to_string());
    let content_length = upstream
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .map(|item| item.to_string());

    let bytes = upstream
        .bytes()
        .await
        .map_err(|err| AppError::internal(format!("failed to read proxy source bytes: {err}")))?;
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "no-cache");
    if let Some(content_length) = content_length {
        builder = builder.header(header::CONTENT_LENGTH, content_length);
    }
    builder
        .body(Body::from(bytes))
        .map_err(|err| AppError::internal(format!("failed to build proxy response: {err}")))
}

async fn handle_copy_from_global(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
    body: Value,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;

    let novel_id = get_novel_id(state, project_id).await?;
    let source_type =
        body_string(&body, "type").ok_or_else(|| AppError::invalid_params("type is required"))?;
    let target_id = body_string(&body, "targetId")
        .ok_or_else(|| AppError::invalid_params("targetId is required"))?;
    let global_asset_id = body_string(&body, "globalAssetId")
        .ok_or_else(|| AppError::invalid_params("globalAssetId is required"))?;

    match source_type.as_str() {
        "character" => {
            let exists: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM novel_promotion_characters WHERE id = ? AND novelPromotionProjectId = ? LIMIT 1",
            )
            .bind(&target_id)
            .bind(&novel_id)
            .fetch_optional(&state.mysql)
            .await?;
            if exists.is_none() {
                return Err(AppError::not_found("character not found"));
            }

            let global_character = sqlx::query_as::<_, GlobalCharacterSourceRow>(
                "SELECT name, aliases, profileData, voiceId, voiceType, customVoiceUrl, customVoiceMediaId FROM global_characters WHERE id = ? AND userId = ? LIMIT 1",
            )
            .bind(&global_asset_id)
            .bind(&user.id)
            .fetch_optional(&state.mysql)
            .await?;

            let Some(global_character) = global_character else {
                return Err(AppError::not_found("global character not found"));
            };

            let global_appearances = sqlx::query_as::<_, GlobalCharacterAppearanceSourceRow>(
                "SELECT appearanceIndex, changeReason, description, descriptions, imageUrl, imageMediaId, imageUrls, selectedIndex, previousImageUrl, previousDescription, previousDescriptions FROM global_character_appearances WHERE characterId = ? ORDER BY appearanceIndex ASC",
            )
            .bind(&global_asset_id)
            .fetch_all(&state.mysql)
            .await?;

            let mut tx = state.mysql.begin().await?;
            sqlx::query("DELETE FROM character_appearances WHERE characterId = ?")
                .bind(&target_id)
                .execute(&mut *tx)
                .await?;

            for item in &global_appearances {
                sqlx::query(
                    "INSERT INTO character_appearances (id, characterId, appearanceIndex, changeReason, description, descriptions, imageUrl, imageMediaId, imageUrls, selectedIndex, previousImageUrl, previousDescription, previousDescriptions, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
                )
                .bind(Uuid::new_v4().to_string())
                .bind(&target_id)
                .bind(item.appearance_index)
                .bind(&item.change_reason)
                .bind(&item.description)
                .bind(&item.descriptions)
                .bind(&item.image_url)
                .bind(&item.image_media_id)
                .bind(&item.image_urls)
                .bind(item.selected_index)
                .bind(&item.previous_image_url)
                .bind(&item.previous_description)
                .bind(&item.previous_descriptions)
                .execute(&mut *tx)
                .await?;
            }

            sqlx::query(
                "UPDATE novel_promotion_characters SET name = ?, aliases = ?, profileData = ?, sourceGlobalCharacterId = ?, profileConfirmed = true, voiceId = ?, voiceType = ?, customVoiceUrl = ?, customVoiceMediaId = ?, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(global_character.name)
            .bind(global_character.aliases)
            .bind(global_character.profile_data)
            .bind(&global_asset_id)
            .bind(global_character.voice_id)
            .bind(global_character.voice_type)
            .bind(global_character.custom_voice_url)
            .bind(global_character.custom_voice_media_id)
            .bind(&target_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;

            Ok(Json(json!({
              "success": true,
              "character": { "id": target_id },
              "copiedAppearancesCount": global_appearances.len(),
            })))
        }
        "location" => {
            let exists: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM novel_promotion_locations WHERE id = ? AND novelPromotionProjectId = ? LIMIT 1",
            )
            .bind(&target_id)
            .bind(&novel_id)
            .fetch_optional(&state.mysql)
            .await?;
            if exists.is_none() {
                return Err(AppError::not_found("location not found"));
            }

            let global_location = sqlx::query_as::<_, GlobalLocationSourceRow>(
                "SELECT name, summary FROM global_locations WHERE id = ? AND userId = ? LIMIT 1",
            )
            .bind(&global_asset_id)
            .bind(&user.id)
            .fetch_optional(&state.mysql)
            .await?;

            let Some(global_location) = global_location else {
                return Err(AppError::not_found("global location not found"));
            };

            let global_images = sqlx::query_as::<_, GlobalLocationImageSourceRow>(
                "SELECT imageIndex, description, imageUrl, imageMediaId, isSelected, previousImageUrl, previousDescription FROM global_location_images WHERE locationId = ? ORDER BY imageIndex ASC",
            )
            .bind(&global_asset_id)
            .fetch_all(&state.mysql)
            .await?;

            let mut tx = state.mysql.begin().await?;
            sqlx::query("DELETE FROM location_images WHERE locationId = ?")
                .bind(&target_id)
                .execute(&mut *tx)
                .await?;

            let mut selected_image_id: Option<String> = None;
            let mut first_image_id: Option<String> = None;
            for item in &global_images {
                let image_id = Uuid::new_v4().to_string();
                if first_image_id.is_none() {
                    first_image_id = Some(image_id.clone());
                }
                if item.is_selected {
                    selected_image_id = Some(image_id.clone());
                }

                sqlx::query(
                    "INSERT INTO location_images (id, locationId, imageIndex, description, imageUrl, imageMediaId, isSelected, previousImageUrl, previousDescription, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
                )
                .bind(&image_id)
                .bind(&target_id)
                .bind(item.image_index)
                .bind(&item.description)
                .bind(&item.image_url)
                .bind(&item.image_media_id)
                .bind(item.is_selected)
                .bind(&item.previous_image_url)
                .bind(&item.previous_description)
                .execute(&mut *tx)
                .await?;
            }
            if selected_image_id.is_none() {
                selected_image_id = first_image_id;
            }

            sqlx::query(
                "UPDATE novel_promotion_locations SET name = ?, summary = ?, sourceGlobalLocationId = ?, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(global_location.name)
            .bind(global_location.summary)
            .bind(&global_asset_id)
            .bind(&target_id)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "UPDATE novel_promotion_locations SET selectedImageId = ?, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(selected_image_id)
            .bind(&target_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;

            Ok(Json(json!({
              "success": true,
              "location": { "id": target_id },
              "copiedImagesCount": global_images.len(),
            })))
        }
        "voice" => {
            let exists: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM novel_promotion_characters WHERE id = ? AND novelPromotionProjectId = ? LIMIT 1",
            )
            .bind(&target_id)
            .bind(&novel_id)
            .fetch_optional(&state.mysql)
            .await?;
            if exists.is_none() {
                return Err(AppError::not_found("character not found"));
            }

            let voice = sqlx::query_as::<_, GlobalVoiceSourceRow>(
                "SELECT name, voiceId, voiceType, customVoiceUrl, customVoiceMediaId FROM global_voices WHERE id = ? AND userId = ? LIMIT 1",
            )
            .bind(&global_asset_id)
            .bind(&user.id)
            .fetch_optional(&state.mysql)
            .await?;
            let Some(voice) = voice else {
                return Err(AppError::not_found("global voice not found"));
            };

            sqlx::query(
                "UPDATE novel_promotion_characters SET voiceId = ?, voiceType = ?, customVoiceUrl = ?, customVoiceMediaId = ?, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(voice.voice_id)
            .bind(voice.voice_type)
            .bind(voice.custom_voice_url)
            .bind(voice.custom_voice_media_id)
            .bind(&target_id)
            .execute(&state.mysql)
            .await?;

            Ok(Json(json!({
              "success": true,
              "character": { "id": target_id },
              "voiceName": voice.name,
            })))
        }
        _ => Err(AppError::invalid_params("unsupported type")),
    }
}

async fn handle_task_submission(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
    path: &str,
    headers: &HeaderMap,
    body: Value,
) -> Result<Option<Json<Value>>, AppError> {
    let mapping: HashMap<&str, (&str, &str, &str)> = HashMap::from([
        ("analyze", ("analyze_novel", "project", "project")),
        ("analyze-global", ("analyze_global", "project", "project")),
        (
            "analyze-shot-variants",
            ("analyze_shot_variants", "shot", "shot"),
        ),
        (
            "story-to-script-stream",
            ("story_to_script_run", "episode", "episode"),
        ),
        (
            "script-to-storyboard-stream",
            ("script_to_storyboard_run", "episode", "episode"),
        ),
        (
            "screenplay-conversion",
            ("screenplay_convert", "episode", "episode"),
        ),
        ("voice-analyze", ("voice_analyze", "episode", "episode")),
        (
            "ai-create-character",
            ("ai_create_character", "project", "project"),
        ),
        (
            "ai-create-location",
            ("ai_create_location", "project", "project"),
        ),
        (
            "ai-modify-appearance",
            ("ai_modify_appearance", "character", "character"),
        ),
        (
            "ai-modify-location",
            ("ai_modify_location", "location", "location"),
        ),
        (
            "ai-modify-shot-prompt",
            ("ai_modify_shot_prompt", "panel", "panel"),
        ),
        (
            "reference-to-character",
            ("reference_to_character", "character", "character"),
        ),
        (
            "generate-image",
            ("image_character", "character", "character"),
        ),
        (
            "generate-character-image",
            ("image_character", "character", "character"),
        ),
        (
            "modify-asset-image",
            ("modify_asset_image", "asset", "asset"),
        ),
        (
            "modify-storyboard-image",
            ("modify_asset_image", "storyboard", "storyboard"),
        ),
        (
            "regenerate-group",
            ("regenerate_group", "storyboard", "storyboard"),
        ),
        ("regenerate-panel-image", ("image_panel", "panel", "panel")),
        (
            "regenerate-single-image",
            ("image_location", "location", "location"),
        ),
        (
            "regenerate-storyboard-text",
            ("regenerate_storyboard_text", "storyboard", "storyboard"),
        ),
        ("insert-panel", ("insert_panel", "storyboard", "storyboard")),
        ("panel-variant", ("panel_variant", "panel", "panel")),
        ("generate-video", ("video_panel", "panel", "panel")),
        ("lip-sync", ("lip_sync", "panel", "panel")),
        ("voice-design", ("voice_design", "voice", "voice")),
        ("voice-generate", ("voice_line", "voice-line", "voice-line")),
        (
            "episodes/split",
            ("episode_split_llm", "project", "project"),
        ),
        (
            "character-profile/confirm",
            ("character_profile_confirm", "character", "character"),
        ),
        (
            "character-profile/batch-confirm",
            (
                "character_profile_batch_confirm",
                "project",
                "project-character-profile",
            ),
        ),
    ]);

    let Some((task_type, fallback_target_type, fallback_target_id)) = mapping.get(path) else {
        return Ok(None);
    };

    validate_task_submission_payload(path, &body, headers)?;

    let mut payload_body = body;
    if path == "generate-character-image"
        && let Some(character_id) = body_string(&payload_body, "characterId")
        && let Some(object) = payload_body.as_object_mut()
    {
        object
            .entry("type".to_string())
            .or_insert_with(|| json!("character"));
        object
            .entry("id".to_string())
            .or_insert_with(|| json!(character_id));
    }

    let (target_type, target_id) =
        normalize_task_target(&payload_body, fallback_target_type, fallback_target_id);
    let priority = task_submission_priority(path);
    let accept_language = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|raw| raw.to_str().ok());
    let result = submit_novel_task(
        state,
        user,
        project_id,
        task_type,
        &target_type,
        &target_id,
        payload_body,
        priority,
        accept_language,
    )
    .await?;
    let mut payload = result.0;
    if let Some(object) = payload.as_object_mut() {
        match path {
            "generate-video" => {
                object.insert("tasks".to_string(), json!([]));
                object.insert("total".to_string(), json!(1));
            }
            "panel-variant" => {
                object.insert("panelId".to_string(), json!(target_id));
            }
            "voice-generate" => {
                let task_id = object.get("taskId").cloned().unwrap_or(Value::Null);
                object.insert("taskIds".to_string(), json!([task_id]));
                object.insert("total".to_string(), json!(1));
                object.remove("deduped");
                object.remove("status");
            }
            _ => {}
        }
    }
    Ok(Some(Json(payload)))
}

pub async fn dispatch(
    State(state): State<AppState>,
    user: AuthUser,
    method: Method,
    headers: HeaderMap,
    Path((project_id, path)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    body: Bytes,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(&state, &project_id, &user.id).await?;

    let path = path.trim_matches('/').to_string();
    let body_json = parse_body_json(&body)?;

    if let Some(result) = handle_task_submission(
        &state,
        &user,
        &project_id,
        &path,
        &headers,
        body_json.clone(),
    )
    .await?
    {
        return Ok(result);
    }

    let segments = path
        .split('/')
        .filter(|item| !item.trim().is_empty())
        .collect::<Vec<_>>();

    match (method.as_str(), segments.as_slice()) {
        ("GET", ["assets"]) => handle_assets(&state, &user, &project_id).await,
        ("GET", ["characters"]) => {
            let assets = handle_assets(&state, &user, &project_id).await?.0;
            let characters = assets
                .get("characters")
                .cloned()
                .unwrap_or_else(|| json!([]));
            Ok(Json(json!({ "characters": characters })))
        }
        ("GET", ["locations"]) => {
            let assets = handle_assets(&state, &user, &project_id).await?.0;
            let locations = assets
                .get("locations")
                .cloned()
                .unwrap_or_else(|| json!([]));
            Ok(Json(json!({ "locations": locations })))
        }
        ("GET", ["episodes"]) => handle_episodes_get(&state, &user, &project_id).await,
        ("POST", ["episodes"]) => handle_episodes_post(&state, &user, &project_id, body_json).await,
        ("POST", ["episodes", "batch"]) => {
            if let Some(episodes) = body_json.get("episodes").and_then(Value::as_array) {
                if body_bool(&body_json, "clearExisting") == Some(true) {
                    let novel_id = get_novel_id(&state, &project_id).await?;
                    sqlx::query(
                        "DELETE FROM novel_promotion_episodes WHERE novelPromotionProjectId = ?",
                    )
                    .bind(&novel_id)
                    .execute(&state.mysql)
                    .await?;
                }

                let mut created = Vec::<Value>::new();
                for item in episodes {
                    let created_one =
                        handle_episodes_post(&state, &user, &project_id, item.clone()).await?;
                    created.push(
                        created_one
                            .0
                            .get("episode")
                            .cloned()
                            .unwrap_or_else(|| json!({})),
                    );
                }
                Ok(Json(json!({
                  "success": true,
                  "episodes": created,
                  "message": "episodes batch created",
                })))
            } else {
                Err(AppError::invalid_params("episodes array is required"))
            }
        }
        ("POST", ["episodes", "split-by-markers"]) => {
            let content = body_string(&body_json, "content")
                .ok_or_else(|| AppError::invalid_params("content is required"))?;
            let sections = content
                .split("\n\n")
                .map(|item| item.trim())
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>();
            let mut created = Vec::new();
            for (index, section) in sections.iter().enumerate() {
                let item = json!({
                  "episodeNumber": i32::try_from(index + 1).unwrap_or(1),
                  "name": format!("Episode {}", index + 1),
                  "novelText": section,
                });
                let episode = handle_episodes_post(&state, &user, &project_id, item).await?;
                created.push(
                    episode
                        .0
                        .get("episode")
                        .cloned()
                        .unwrap_or_else(|| json!({})),
                );
            }
            Ok(Json(json!({
              "success": true,
              "episodes": created,
              "markerType": "double-newline",
              "method": "split-by-markers",
            })))
        }
        ("GET", ["episodes", episode_id]) => {
            handle_episode_get(&state, &user, &project_id, episode_id).await
        }
        ("PATCH", ["episodes", episode_id]) => {
            handle_episode_patch(&state, &user, &project_id, episode_id, body_json).await
        }
        ("DELETE", ["episodes", episode_id]) => {
            handle_episode_delete(&state, &user, &project_id, episode_id).await
        }
        (method, ["character"]) if method == "POST" || method == "PATCH" || method == "DELETE" => {
            handle_character_route(
                &state,
                &user,
                &project_id,
                method,
                &query,
                &headers,
                body_json,
            )
            .await
        }
        (method, ["characters"]) if method == "POST" || method == "PATCH" || method == "DELETE" => {
            handle_character_route(
                &state,
                &user,
                &project_id,
                method,
                &query,
                &headers,
                body_json,
            )
            .await
        }
        (method, ["location"]) if method == "POST" || method == "PATCH" || method == "DELETE" => {
            handle_location_route(
                &state,
                &user,
                &project_id,
                method,
                &query,
                &headers,
                body_json,
            )
            .await
        }
        (method, ["locations"]) if method == "POST" || method == "PATCH" || method == "DELETE" => {
            handle_location_route(
                &state,
                &user,
                &project_id,
                method,
                &query,
                &headers,
                body_json,
            )
            .await
        }
        (method, ["voice-lines"])
            if method == "GET" || method == "POST" || method == "PATCH" || method == "DELETE" =>
        {
            handle_voice_lines(&state, &user, &project_id, method, &query, body_json).await
        }
        (method, ["editor"]) if method == "GET" || method == "PUT" || method == "DELETE" => {
            handle_editor(&state, &user, &project_id, method, &query, body_json).await
        }
        (method, ["storyboards"]) if method == "GET" || method == "PATCH" => {
            handle_storyboards(&state, &user, &project_id, method, &query, body_json).await
        }
        (method, ["panel"])
            if method == "POST" || method == "PATCH" || method == "PUT" || method == "DELETE" =>
        {
            handle_panel(&state, &user, &project_id, method, &query, body_json).await
        }
        (method, ["storyboard-group"])
            if method == "POST" || method == "PUT" || method == "DELETE" =>
        {
            handle_storyboard_group(
                &state,
                &user,
                &project_id,
                method,
                &query,
                &headers,
                body_json,
            )
            .await
        }
        ("POST", ["clips"]) => {
            handle_clips(&state, &user, &project_id, "POST", None, body_json).await
        }
        ("PATCH", ["clips", clip_id]) => {
            handle_clips(
                &state,
                &user,
                &project_id,
                "PATCH",
                Some(clip_id),
                body_json,
            )
            .await
        }
        (method, ["speaker-voice"]) if method == "GET" || method == "PATCH" => {
            handle_speaker_voice(&state, &user, &project_id, method, &query, body_json).await
        }
        ("POST", ["video-urls"]) => {
            handle_video_urls(&state, &user, &project_id, &headers, body_json).await
        }
        ("POST", ["copy-from-global"]) => {
            handle_copy_from_global(&state, &user, &project_id, body_json).await
        }
        ("POST", ["update-location"]) => {
            let location_id = body_string(&body_json, "locationId")
                .ok_or_else(|| AppError::invalid_params("locationId is required"))?;
            let image_index = body_i32(&body_json, "imageIndex")
                .ok_or_else(|| AppError::invalid_params("imageIndex is required"))?;
            let description = body_string(&body_json, "description")
                .map(|item| normalize_location_description(&item));
            sqlx::query("UPDATE location_images SET description = ?, updatedAt = NOW(3) WHERE locationId = ? AND imageIndex = ?")
                .bind(description)
                .bind(location_id)
                .bind(image_index)
                .execute(&state.mysql)
                .await?;
            Ok(Json(json!({ "success": true })))
        }
        ("POST", ["update-prompt"]) => {
            let shot_id = body_string(&body_json, "shotId")
                .ok_or_else(|| AppError::invalid_params("shotId is required"))?;
            let field = body_string(&body_json, "field")
                .ok_or_else(|| AppError::invalid_params("field is required"))?;
            let value = body_string(&body_json, "value");

            let allowed_fields = [
                "imagePrompt",
                "scale",
                "module",
                "focus",
                "sequence",
                "locations",
                "characters",
                "plot",
                "pov",
                "zhSummarize",
            ];
            if !allowed_fields.contains(&field.as_str()) {
                return Err(AppError::invalid_params("field is not editable"));
            }

            let sql = format!(
                "UPDATE novel_promotion_shots SET {} = ?, updatedAt = NOW(3) WHERE id = ?",
                field
            );
            sqlx::query(&sql)
                .bind(value)
                .bind(&shot_id)
                .execute(&state.mysql)
                .await?;
            Ok(Json(json!({
              "success": true,
              "shot": { "id": shot_id, "field": field },
            })))
        }
        ("POST", ["select-character-image"]) => {
            let appearance_id = body_string(&body_json, "appearanceId")
                .ok_or_else(|| AppError::invalid_params("appearanceId is required"))?;
            let selected_index = body_i32(&body_json, "selectedIndex")
                .ok_or_else(|| AppError::invalid_params("selectedIndex is required"))?;
            sqlx::query("UPDATE character_appearances SET selectedIndex = ?, updatedAt = NOW(3) WHERE id = ?")
                .bind(selected_index)
                .bind(&appearance_id)
                .execute(&state.mysql)
                .await?;
            let selected_image: Option<(Option<String>,)> =
                sqlx::query_as("SELECT imageUrl FROM character_appearances WHERE id = ? LIMIT 1")
                    .bind(&appearance_id)
                    .fetch_optional(&state.mysql)
                    .await?;
            Ok(Json(json!({
              "success": true,
              "selectedIndex": selected_index,
              "imageUrl": selected_image.and_then(|item| item.0),
            })))
        }
        ("POST", ["select-location-image"]) => {
            let location_id = body_string(&body_json, "locationId")
                .ok_or_else(|| AppError::invalid_params("locationId is required"))?;
            let selected_index = body_i32(&body_json, "selectedIndex")
                .ok_or_else(|| AppError::invalid_params("selectedIndex is required"))?;

            let mut tx = state.mysql.begin().await?;
            sqlx::query("UPDATE location_images SET isSelected = false, updatedAt = NOW(3) WHERE locationId = ?")
                .bind(&location_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE location_images SET isSelected = true, updatedAt = NOW(3) WHERE locationId = ? AND imageIndex = ?")
                .bind(&location_id)
                .bind(selected_index)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE novel_promotion_locations SET selectedImageId = (SELECT id FROM location_images WHERE locationId = ? AND imageIndex = ? LIMIT 1), updatedAt = NOW(3) WHERE id = ?")
                .bind(&location_id)
                .bind(selected_index)
                .bind(&location_id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            let selected_image: Option<(Option<String>,)> = sqlx::query_as(
                "SELECT imageUrl FROM location_images WHERE locationId = ? AND imageIndex = ? LIMIT 1",
            )
            .bind(&location_id)
            .bind(selected_index)
            .fetch_optional(&state.mysql)
            .await?;
            Ok(Json(json!({
              "success": true,
              "selectedIndex": selected_index,
              "imageUrl": selected_image.and_then(|item| item.0),
            })))
        }
        ("POST", ["panel-link"]) => {
            let panel_id = body_string(&body_json, "panelId")
                .ok_or_else(|| AppError::invalid_params("panelId is required"))?;
            sqlx::query("UPDATE novel_promotion_panels SET linkedToNextPanel = ?, updatedAt = NOW(3) WHERE id = ?")
                .bind(body_bool(&body_json, "linkedToNextPanel"))
                .bind(panel_id)
                .execute(&state.mysql)
                .await?;
            Ok(Json(json!({ "success": true })))
        }
        ("POST", ["character-voice"]) => {
            let character_id = body_string(&body_json, "characterId")
                .ok_or_else(|| AppError::invalid_params("characterId is required"))?;
            let custom_voice_url = body_string(&body_json, "customVoiceUrl");
            sqlx::query(
                "UPDATE novel_promotion_characters SET voiceId = ?, voiceType = ?, customVoiceUrl = ?, customVoiceMediaId = ?, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(body_string(&body_json, "voiceId"))
            .bind(body_string(&body_json, "voiceType"))
            .bind(custom_voice_url.clone())
            .bind(body_string(&body_json, "customVoiceMediaId"))
            .bind(&character_id)
            .execute(&state.mysql)
            .await?;
            Ok(Json(json!({
              "success": true,
              "character": { "id": character_id },
              "audioUrl": custom_voice_url,
            })))
        }
        ("PATCH", ["character-voice"]) => {
            let character_id = body_string(&body_json, "characterId")
                .ok_or_else(|| AppError::invalid_params("characterId is required"))?;
            sqlx::query(
                "UPDATE novel_promotion_characters SET voiceId = ?, voiceType = ?, customVoiceUrl = ?, customVoiceMediaId = ?, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(body_string(&body_json, "voiceId"))
            .bind(body_string(&body_json, "voiceType"))
            .bind(body_string(&body_json, "customVoiceUrl"))
            .bind(body_string(&body_json, "customVoiceMediaId"))
            .bind(&character_id)
            .execute(&state.mysql)
            .await?;

            Ok(Json(json!({
              "success": true,
              "character": { "id": character_id },
            })))
        }
        ("POST", ["location", "confirm-selection"]) => {
            let location_id = body_string(&body_json, "locationId")
                .ok_or_else(|| AppError::invalid_params("locationId is required"))?;
            let selected_image_id = body_string(&body_json, "selectedImageId")
                .ok_or_else(|| AppError::invalid_params("selectedImageId is required"))?;
            let mut tx = state.mysql.begin().await?;
            sqlx::query("UPDATE location_images SET isSelected = false, updatedAt = NOW(3) WHERE locationId = ?")
                .bind(&location_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "UPDATE location_images SET isSelected = true, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(&selected_image_id)
            .execute(&mut *tx)
            .await?;
            sqlx::query("UPDATE novel_promotion_locations SET selectedImageId = ?, updatedAt = NOW(3) WHERE id = ?")
                .bind(&selected_image_id)
                .bind(&location_id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            Ok(Json(json!({
              "success": true,
              "deletedCount": 0,
              "message": "location selection confirmed",
            })))
        }
        ("POST", ["character", "confirm-selection"]) => {
            let appearance_id = body_string(&body_json, "appearanceId")
                .ok_or_else(|| AppError::invalid_params("appearanceId is required"))?;
            let selected_index = body_i32(&body_json, "selectedIndex")
                .ok_or_else(|| AppError::invalid_params("selectedIndex is required"))?;
            sqlx::query("UPDATE character_appearances SET selectedIndex = ?, profileConfirmed = true, updatedAt = NOW(3) WHERE id = ?")
                .bind(selected_index)
                .bind(&appearance_id)
                .execute(&state.mysql)
                .await?;
            Ok(Json(json!({
              "success": true,
              "deletedCount": 0,
              "message": "character selection confirmed",
            })))
        }
        (method, ["character", "appearance"])
            if method == "POST" || method == "PATCH" || method == "DELETE" =>
        {
            let character_id = body_string(&body_json, "characterId")
                .or_else(|| query_string(&query, "characterId"))
                .ok_or_else(|| AppError::invalid_params("characterId is required"))?;
            let character_exists: Option<(String,)> = sqlx::query_as(
                "SELECT c.id
                 FROM novel_promotion_characters c
                 INNER JOIN novel_promotion_projects p ON p.id = c.novelPromotionProjectId
                 WHERE c.id = ? AND p.projectId = ?
                 LIMIT 1",
            )
            .bind(&character_id)
            .bind(&project_id)
            .fetch_optional(&state.mysql)
            .await?;
            if character_exists.is_none() {
                return Err(AppError::not_found("character not found"));
            }
            match method {
                "POST" => {
                    let change_reason = body_string(&body_json, "changeReason")
                        .ok_or_else(|| AppError::invalid_params("changeReason is required"))?;
                    let description = body_string(&body_json, "description")
                        .ok_or_else(|| AppError::invalid_params("description is required"))?;
                    let appearance_index = if let Some(index) =
                        body_i32(&body_json, "appearanceIndex")
                    {
                        index
                    } else {
                        let max: Option<(i32,)> = sqlx::query_as(
                            "SELECT appearanceIndex FROM character_appearances WHERE characterId = ? ORDER BY appearanceIndex DESC LIMIT 1",
                        )
                        .bind(&character_id)
                        .fetch_optional(&state.mysql)
                        .await?;
                        max.map(|item| item.0 + 1).unwrap_or(0)
                    };
                    let appearance_id = Uuid::new_v4().to_string();
                    let descriptions = if body_json.get("descriptions").is_some() {
                        normalize_optional_json(body_json.get("descriptions").cloned())
                    } else {
                        normalize_optional_json(Some(json!([description.clone()])))
                    };
                    let image_urls = if body_json.get("imageUrls").is_some() {
                        normalize_optional_json(body_json.get("imageUrls").cloned())
                    } else {
                        normalize_optional_json(Some(json!([])))
                    };
                    sqlx::query(
                        "INSERT INTO character_appearances (id, characterId, appearanceIndex, changeReason, description, descriptions, imageUrl, imageUrls, selectedIndex, previousImageUrls, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
                    )
                    .bind(&appearance_id)
                    .bind(&character_id)
                    .bind(appearance_index)
                    .bind(change_reason)
                    .bind(description)
                    .bind(descriptions)
                    .bind(body_string(&body_json, "imageUrl"))
                    .bind(image_urls)
                    .bind(body_i32(&body_json, "selectedIndex"))
                    .bind(normalize_optional_json(Some(json!([]))))
                    .execute(&state.mysql)
                    .await?;

                    let appearance = sqlx::query_as::<_, CharacterAppearanceRow>(
                        "SELECT id, characterId, appearanceIndex, description, descriptions, imageUrl, imageMediaId, imageUrls, selectedIndex, previousImageUrl, previousDescription, previousDescriptions, createdAt, updatedAt FROM character_appearances WHERE id = ? LIMIT 1",
                    )
                    .bind(&appearance_id)
                    .fetch_one(&state.mysql)
                    .await?;

                    Ok(Json(json!({
                      "success": true,
                      "appearance": appearance,
                    })))
                }
                "PATCH" => {
                    let appearance_id = body_string(&body_json, "appearanceId")
                        .or_else(|| query_string(&query, "appearanceId"))
                        .ok_or_else(|| AppError::invalid_params("appearanceId is required"))?;
                    let description = body_string(&body_json, "description")
                        .ok_or_else(|| AppError::invalid_params("description is required"))?;
                    let description_index = body_i32(&body_json, "descriptionIndex").unwrap_or(0);
                    let current: Option<(Option<String>,)> = sqlx::query_as(
                        "SELECT descriptions FROM character_appearances WHERE id = ? AND characterId = ? LIMIT 1",
                    )
                    .bind(&appearance_id)
                    .bind(&character_id)
                    .fetch_optional(&state.mysql)
                    .await?;
                    let Some((raw_descriptions,)) = current else {
                        return Err(AppError::not_found("appearance not found"));
                    };
                    let mut descriptions = parse_json_string_array(raw_descriptions.as_deref());
                    let index = usize::try_from(description_index.max(0)).unwrap_or(0);
                    if index < descriptions.len() {
                        descriptions[index] = description.clone();
                    } else {
                        descriptions.push(description.clone());
                    }
                    sqlx::query(
                        "UPDATE character_appearances SET description = ?, descriptions = ?, updatedAt = NOW(3) WHERE id = ? AND characterId = ?",
                    )
                    .bind(description)
                    .bind(normalize_optional_json(Some(json!(descriptions))))
                    .bind(&appearance_id)
                    .bind(&character_id)
                    .execute(&state.mysql)
                    .await?;

                    Ok(Json(json!({ "success": true })))
                }
                "DELETE" => {
                    let appearance_id = body_string(&body_json, "appearanceId")
                        .or_else(|| query_string(&query, "appearanceId"))
                        .ok_or_else(|| AppError::invalid_params("appearanceId is required"))?;
                    let current: Option<(i32,)> = sqlx::query_as(
                        "SELECT appearanceIndex FROM character_appearances WHERE id = ? AND characterId = ? LIMIT 1",
                    )
                    .bind(&appearance_id)
                    .bind(&character_id)
                    .fetch_optional(&state.mysql)
                    .await?;
                    let Some((deleted_index,)) = current else {
                        return Err(AppError::not_found("appearance not found"));
                    };

                    let count: Option<(i64,)> = sqlx::query_as(
                        "SELECT COUNT(*) FROM character_appearances WHERE characterId = ?",
                    )
                    .bind(&character_id)
                    .fetch_optional(&state.mysql)
                    .await?;
                    if count.map(|item| item.0).unwrap_or(0) <= 1 {
                        return Err(AppError::invalid_params("cannot delete last appearance"));
                    }

                    let mut tx = state.mysql.begin().await?;
                    let deleted_images = sqlx::query(
                        "DELETE FROM character_appearances WHERE id = ? AND characterId = ?",
                    )
                    .bind(&appearance_id)
                    .bind(&character_id)
                    .execute(&mut *tx)
                    .await?
                    .rows_affected();
                    sqlx::query(
                        "UPDATE character_appearances
                         SET appearanceIndex = appearanceIndex - 1, updatedAt = NOW(3)
                         WHERE characterId = ? AND appearanceIndex > ?",
                    )
                    .bind(&character_id)
                    .bind(deleted_index)
                    .execute(&mut *tx)
                    .await?;
                    tx.commit().await?;

                    Ok(Json(
                        json!({ "success": true, "deletedImages": deleted_images }),
                    ))
                }
                _ => Err(AppError::invalid_params(
                    "unsupported method for /character/appearance",
                )),
            }
        }
        ("POST", ["upload-asset-image"]) => {
            let asset_type = body_string(&body_json, "type")
                .ok_or_else(|| AppError::invalid_params("type is required"))?;
            let image_url = body_string(&body_json, "imageUrl")
                .ok_or_else(|| AppError::invalid_params("imageUrl is required"))?;
            if asset_type == "character" {
                let appearance_id = body_string(&body_json, "appearanceId")
                    .ok_or_else(|| AppError::invalid_params("appearanceId is required"))?;
                sqlx::query(
                    "UPDATE character_appearances SET previousImageUrl = imageUrl, previousDescription = description, imageUrl = ?, description = COALESCE(?, description), updatedAt = NOW(3) WHERE id = ?",
                )
                .bind(&image_url)
                .bind(body_string(&body_json, "description"))
                .bind(&appearance_id)
                .execute(&state.mysql)
                .await?;
            } else if asset_type == "location" {
                let location_id = body_string(&body_json, "id")
                    .ok_or_else(|| AppError::invalid_params("id is required"))?;
                let image_index = body_i32(&body_json, "imageIndex").unwrap_or(0);
                let exists: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM location_images WHERE locationId = ? AND imageIndex = ? LIMIT 1",
                )
                .bind(&location_id)
                .bind(image_index)
                .fetch_optional(&state.mysql)
                .await?;

                if let Some((image_id,)) = exists {
                    sqlx::query(
                        "UPDATE location_images SET previousImageUrl = imageUrl, previousDescription = description, imageUrl = ?, description = COALESCE(?, description), updatedAt = NOW(3) WHERE id = ?",
                    )
                    .bind(&image_url)
                    .bind(body_string(&body_json, "description"))
                    .bind(&image_id)
                    .execute(&state.mysql)
                    .await?;
                } else {
                    sqlx::query(
                        "INSERT INTO location_images (id, locationId, imageIndex, description, imageUrl, isSelected, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, false, NOW(3), NOW(3))",
                    )
                    .bind(Uuid::new_v4().to_string())
                    .bind(&location_id)
                    .bind(image_index)
                    .bind(body_string(&body_json, "description"))
                    .bind(&image_url)
                    .execute(&state.mysql)
                    .await?;
                }
            } else {
                return Err(AppError::invalid_params("unsupported upload type"));
            }

            Ok(Json(json!({
              "success": true,
              "imageKey": image_url,
              "imageIndex": body_i32(&body_json, "imageIndex").unwrap_or(0),
            })))
        }
        ("POST", ["update-appearance"]) => {
            let appearance_id = body_string(&body_json, "appearanceId")
                .ok_or_else(|| AppError::invalid_params("appearanceId is required"))?;
            sqlx::query(
                "UPDATE character_appearances SET description = ?, descriptions = ?, imageUrl = ?, imageUrls = ?, selectedIndex = ?, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(body_string(&body_json, "description"))
            .bind(normalize_optional_json(body_json.get("descriptions").cloned()))
            .bind(body_string(&body_json, "imageUrl"))
            .bind(normalize_optional_json(body_json.get("imageUrls").cloned()))
            .bind(body_i32(&body_json, "selectedIndex"))
            .bind(appearance_id)
            .execute(&state.mysql)
            .await?;
            Ok(Json(json!({ "success": true })))
        }
        ("POST", ["update-asset-label"]) => {
            let asset_type = body_string(&body_json, "type")
                .ok_or_else(|| AppError::invalid_params("type is required"))?;
            let id = body_string(&body_json, "id")
                .ok_or_else(|| AppError::invalid_params("id is required"))?;
            let old_url = body_string(&body_json, "oldUrl")
                .ok_or_else(|| AppError::invalid_params("oldUrl is required"))?;
            let new_url = body_string(&body_json, "newUrl")
                .ok_or_else(|| AppError::invalid_params("newUrl is required"))?;

            let affected = if asset_type == "character" {
                sqlx::query("UPDATE character_appearances SET imageUrl = ?, updatedAt = NOW(3) WHERE characterId = ? AND imageUrl = ?")
                    .bind(new_url)
                    .bind(id)
                    .bind(old_url)
                    .execute(&state.mysql)
                    .await?
                    .rows_affected()
            } else {
                sqlx::query("UPDATE location_images SET imageUrl = ?, updatedAt = NOW(3) WHERE locationId = ? AND imageUrl = ?")
                    .bind(new_url)
                    .bind(id)
                    .bind(old_url)
                    .execute(&state.mysql)
                    .await?
                    .rows_affected()
            };

            Ok(Json(json!({
              "success": true,
              "results": [{ "updated": affected }],
            })))
        }
        ("POST", ["cleanup-unselected-images"]) => {
            let novel_id = get_novel_id(&state, &project_id).await?;
            let appearance_rows: Vec<(String, Option<String>, Option<i32>)> = sqlx::query_as(
                "SELECT id, imageUrls, selectedIndex FROM character_appearances WHERE characterId IN (SELECT id FROM novel_promotion_characters WHERE novelPromotionProjectId = ?)",
            )
            .bind(&novel_id)
            .fetch_all(&state.mysql)
            .await?;

            let mut deleted_count: u64 = 0;

            for (appearance_id, image_urls, selected_index) in appearance_rows {
                let Some(selected_index) = selected_index else {
                    continue;
                };
                let image_urls = parse_json_string_array(image_urls.as_deref());
                if image_urls.len() <= 1 {
                    continue;
                }
                let Some(selected_url) = usize::try_from(selected_index)
                    .ok()
                    .and_then(|idx| image_urls.get(idx))
                    .cloned()
                else {
                    continue;
                };
                let removed = u64::try_from(image_urls.len().saturating_sub(1)).unwrap_or(0);
                let normalized_image_urls = serde_json::to_string(&vec![selected_url.clone()])
                    .map_err(|err| AppError::invalid_params(format!("invalid imageUrls: {err}")))?;

                sqlx::query(
                    "UPDATE character_appearances SET imageUrl = ?, imageUrls = ?, selectedIndex = 0, updatedAt = NOW(3) WHERE id = ?",
                )
                .bind(Some(selected_url))
                .bind(Some(normalized_image_urls))
                .bind(&appearance_id)
                .execute(&state.mysql)
                .await?;

                deleted_count += removed;
            }

            let location_rows: Vec<(String, Option<String>)> = sqlx::query_as(
                "SELECT id, selectedImageId FROM novel_promotion_locations WHERE novelPromotionProjectId = ?",
            )
            .bind(&novel_id)
            .fetch_all(&state.mysql)
            .await?;

            for (location_id, selected_image_id) in location_rows {
                let images: Vec<(String, i32, bool)> = sqlx::query_as(
                    "SELECT id, imageIndex, isSelected FROM location_images WHERE locationId = ? ORDER BY imageIndex ASC",
                )
                .bind(&location_id)
                .fetch_all(&state.mysql)
                .await?;
                if images.is_empty() {
                    continue;
                }

                let selected_id = selected_image_id
                    .filter(|id| images.iter().any(|(image_id, _, _)| image_id == id))
                    .or_else(|| {
                        images
                            .iter()
                            .find(|(_, _, is_selected)| *is_selected)
                            .map(|(id, _, _)| id.clone())
                    })
                    .or_else(|| images.first().map(|(id, _, _)| id.clone()));

                let Some(selected_id) = selected_id else {
                    continue;
                };

                let removed =
                    sqlx::query("DELETE FROM location_images WHERE locationId = ? AND id <> ?")
                        .bind(&location_id)
                        .bind(&selected_id)
                        .execute(&state.mysql)
                        .await?
                        .rows_affected();
                deleted_count += removed;

                sqlx::query(
                    "UPDATE location_images SET imageIndex = 0, isSelected = true, updatedAt = NOW(3) WHERE id = ?",
                )
                .bind(&selected_id)
                .execute(&state.mysql)
                .await?;

                sqlx::query(
                    "UPDATE novel_promotion_locations SET selectedImageId = ?, updatedAt = NOW(3) WHERE id = ?",
                )
                .bind(&selected_id)
                .bind(&location_id)
                .execute(&state.mysql)
                .await?;
            }

            Ok(Json(
                json!({ "success": true, "deletedCount": deleted_count }),
            ))
        }
        ("POST", ["undo-regenerate"]) => {
            let asset_type = body_string(&body_json, "type")
                .ok_or_else(|| AppError::invalid_params("type is required"))?;
            let message = if asset_type == "character" {
                let appearance_id = body_string(&body_json, "appearanceId")
                    .ok_or_else(|| AppError::invalid_params("appearanceId is required"))?;
                let affected = sqlx::query(
                    "UPDATE character_appearances SET imageUrl = previousImageUrl, previousImageUrl = NULL, description = COALESCE(previousDescription, description), previousDescription = NULL, descriptions = COALESCE(previousDescriptions, descriptions), previousDescriptions = NULL, updatedAt = NOW(3) WHERE id = ? AND previousImageUrl IS NOT NULL",
                )
                .bind(appearance_id)
                .execute(&state.mysql)
                .await?
                .rows_affected();
                if affected == 0 {
                    return Err(AppError::invalid_params("no previous version"));
                }
                "character appearance reverted"
            } else if asset_type == "location" {
                let location_id = body_string(&body_json, "id")
                    .ok_or_else(|| AppError::invalid_params("id is required"))?;
                let affected = sqlx::query(
                    "UPDATE location_images SET imageUrl = previousImageUrl, previousImageUrl = NULL, description = COALESCE(previousDescription, description), previousDescription = NULL, updatedAt = NOW(3) WHERE locationId = ? AND previousImageUrl IS NOT NULL",
                )
                .bind(location_id)
                .execute(&state.mysql)
                .await?
                .rows_affected();
                if affected == 0 {
                    return Err(AppError::invalid_params("no previous version"));
                }
                "location image reverted"
            } else if asset_type == "panel" {
                let panel_id = body_string(&body_json, "id")
                    .or_else(|| body_string(&body_json, "panelId"))
                    .ok_or_else(|| AppError::invalid_params("id is required"))?;
                let affected = sqlx::query(
                    "UPDATE novel_promotion_panels SET imageUrl = previousImageUrl, previousImageUrl = NULL, candidateImages = NULL, updatedAt = NOW(3) WHERE id = ? AND previousImageUrl IS NOT NULL",
                )
                .bind(panel_id)
                .execute(&state.mysql)
                .await?
                .rows_affected();
                if affected == 0 {
                    return Err(AppError::invalid_params("no previous version"));
                }
                "panel image reverted"
            } else {
                return Err(AppError::invalid_params("unsupported type"));
            };

            Ok(Json(json!({ "success": true, "message": message })))
        }
        ("POST", ["panel", "select-candidate"]) => {
            let panel_id = body_string(&body_json, "panelId")
                .ok_or_else(|| AppError::invalid_params("panelId is required"))?;
            let selected_index = body_i32(&body_json, "selectedIndex").unwrap_or(0);
            let panel: Option<(Option<String>,)> = sqlx::query_as(
                "SELECT candidateImages FROM novel_promotion_panels WHERE id = ? LIMIT 1",
            )
            .bind(&panel_id)
            .fetch_optional(&state.mysql)
            .await?;
            let Some((candidate_images_raw,)) = panel else {
                return Err(AppError::not_found("panel not found"));
            };
            let candidate_images =
                parse_json_str(candidate_images_raw.as_deref()).unwrap_or_else(|| json!([]));
            let selected_url = candidate_images
                .as_array()
                .and_then(|items| {
                    usize::try_from(selected_index)
                        .ok()
                        .and_then(|idx| items.get(idx))
                })
                .and_then(Value::as_str)
                .map(|item| item.to_string());
            sqlx::query(
                "UPDATE novel_promotion_panels SET imageUrl = ?, updatedAt = NOW(3) WHERE id = ?",
            )
            .bind(selected_url.clone())
            .bind(panel_id)
            .execute(&state.mysql)
            .await?;
            let cos_key = selected_url.clone();
            Ok(Json(json!({
              "success": true,
              "imageUrl": selected_url,
              "cosKey": cos_key,
              "message": "panel candidate selected",
            })))
        }
        ("PUT", ["photography-plan"]) => {
            let storyboard_id = body_string(&body_json, "storyboardId")
                .ok_or_else(|| AppError::invalid_params("storyboardId is required"))?;
            sqlx::query("UPDATE novel_promotion_storyboards SET photographyPlan = ?, updatedAt = NOW(3) WHERE id = ?")
                .bind(normalize_optional_json(body_json.get("photographyPlan").cloned()))
                .bind(storyboard_id)
                .execute(&state.mysql)
                .await?;
            Ok(Json(json!({ "success": true })))
        }
        _ => Err(AppError::internal(format!(
            "route /api/novel-promotion/{project_id}/{path} is not implemented in Rust backend yet"
        ))),
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/novel-promotion/{projectId}",
            get(get_root).patch(patch_root),
        )
        .route("/api/novel-promotion/{projectId}/assets", get(route_assets))
        .route(
            "/api/novel-promotion/{projectId}/episodes",
            get(route_episodes_get).post(route_episodes_post),
        )
        .route(
            "/api/novel-promotion/{projectId}/video-urls",
            post(route_video_urls),
        )
        .route(
            "/api/novel-promotion/{projectId}/download-images",
            get(route_download_images),
        )
        .route(
            "/api/novel-promotion/{projectId}/download-videos",
            post(route_download_videos),
        )
        .route(
            "/api/novel-promotion/{projectId}/download-voices",
            get(route_download_voices),
        )
        .route(
            "/api/novel-promotion/{projectId}/video-proxy",
            get(route_video_proxy),
        )
        .route("/api/novel-promotion/{projectId}/{*path}", any(dispatch))
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};
    use serde_json::json;

    use super::*;

    #[test]
    fn require_task_locale_accepts_header_locale() {
        let body = json!({});
        let mut headers = HeaderMap::new();
        headers.insert(
            "accept-language",
            HeaderValue::from_static("zh-CN,zh;q=0.9"),
        );

        assert!(require_task_locale(&body, &headers).is_ok());
    }

    #[test]
    fn validate_generate_image_submission_requires_type_and_id() {
        let body = json!({ "type": "character" });
        assert!(validate_generate_image_submission(&body).is_err());
    }

    #[test]
    fn validate_generate_character_image_submission_requires_character_id() {
        let body = json!({ "appearanceId": "appearance-1" });
        assert!(validate_generate_character_image_submission(&body).is_err());
    }

    #[test]
    fn validate_generate_video_submission_requires_valid_model_key() {
        let body = json!({
            "videoModel": "fal-video-kling-v2-1-master",
            "storyboardId": "storyboard-1",
            "panelIndex": 0,
        });
        assert!(validate_generate_video_submission(&body).is_err());
    }

    #[test]
    fn validate_generate_video_submission_requires_episode_for_batch() {
        let body = json!({
            "videoModel": "fal::video-kling-v2-1-master",
            "all": true,
        });
        assert!(validate_generate_video_submission(&body).is_err());
    }

    #[test]
    fn validate_task_submission_payload_rejects_missing_locale() {
        let body = json!({ "type": "character", "id": "character-1" });
        let headers = HeaderMap::new();
        assert!(validate_task_submission_payload("generate-image", &body, &headers).is_err());
    }

    #[test]
    fn task_submission_priority_matches_ts_routes() {
        assert_eq!(task_submission_priority("analyze"), Some(1));
        assert_eq!(task_submission_priority("voice-analyze"), Some(1));
        assert_eq!(task_submission_priority("story-to-script-stream"), Some(2));
        assert_eq!(
            task_submission_priority("script-to-storyboard-stream"),
            Some(2)
        );
        assert_eq!(task_submission_priority("screenplay-conversion"), Some(2));
    }

    #[test]
    fn task_submission_priority_defaults_to_none_for_other_routes() {
        assert_eq!(task_submission_priority("generate-image"), None);
    }

    #[test]
    fn parse_nullable_f64_field_accepts_number_string_and_null() {
        assert_eq!(
            parse_nullable_f64_field(&json!(1.25), "duration").unwrap(),
            Some(1.25)
        );
        assert_eq!(
            parse_nullable_f64_field(&json!("2.5"), "duration").unwrap(),
            Some(2.5)
        );
        assert_eq!(
            parse_nullable_f64_field(&json!(null), "duration").unwrap(),
            None
        );
        assert!(parse_nullable_f64_field(&json!({}), "duration").is_err());
    }

    #[test]
    fn parse_nullable_i32_field_accepts_integer_string_and_null() {
        assert_eq!(
            parse_nullable_i32_field(&json!(12), "lineIndex").unwrap(),
            Some(12)
        );
        assert_eq!(
            parse_nullable_i32_field(&json!("34"), "lineIndex").unwrap(),
            Some(34)
        );
        assert_eq!(
            parse_nullable_i32_field(&json!(null), "lineIndex").unwrap(),
            None
        );
        assert!(parse_nullable_i32_field(&json!("1.2"), "lineIndex").is_err());
    }

    #[test]
    fn query_string_trims_and_rejects_empty() {
        let mut query = HashMap::new();
        query.insert("episodeId".to_string(), "  ep-1  ".to_string());
        query.insert("blank".to_string(), "   ".to_string());
        assert_eq!(query_string(&query, "episodeId"), Some("ep-1".to_string()));
        assert_eq!(query_string(&query, "blank"), None);
        assert_eq!(query_string(&query, "missing"), None);
    }
}
