use std::collections::HashMap;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, Method, header},
    routing::{any, get},
};
use chrono::NaiveDateTime;
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

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
struct PanelRow {
    id: String,
    #[sqlx(rename = "storyboardId")]
    storyboard_id: String,
    #[sqlx(rename = "panelIndex")]
    panel_index: i32,
    #[sqlx(rename = "panelNumber")]
    panel_number: Option<i32>,
    description: Option<String>,
    location: Option<String>,
    characters: Option<String>,
    #[sqlx(rename = "imagePrompt")]
    image_prompt: Option<String>,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
    #[sqlx(rename = "candidateImages")]
    candidate_images: Option<String>,
    #[sqlx(rename = "videoPrompt")]
    video_prompt: Option<String>,
    #[sqlx(rename = "videoUrl")]
    video_url: Option<String>,
    #[sqlx(rename = "linkedToNextPanel")]
    linked_to_next_panel: bool,
    #[sqlx(rename = "previousImageUrl")]
    previous_image_url: Option<String>,
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

fn parse_body_json(raw: &Bytes) -> Result<Value, AppError> {
    if raw.is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_slice::<Value>(raw)
        .map_err(|err| AppError::invalid_params(format!("invalid json body: {err}")))
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
            separated.push(db_col);
            separated.push_unseparated(" = ");
            separated.push_bind(normalize_optional_string(
                body.get(body_key)
                    .and_then(Value::as_str)
                    .map(|item| item.to_string()),
            ));
        }
    }

    if let Some(value) = body.get("capabilityOverrides") {
        touched = true;
        separated.push("capabilityOverrides");
        separated.push_unseparated(" = ");
        separated.push_bind(normalize_optional_json(Some(value.clone())));
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
            separated.push(db_col);
            separated.push_unseparated(" = ");
            separated.push_bind(body_string(&body, body_key));
        }
    }

    if body.get("speakerVoices").is_some() {
        touched = true;
        separated.push("speakerVoices");
        separated.push_unseparated(" = ");
        separated.push_bind(normalize_optional_json(body.get("speakerVoices").cloned()));
    }

    if body.get("episodeNumber").is_some() {
        touched = true;
        separated.push("episodeNumber");
        separated.push_unseparated(" = ");
        separated.push_bind(body_i32(&body, "episodeNumber"));
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
    body: Value,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;
    let novel_id = get_novel_id(state, project_id).await?;

    match method {
        "POST" => {
            let name = body_string(&body, "name")
                .ok_or_else(|| AppError::invalid_params("name is required"))?;
            let character_id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO novel_promotion_characters (id, novelPromotionProjectId, name, aliases, profileData, introduction, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
            )
            .bind(&character_id)
            .bind(&novel_id)
            .bind(name)
            .bind(normalize_optional_json(body.get("aliases").cloned()))
            .bind(normalize_optional_json(body.get("profileData").cloned()))
            .bind(body_string(&body, "introduction"))
            .execute(&state.mysql)
            .await?;

            sqlx::query(
                "INSERT INTO character_appearances (id, characterId, appearanceIndex, createdAt, updatedAt) VALUES (?, ?, 0, NOW(3), NOW(3))",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&character_id)
            .execute(&state.mysql)
            .await?;

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
                    separated.push(db_col);
                    separated.push_unseparated(" = ");
                    separated.push_bind(body_string(&body, key));
                }
            }

            if body.get("aliases").is_some() {
                touched = true;
                separated.push("aliases");
                separated.push_unseparated(" = ");
                separated.push_bind(normalize_optional_json(body.get("aliases").cloned()));
            }
            if body.get("profileData").is_some() {
                touched = true;
                separated.push("profileData");
                separated.push_unseparated(" = ");
                separated.push_bind(normalize_optional_json(body.get("profileData").cloned()));
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
    body: Value,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;
    let novel_id = get_novel_id(state, project_id).await?;

    match method {
        "POST" => {
            let name = body_string(&body, "name")
                .ok_or_else(|| AppError::invalid_params("name is required"))?;
            let location_id = Uuid::new_v4().to_string();

            sqlx::query(
                "INSERT INTO novel_promotion_locations (id, novelPromotionProjectId, name, summary, createdAt, updatedAt) VALUES (?, ?, ?, ?, NOW(3), NOW(3))",
            )
            .bind(&location_id)
            .bind(&novel_id)
            .bind(name)
            .bind(body_string(&body, "summary"))
            .execute(&state.mysql)
            .await?;

            sqlx::query(
                "INSERT INTO location_images (id, locationId, imageIndex, description, imageUrl, isSelected, createdAt, updatedAt) VALUES (?, ?, 0, ?, ?, true, NOW(3), NOW(3))",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&location_id)
            .bind(body_string(&body, "description"))
            .bind(body_string(&body, "imageUrl"))
            .execute(&state.mysql)
            .await?;

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
                sqlx::query(
                    "UPDATE location_images SET description = ?, updatedAt = NOW(3) WHERE locationId = ? AND imageIndex = ?",
                )
                .bind(body_string(&body, "description"))
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
                separated.push("name");
                separated.push_unseparated(" = ");
                separated.push_bind(body_string(&body, "name"));
            }
            if body.get("summary").is_some() {
                touched = true;
                separated.push("summary");
                separated.push_unseparated(" = ");
                separated.push_bind(body_string(&body, "summary"));
            }
            if body.get("selectedImageId").is_some() {
                touched = true;
                separated.push("selectedImageId");
                separated.push_unseparated(" = ");
                separated.push_bind(body_string(&body, "selectedImageId"));
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
            let episode_id = query
                .get("episodeId")
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
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
            let mut speaker_counts: HashMap<String, usize> = HashMap::new();
            for row in &rows {
                let entry = speaker_counts.entry(row.speaker.clone()).or_insert(0);
                *entry += 1;
            }
            let speakers = speaker_counts.keys().cloned().collect::<Vec<_>>();

            Ok(Json(json!({
              "voiceLines": rows,
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

            let max: Option<(i32,)> = sqlx::query_as(
                "SELECT lineIndex FROM novel_promotion_voice_lines WHERE episodeId = ? ORDER BY lineIndex DESC LIMIT 1",
            )
            .bind(&episode_id)
            .fetch_optional(&state.mysql)
            .await?;

            let line_index = max.map(|item| item.0 + 1).unwrap_or(1);
            let line_id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO novel_promotion_voice_lines (id, episodeId, lineIndex, speaker, content, voicePresetId, audioUrl, audioMediaId, emotionPrompt, emotionStrength, matchedPanelIndex, matchedStoryboardId, audioDuration, matchedPanelId, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
            )
            .bind(&line_id)
            .bind(&episode_id)
            .bind(line_index)
            .bind(speaker)
            .bind(content)
            .bind(body_string(&body, "voicePresetId"))
            .bind(body_string(&body, "audioUrl"))
            .bind(body_string(&body, "audioMediaId"))
            .bind(body_string(&body, "emotionPrompt"))
            .bind(body_f64(&body, "emotionStrength"))
            .bind(body_i32(&body, "matchedPanelIndex"))
            .bind(body_string(&body, "matchedStoryboardId"))
            .bind(body_i32(&body, "audioDuration"))
            .bind(body_string(&body, "matchedPanelId"))
            .execute(&state.mysql)
            .await?;
            let voice_line = sqlx::query_as::<_, VoiceLineRow>(
                "SELECT id, episodeId, lineIndex, speaker, content, voicePresetId, audioUrl, audioMediaId, emotionPrompt, emotionStrength, matchedPanelIndex, matchedStoryboardId, audioDuration, matchedPanelId, createdAt, updatedAt FROM novel_promotion_voice_lines WHERE id = ? LIMIT 1",
            )
            .bind(&line_id)
            .fetch_one(&state.mysql)
            .await?;

            Ok(Json(json!({
              "success": true,
              "voiceLine": voice_line,
            })))
        }
        "PATCH" => {
            let line_id = body_string(&body, "lineId")
                .ok_or_else(|| AppError::invalid_params("lineId is required"))?;

            let mut qb: QueryBuilder<'_, MySql> =
                QueryBuilder::new("UPDATE novel_promotion_voice_lines SET ");
            let mut separated = qb.separated(", ");
            let mut touched = false;

            for key in [
                "speaker",
                "content",
                "voicePresetId",
                "audioUrl",
                "audioMediaId",
                "emotionPrompt",
                "matchedStoryboardId",
                "matchedPanelId",
            ] {
                if body.get(key).is_some() {
                    touched = true;
                    separated.push(key);
                    separated.push_unseparated(" = ");
                    separated.push_bind(body_string(&body, key));
                }
            }

            for key in ["lineIndex", "matchedPanelIndex", "audioDuration"] {
                if body.get(key).is_some() {
                    touched = true;
                    separated.push(key);
                    separated.push_unseparated(" = ");
                    separated.push_bind(body_i32(&body, key));
                }
            }

            if body.get("emotionStrength").is_some() {
                touched = true;
                separated.push("emotionStrength");
                separated.push_unseparated(" = ");
                separated.push_bind(body_f64(&body, "emotionStrength"));
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
            let speaker = voice_line.speaker.clone();
            let voice_preset_id = voice_line.voice_preset_id.clone();
            Ok(Json(json!({
              "success": true,
              "updatedCount": updated,
              "voiceLine": voice_line,
              "speaker": speaker,
              "voicePresetId": voice_preset_id,
            })))
        }
        "DELETE" => {
            let line_id = body_string(&body, "lineId")
                .ok_or_else(|| AppError::invalid_params("lineId is required"))?;
            let episode: Option<(String,)> = sqlx::query_as(
                "SELECT episodeId FROM novel_promotion_voice_lines WHERE id = ? LIMIT 1",
            )
            .bind(&line_id)
            .fetch_optional(&state.mysql)
            .await?;
            let Some((episode_id,)) = episode else {
                return Err(AppError::not_found("voice line not found"));
            };
            sqlx::query("DELETE FROM novel_promotion_voice_lines WHERE id = ?")
                .bind(&line_id)
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

    let episode_id = query
        .get("episodeId")
        .cloned()
        .or_else(|| body_string(&body, "episodeId"))
        .ok_or_else(|| AppError::invalid_params("episodeId is required"))?;

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

    match method {
        "GET" => {
            let episode_id = query
                .get("episodeId")
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .ok_or_else(|| AppError::invalid_params("episodeId is required"))?;

            let storyboards = sqlx::query_as::<_, StoryboardRow>(
                "SELECT id, episodeId, clipId, storyboardImageUrl, panelCount, storyboardTextJson, imageHistory, candidateImages, lastError, photographyPlan, createdAt, updatedAt FROM novel_promotion_storyboards WHERE episodeId = ? ORDER BY createdAt ASC",
            )
            .bind(&episode_id)
            .fetch_all(&state.mysql)
            .await?;

            Ok(Json(json!({ "storyboards": storyboards })))
        }
        "PATCH" => {
            let storyboard_id = body_string(&body, "storyboardId")
                .ok_or_else(|| AppError::invalid_params("storyboardId is required"))?;
            sqlx::query("UPDATE novel_promotion_storyboards SET lastError = NULL, updatedAt = NOW(3) WHERE id = ?")
                .bind(&storyboard_id)
                .execute(&state.mysql)
                .await?;
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
    body: Value,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;

    match method {
        "POST" => {
            let storyboard_id = body_string(&body, "storyboardId")
                .ok_or_else(|| AppError::invalid_params("storyboardId is required"))?;
            let panel_index = body_i32(&body, "panelIndex")
                .ok_or_else(|| AppError::invalid_params("panelIndex is required"))?;

            let panel_id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO novel_promotion_panels (id, storyboardId, panelIndex, panelNumber, description, location, characters, imagePrompt, imageUrl, candidateImages, videoPrompt, videoUrl, linkedToNextPanel, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
            )
            .bind(&panel_id)
            .bind(&storyboard_id)
            .bind(panel_index)
            .bind(body_i32(&body, "panelNumber"))
            .bind(body_string(&body, "description"))
            .bind(body_string(&body, "location"))
            .bind(normalize_optional_json(body.get("characters").cloned()))
            .bind(body_string(&body, "imagePrompt"))
            .bind(body_string(&body, "imageUrl"))
            .bind(normalize_optional_json(body.get("candidateImages").cloned()))
            .bind(body_string(&body, "videoPrompt"))
            .bind(body_string(&body, "videoUrl"))
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

            Ok(Json(json!({
              "success": true,
              "panel": { "id": panel_id },
            })))
        }
        "PATCH" | "PUT" => {
            let panel_id = body_string(&body, "panelId")
                .ok_or_else(|| AppError::invalid_params("panelId is required"))?;
            let mut qb: QueryBuilder<'_, MySql> =
                QueryBuilder::new("UPDATE novel_promotion_panels SET ");
            let mut separated = qb.separated(", ");
            let mut touched = false;

            for key in [
                "description",
                "location",
                "imagePrompt",
                "imageUrl",
                "videoPrompt",
                "videoUrl",
                "previousImageUrl",
            ] {
                if body.get(key).is_some() {
                    touched = true;
                    separated.push(key);
                    separated.push_unseparated(" = ");
                    separated.push_bind(body_string(&body, key));
                }
            }

            for key in ["panelIndex", "panelNumber"] {
                if body.get(key).is_some() {
                    touched = true;
                    separated.push(key);
                    separated.push_unseparated(" = ");
                    separated.push_bind(body_i32(&body, key));
                }
            }

            if body.get("linkedToNextPanel").is_some() {
                touched = true;
                separated.push("linkedToNextPanel");
                separated.push_unseparated(" = ");
                separated.push_bind(body_bool(&body, "linkedToNextPanel"));
            }

            if body.get("characters").is_some() {
                touched = true;
                separated.push("characters");
                separated.push_unseparated(" = ");
                separated.push_bind(normalize_optional_json(body.get("characters").cloned()));
            }
            if body.get("candidateImages").is_some() {
                touched = true;
                separated.push("candidateImages");
                separated.push_unseparated(" = ");
                separated.push_bind(normalize_optional_json(
                    body.get("candidateImages").cloned(),
                ));
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
                .ok_or_else(|| AppError::invalid_params("panelId is required"))?;
            sqlx::query("DELETE FROM novel_promotion_panels WHERE id = ?")
                .bind(&panel_id)
                .execute(&state.mysql)
                .await?;
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
    body: Value,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;

    match method {
        "POST" => {
            let episode_id = body_string(&body, "episodeId")
                .ok_or_else(|| AppError::invalid_params("episodeId is required"))?;
            let clip_id = Uuid::new_v4().to_string();
            let storyboard_id = Uuid::new_v4().to_string();

            let mut tx = state.mysql.begin().await?;
            sqlx::query(
                "INSERT INTO novel_promotion_clips (id, episodeId, summary, content, createdAt, updatedAt) VALUES (?, ?, ?, ?, NOW(3), NOW(3))",
            )
            .bind(&clip_id)
            .bind(&episode_id)
            .bind(body_string(&body, "summary").unwrap_or_else(|| "clip".to_string()))
            .bind(body_string(&body, "content").unwrap_or_default())
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO novel_promotion_storyboards (id, episodeId, clipId, panelCount, createdAt, updatedAt) VALUES (?, ?, ?, 0, NOW(3), NOW(3))",
            )
            .bind(&storyboard_id)
            .bind(&episode_id)
            .bind(&clip_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;

            Ok(Json(json!({
              "success": true,
              "clip": { "id": clip_id },
              "panel": Value::Null,
              "storyboard": { "id": storyboard_id },
            })))
        }
        "PUT" => {
            let current_clip_id = body_string(&body, "currentClipId")
                .ok_or_else(|| AppError::invalid_params("currentClipId is required"))?;
            let target_clip_id = body_string(&body, "targetClipId")
                .ok_or_else(|| AppError::invalid_params("targetClipId is required"))?;

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
                .ok_or_else(|| AppError::invalid_params("storyboardId is required"))?;

            let clip: Option<(String,)> = sqlx::query_as(
                "SELECT clipId FROM novel_promotion_storyboards WHERE id = ? LIMIT 1",
            )
            .bind(&storyboard_id)
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
                    separated.push(key);
                    separated.push_unseparated(" = ");
                    separated.push_bind(body_string(&body, key));
                }
            }
            for key in ["start", "end", "duration", "shotCount"] {
                if body.get(key).is_some() {
                    touched = true;
                    separated.push(key);
                    separated.push_unseparated(" = ");
                    separated.push_bind(body_i32(&body, key));
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

    let episode_id = query
        .get("episodeId")
        .cloned()
        .or_else(|| body_string(&body, "episodeId"))
        .ok_or_else(|| AppError::invalid_params("episodeId is required"))?;

    match method {
        "GET" => {
            let row: Option<(Option<String>,)> = sqlx::query_as(
                "SELECT speakerVoices FROM novel_promotion_episodes WHERE id = ? LIMIT 1",
            )
            .bind(&episode_id)
            .fetch_optional(&state.mysql)
            .await?;
            let speaker_voices = row
                .and_then(|item| item.0)
                .and_then(|item| serde_json::from_str::<Value>(&item).ok())
                .unwrap_or_else(|| json!({}));
            Ok(Json(json!({ "speakerVoices": speaker_voices })))
        }
        "PATCH" => {
            let voices = body
                .get("speakerVoices")
                .cloned()
                .unwrap_or_else(|| json!({}));
            sqlx::query("UPDATE novel_promotion_episodes SET speakerVoices = ?, updatedAt = NOW(3) WHERE id = ?")
                .bind(serde_json::to_string(&voices).map_err(|err| AppError::invalid_params(format!("invalid speakerVoices: {err}")))?)
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
    body: Value,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;
    let episode_id = body_string(&body, "episodeId");

    let rows = if let Some(episode_id) = episode_id {
        sqlx::query_as::<_, PanelRow>(
            "SELECT id, storyboardId, panelIndex, panelNumber, description, location, characters, imagePrompt, imageUrl, candidateImages, videoPrompt, videoUrl, linkedToNextPanel, previousImageUrl, createdAt, updatedAt FROM novel_promotion_panels WHERE storyboardId IN (SELECT id FROM novel_promotion_storyboards WHERE episodeId = ?) ORDER BY panelIndex ASC",
        )
        .bind(&episode_id)
        .fetch_all(&state.mysql)
        .await?
    } else {
        let novel_id = get_novel_id(state, project_id).await?;
        sqlx::query_as::<_, PanelRow>(
            "SELECT id, storyboardId, panelIndex, panelNumber, description, location, characters, imagePrompt, imageUrl, candidateImages, videoPrompt, videoUrl, linkedToNextPanel, previousImageUrl, createdAt, updatedAt FROM novel_promotion_panels WHERE storyboardId IN (SELECT id FROM novel_promotion_storyboards WHERE episodeId IN (SELECT id FROM novel_promotion_episodes WHERE novelPromotionProjectId = ?)) ORDER BY panelIndex ASC",
        )
        .bind(&novel_id)
        .fetch_all(&state.mysql)
        .await?
    };

    let videos = rows
        .into_iter()
        .filter_map(|item| {
            item.video_url.map(|url| {
                json!({
                  "panelId": item.id,
                  "storyboardId": item.storyboard_id,
                  "panelIndex": item.panel_index,
                  "videoUrl": url,
                })
            })
        })
        .collect::<Vec<_>>();

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

async fn handle_download_images(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
    query: &HashMap<String, String>,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;

    let episode_id = query
        .get("episodeId")
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty());

    let rows = if let Some(episode_id) = episode_id {
        sqlx::query_as::<_, PanelRow>(
            "SELECT id, storyboardId, panelIndex, panelNumber, description, location, characters, imagePrompt, imageUrl, candidateImages, videoPrompt, videoUrl, linkedToNextPanel, previousImageUrl, createdAt, updatedAt FROM novel_promotion_panels WHERE storyboardId IN (SELECT id FROM novel_promotion_storyboards WHERE episodeId = ?) ORDER BY panelIndex ASC",
        )
        .bind(&episode_id)
        .fetch_all(&state.mysql)
        .await?
    } else {
        let novel_id = get_novel_id(state, project_id).await?;
        sqlx::query_as::<_, PanelRow>(
            "SELECT id, storyboardId, panelIndex, panelNumber, description, location, characters, imagePrompt, imageUrl, candidateImages, videoPrompt, videoUrl, linkedToNextPanel, previousImageUrl, createdAt, updatedAt FROM novel_promotion_panels WHERE storyboardId IN (SELECT id FROM novel_promotion_storyboards WHERE episodeId IN (SELECT id FROM novel_promotion_episodes WHERE novelPromotionProjectId = ?)) ORDER BY panelIndex ASC",
        )
        .bind(&novel_id)
        .fetch_all(&state.mysql)
        .await?
    };

    let images = rows
        .into_iter()
        .filter_map(|row| {
            row.image_url
                .map(|url| json!({ "panelId": row.id, "url": url }))
        })
        .collect::<Vec<_>>();

    Ok(Json(json!({ "images": images })))
}

async fn handle_download_videos(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
    body: Value,
) -> Result<Json<Value>, AppError> {
    handle_video_urls(state, user, project_id, body).await
}

async fn handle_download_voices(
    state: &AppState,
    user: &AuthUser,
    project_id: &str,
    query: &HashMap<String, String>,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(state, project_id, &user.id).await?;
    let episode_id = query
        .get("episodeId")
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty());

    let rows = if let Some(episode_id) = episode_id {
        sqlx::query_as::<_, VoiceLineRow>(
            "SELECT id, episodeId, lineIndex, speaker, content, voicePresetId, audioUrl, audioMediaId, emotionPrompt, emotionStrength, matchedPanelIndex, matchedStoryboardId, audioDuration, matchedPanelId, createdAt, updatedAt FROM novel_promotion_voice_lines WHERE episodeId = ? ORDER BY lineIndex ASC",
        )
        .bind(&episode_id)
        .fetch_all(&state.mysql)
        .await?
    } else {
        let novel_id = get_novel_id(state, project_id).await?;
        sqlx::query_as::<_, VoiceLineRow>(
            "SELECT id, episodeId, lineIndex, speaker, content, voicePresetId, audioUrl, audioMediaId, emotionPrompt, emotionStrength, matchedPanelIndex, matchedStoryboardId, audioDuration, matchedPanelId, createdAt, updatedAt FROM novel_promotion_voice_lines WHERE episodeId IN (SELECT id FROM novel_promotion_episodes WHERE novelPromotionProjectId = ?) ORDER BY lineIndex ASC",
        )
        .bind(&novel_id)
        .fetch_all(&state.mysql)
        .await?
    };

    let voices = rows
        .into_iter()
        .filter_map(|item| {
            item.audio_url
                .map(|url| json!({ "lineId": item.id, "url": url }))
        })
        .collect::<Vec<_>>();

    Ok(Json(json!({ "voices": voices })))
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
            handle_character_route(&state, &user, &project_id, method, body_json).await
        }
        (method, ["location"]) if method == "POST" || method == "PATCH" || method == "DELETE" => {
            handle_location_route(&state, &user, &project_id, method, body_json).await
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
            handle_panel(&state, &user, &project_id, method, body_json).await
        }
        (method, ["storyboard-group"])
            if method == "POST" || method == "PUT" || method == "DELETE" =>
        {
            handle_storyboard_group(&state, &user, &project_id, method, body_json).await
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
        ("POST", ["video-urls"]) => handle_video_urls(&state, &user, &project_id, body_json).await,
        ("GET", ["download-images"]) => {
            handle_download_images(&state, &user, &project_id, &query).await
        }
        ("POST", ["download-videos"]) => {
            handle_download_videos(&state, &user, &project_id, body_json).await
        }
        ("GET", ["download-voices"]) => {
            handle_download_voices(&state, &user, &project_id, &query).await
        }
        ("POST", ["copy-from-global"]) => {
            handle_copy_from_global(&state, &user, &project_id, body_json).await
        }
        ("POST", ["update-location"]) => {
            let location_id = body_string(&body_json, "locationId")
                .ok_or_else(|| AppError::invalid_params("locationId is required"))?;
            let image_index = body_i32(&body_json, "imageIndex")
                .ok_or_else(|| AppError::invalid_params("imageIndex is required"))?;
            sqlx::query("UPDATE location_images SET description = ?, updatedAt = NOW(3) WHERE locationId = ? AND imageIndex = ?")
                .bind(body_string(&body_json, "description"))
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
                .ok_or_else(|| AppError::invalid_params("characterId is required"))?;
            match method {
                "POST" => {
                    let appearance_index = body_i32(&body_json, "appearanceIndex")
                        .ok_or_else(|| AppError::invalid_params("appearanceIndex is required"))?;
                    let appearance_id = Uuid::new_v4().to_string();
                    sqlx::query(
                        "INSERT INTO character_appearances (id, characterId, appearanceIndex, description, descriptions, imageUrl, imageUrls, selectedIndex, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, ?, ?, NOW(3), NOW(3))",
                    )
                    .bind(&appearance_id)
                    .bind(&character_id)
                    .bind(appearance_index)
                    .bind(body_string(&body_json, "description"))
                    .bind(normalize_optional_json(body_json.get("descriptions").cloned()))
                    .bind(body_string(&body_json, "imageUrl"))
                    .bind(normalize_optional_json(body_json.get("imageUrls").cloned()))
                    .bind(body_i32(&body_json, "selectedIndex"))
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
                        .ok_or_else(|| AppError::invalid_params("appearanceId is required"))?;
                    sqlx::query(
                        "UPDATE character_appearances SET description = ?, descriptions = ?, imageUrl = ?, imageUrls = ?, selectedIndex = ?, updatedAt = NOW(3) WHERE id = ? AND characterId = ?",
                    )
                    .bind(body_string(&body_json, "description"))
                    .bind(normalize_optional_json(body_json.get("descriptions").cloned()))
                    .bind(body_string(&body_json, "imageUrl"))
                    .bind(normalize_optional_json(body_json.get("imageUrls").cloned()))
                    .bind(body_i32(&body_json, "selectedIndex"))
                    .bind(&appearance_id)
                    .bind(&character_id)
                    .execute(&state.mysql)
                    .await?;

                    Ok(Json(json!({ "success": true })))
                }
                "DELETE" => {
                    let appearance_id = body_string(&body_json, "appearanceId")
                        .ok_or_else(|| AppError::invalid_params("appearanceId is required"))?;
                    let deleted_images = sqlx::query(
                        "DELETE FROM character_appearances WHERE id = ? AND characterId = ?",
                    )
                    .bind(&appearance_id)
                    .bind(&character_id)
                    .execute(&state.mysql)
                    .await?
                    .rows_affected();

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
        ("GET", ["video-proxy"]) => {
            let url = query
                .get("key")
                .or_else(|| query.get("url"))
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .ok_or_else(|| AppError::invalid_params("key is required"))?;
            Ok(Json(json!({ "url": url })))
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
}
