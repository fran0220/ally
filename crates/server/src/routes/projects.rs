use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;
use waoowaoo_core::media;

use crate::{app_state::AppState, error::AppError, extractors::auth::AuthUser};

#[derive(Debug, Deserialize)]
pub struct ProjectListQuery {
    #[serde(default = "default_page")]
    page: i64,
    #[serde(default = "default_page_size", rename = "pageSize")]
    page_size: i64,
    #[serde(default)]
    search: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    name: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectRequest {
    name: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct ProjectRow {
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
struct EpisodeDetailRow {
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

#[derive(Debug, sqlx::FromRow)]
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

#[derive(Debug, sqlx::FromRow)]
struct CharacterAppearanceRow {
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
    #[sqlx(rename = "previousDescription")]
    previous_description: Option<String>,
    #[sqlx(rename = "previousDescriptions")]
    previous_descriptions: Option<String>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
}

#[derive(Debug, sqlx::FromRow)]
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

#[derive(Debug, sqlx::FromRow)]
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

#[derive(Debug, sqlx::FromRow)]
struct NovelProjectDataRow {
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

#[derive(Debug, sqlx::FromRow)]
struct ProjectOwnerRow {
    #[sqlx(rename = "userId")]
    user_id: String,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    12
}

fn normalize_page(page: i64) -> i64 {
    page.max(1)
}

fn normalize_page_size(page_size: i64) -> i64 {
    page_size.clamp(1, 100)
}

fn parse_json_value(value: Option<&str>) -> Option<Value> {
    value
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .filter(|parsed| !parsed.is_null())
}

fn parse_json_array(value: Option<&str>) -> Value {
    parse_json_value(value).unwrap_or_else(|| json!([]))
}

fn normalize_media_url(value: Option<&str>) -> Option<String> {
    media::to_public_media_url(value)
}

async fn load_project_assets_payload(
    state: &AppState,
    novel_id: &str,
) -> Result<(Vec<Value>, Vec<Value>), AppError> {
    let characters = sqlx::query_as::<_, CharacterRow>(
        "SELECT id, novelPromotionProjectId, name, aliases, profileData, profileConfirmed, customVoiceUrl, customVoiceMediaId, voiceId, voiceType, introduction, sourceGlobalCharacterId, createdAt, updatedAt
         FROM novel_promotion_characters
         WHERE novelPromotionProjectId = ?
         ORDER BY createdAt ASC",
    )
    .bind(novel_id)
    .fetch_all(&state.mysql)
    .await?;
    let locations = sqlx::query_as::<_, LocationRow>(
        "SELECT id, novelPromotionProjectId, name, summary, sourceGlobalLocationId, selectedImageId, createdAt, updatedAt
         FROM novel_promotion_locations
         WHERE novelPromotionProjectId = ?
         ORDER BY createdAt ASC",
    )
    .bind(novel_id)
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
        let mut builder: QueryBuilder<'_, MySql> = QueryBuilder::new(
            "SELECT id, characterId, appearanceIndex, changeReason, description, descriptions, imageUrl, imageMediaId, imageUrls, selectedIndex, previousImageUrl, previousDescription, previousDescriptions, createdAt, updatedAt
             FROM character_appearances
             WHERE characterId IN (",
        );
        let mut separated = builder.separated(",");
        for id in &character_ids {
            separated.push_bind(id);
        }
        separated.push_unseparated(") ORDER BY appearanceIndex ASC, createdAt ASC");
        builder
            .build_query_as::<CharacterAppearanceRow>()
            .fetch_all(&state.mysql)
            .await?
    };

    let images = if location_ids.is_empty() {
        Vec::new()
    } else {
        let mut builder: QueryBuilder<'_, MySql> = QueryBuilder::new(
            "SELECT id, locationId, imageIndex, description, imageUrl, imageMediaId, isSelected, previousImageUrl, previousDescription, createdAt, updatedAt
             FROM location_images
             WHERE locationId IN (",
        );
        let mut separated = builder.separated(",");
        for id in &location_ids {
            separated.push_bind(id);
        }
        separated.push_unseparated(") ORDER BY imageIndex ASC, createdAt ASC");
        builder
            .build_query_as::<LocationImageRow>()
            .fetch_all(&state.mysql)
            .await?
    };

    let mut appearance_map: std::collections::HashMap<String, Vec<Value>> =
        std::collections::HashMap::new();
    for item in appearances {
        let image_url = normalize_media_url(item.image_url.as_deref()).or(item.image_url.clone());
        let previous_image_url =
            normalize_media_url(item.previous_image_url.as_deref()).or(item.previous_image_url);
        let value = json!({
          "id": item.id,
          "characterId": item.character_id,
          "appearanceIndex": item.appearance_index,
          "changeReason": item.change_reason,
          "description": item.description,
          "descriptions": parse_json_value(item.descriptions.as_deref()),
          "imageUrl": image_url,
          "imageMediaId": item.image_media_id,
          "imageUrls": parse_json_array(item.image_urls.as_deref()),
          "selectedIndex": item.selected_index,
          "previousImageUrl": previous_image_url,
          "previousDescription": item.previous_description,
          "previousDescriptions": parse_json_value(item.previous_descriptions.as_deref()),
          "createdAt": item.created_at,
          "updatedAt": item.updated_at,
        });
        appearance_map
            .entry(item.character_id)
            .or_default()
            .push(value);
    }

    let mut image_map: std::collections::HashMap<String, Vec<Value>> =
        std::collections::HashMap::new();
    for item in images {
        let image_url = normalize_media_url(item.image_url.as_deref()).or(item.image_url.clone());
        let previous_image_url =
            normalize_media_url(item.previous_image_url.as_deref()).or(item.previous_image_url);
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
        image_map.entry(item.location_id).or_default().push(value);
    }

    let character_values = characters
        .into_iter()
        .map(|item| {
            let custom_voice_url =
                normalize_media_url(item.custom_voice_url.as_deref()).or(item.custom_voice_url);
            json!({
              "id": item.id,
              "novelPromotionProjectId": item.novel_promotion_project_id,
              "name": item.name,
              "aliases": parse_json_value(item.aliases.as_deref()),
              "profileData": parse_json_value(item.profile_data.as_deref()),
              "profileConfirmed": item.profile_confirmed,
              "customVoiceUrl": custom_voice_url,
              "customVoiceMediaId": item.custom_voice_media_id,
              "voiceId": item.voice_id,
              "voiceType": item.voice_type,
              "introduction": item.introduction,
              "sourceGlobalCharacterId": item.source_global_character_id,
              "appearances": appearance_map.remove(&item.id).unwrap_or_default(),
              "createdAt": item.created_at,
              "updatedAt": item.updated_at,
            })
        })
        .collect::<Vec<_>>();

    let location_values = locations
        .into_iter()
        .map(|item| {
            json!({
              "id": item.id,
              "novelPromotionProjectId": item.novel_promotion_project_id,
              "name": item.name,
              "summary": item.summary,
              "sourceGlobalLocationId": item.source_global_location_id,
              "selectedImageId": item.selected_image_id,
              "images": image_map.remove(&item.id).unwrap_or_default(),
              "createdAt": item.created_at,
              "updatedAt": item.updated_at,
            })
        })
        .collect::<Vec<_>>();

    Ok((character_values, location_values))
}

async fn verify_project_owner(
    state: &AppState,
    project_id: &str,
    user_id: &str,
) -> Result<(), AppError> {
    let owner =
        sqlx::query_as::<_, ProjectOwnerRow>("SELECT userId FROM projects WHERE id = ? LIMIT 1")
            .bind(project_id)
            .fetch_optional(&state.mysql)
            .await?;

    let Some(owner) = owner else {
        return Err(AppError::not_found("project not found"));
    };

    if owner.user_id != user_id {
        return Err(AppError::forbidden("project access denied"));
    }

    Ok(())
}

pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
    Query(params): Query<ProjectListQuery>,
) -> Result<Json<Value>, AppError> {
    let page = normalize_page(params.page);
    let page_size = normalize_page_size(params.page_size);
    let offset = (page - 1) * page_size;

    let search = params.search.unwrap_or_default().trim().to_string();

    let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
        "SELECT id, name, description, mode, userId, createdAt, updatedAt, lastAccessedAt FROM projects WHERE userId = ",
    );
    qb.push_bind(&user.id);

    if !search.is_empty() {
        let like = format!("%{search}%");
        qb.push(" AND (name LIKE ");
        qb.push_bind(like.clone());
        qb.push(" OR description LIKE ");
        qb.push_bind(like.clone());
        qb.push(")");
    }

    qb.push(" ORDER BY COALESCE(lastAccessedAt, createdAt) DESC LIMIT ");
    qb.push_bind(page_size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);

    let projects = qb
        .build_query_as::<ProjectRow>()
        .fetch_all(&state.mysql)
        .await?;

    let mut count_qb: QueryBuilder<'_, MySql> =
        QueryBuilder::new("SELECT COUNT(*) FROM projects WHERE userId = ");
    count_qb.push_bind(&user.id);

    if !search.is_empty() {
        let like = format!("%{search}%");
        count_qb.push(" AND (name LIKE ");
        count_qb.push_bind(like.clone());
        count_qb.push(" OR description LIKE ");
        count_qb.push_bind(like.clone());
        count_qb.push(")");
    }

    let total = count_qb
        .build_query_scalar::<i64>()
        .fetch_one(&state.mysql)
        .await?;

    let total_pages = if total == 0 {
        0
    } else {
        ((total + page_size - 1) / page_size).max(1)
    };

    Ok(Json(json!({
      "projects": projects,
      "pagination": {
        "page": page,
        "pageSize": page_size,
        "total": total,
        "totalPages": total_pages
      }
    })))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<CreateProjectRequest>,
) -> Result<Json<Value>, AppError> {
    let name = payload.name.trim();
    if name.is_empty() {
        return Err(AppError::invalid_params("project name is required"));
    }
    if name.len() > 100 {
        return Err(AppError::invalid_params("project name too long"));
    }
    if payload
        .description
        .as_ref()
        .map(|value| value.len() > 500)
        .unwrap_or(false)
    {
        return Err(AppError::invalid_params("project description too long"));
    }

    let project_id = Uuid::new_v4().to_string();
    let novel_id = Uuid::new_v4().to_string();
    let normalized_description = payload.description.map(|value| value.trim().to_string());

    let mut tx = state.mysql.begin().await?;

    sqlx::query(
        "INSERT INTO projects (id, name, description, mode, userId, createdAt, updatedAt) VALUES (?, ?, ?, 'novel-promotion', ?, NOW(3), NOW(3))",
    )
    .bind(&project_id)
    .bind(name)
    .bind(normalized_description.clone())
    .bind(&user.id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO novel_promotion_projects (id, projectId, videoRatio, ttsRate, artStyle, workflowMode, videoResolution, imageResolution, createdAt, updatedAt) VALUES (?, ?, '9:16', '+50%', 'american-comic', 'srt', '720p', '2K', NOW(3), NOW(3))",
    )
    .bind(&novel_id)
    .bind(&project_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let project = sqlx::query_as::<_, ProjectRow>(
        "SELECT id, name, description, mode, userId, createdAt, updatedAt, lastAccessedAt FROM projects WHERE id = ? LIMIT 1",
    )
    .bind(&project_id)
    .fetch_one(&state.mysql)
    .await?;

    Ok(Json(json!({
      "project": project
    })))
}

pub async fn get(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(&state, &project_id, &user.id).await?;

    let project = sqlx::query_as::<_, ProjectRow>(
        "SELECT id, name, description, mode, userId, createdAt, updatedAt, lastAccessedAt FROM projects WHERE id = ? LIMIT 1",
    )
    .bind(&project_id)
    .fetch_one(&state.mysql)
    .await?;

    Ok(Json(json!({ "project": project })))
}

pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
    Json(payload): Json<UpdateProjectRequest>,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(&state, &project_id, &user.id).await?;

    let mut builder: QueryBuilder<'_, MySql> = QueryBuilder::new("UPDATE projects SET ");
    let mut separated = builder.separated(", ");
    let mut touched = false;

    if let Some(name) = payload.name {
        touched = true;
        separated.push("name = ");
        separated.push_bind(name.trim().to_string());
    }

    if let Some(description) = payload.description {
        touched = true;
        separated.push("description = ");
        separated.push_bind(description.trim().to_string());
    }

    if !touched {
        return Err(AppError::invalid_params("empty update payload"));
    }

    separated.push("updatedAt = NOW(3)");
    builder.push(" WHERE id = ");
    builder.push_bind(&project_id);

    builder.build().execute(&state.mysql).await?;

    get(State(state), user, Path(project_id)).await
}

pub async fn delete(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(&state, &project_id, &user.id).await?;

    sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(&project_id)
        .execute(&state.mysql)
        .await?;

    Ok(Json(json!({
      "success": true,
      "cosFilesDeleted": 0,
      "cosFilesFailed": 0,
    })))
}

pub async fn assets(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(&state, &project_id, &user.id).await?;

    let novel_project_id: Option<(String,)> =
        sqlx::query_as("SELECT id FROM novel_promotion_projects WHERE projectId = ? LIMIT 1")
            .bind(&project_id)
            .fetch_optional(&state.mysql)
            .await?;

    let Some((novel_id,)) = novel_project_id else {
        return Err(AppError::not_found("novel promotion data not found"));
    };

    let (characters, locations) = load_project_assets_payload(&state, &novel_id).await?;

    Ok(Json(json!({
      "characters": characters,
      "locations": locations,
    })))
}

pub async fn data(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(&state, &project_id, &user.id).await?;

    let project = sqlx::query_as::<_, ProjectRow>(
        "SELECT id, name, description, mode, userId, createdAt, updatedAt, lastAccessedAt FROM projects WHERE id = ? LIMIT 1",
    )
    .bind(&project_id)
    .fetch_one(&state.mysql)
    .await?;

    let novel_data = sqlx::query_as::<_, NovelProjectDataRow>(
        "SELECT id, projectId, analysisModel, imageModel, videoModel, videoRatio, ttsRate, globalAssetText, artStyle, artStylePrompt, characterModel, locationModel, storyboardModel, editModel, videoResolution, workflowMode, lastEpisodeId, imageResolution, importStatus, capabilityOverrides, createdAt, updatedAt
         FROM novel_promotion_projects
         WHERE projectId = ?
         LIMIT 1",
    )
    .bind(&project_id)
    .fetch_optional(&state.mysql)
    .await?;

    let novel_data = match novel_data {
        Some(value) => value,
        None => return Err(AppError::not_found("novel promotion data not found")),
    };

    let episodes = sqlx::query_as::<_, EpisodeDetailRow>(
        "SELECT id, novelPromotionProjectId, episodeNumber, name, description, novelText, audioUrl, audioMediaId, srtContent, speakerVoices, createdAt, updatedAt
         FROM novel_promotion_episodes
         WHERE novelPromotionProjectId = ?
         ORDER BY episodeNumber ASC",
    )
    .bind(&novel_data.id)
    .fetch_all(&state.mysql)
    .await?;
    let episodes = episodes
        .into_iter()
        .map(|episode| {
            let audio_url = normalize_media_url(episode.audio_url.as_deref()).or(episode.audio_url);
            json!({
              "id": episode.id,
              "novelPromotionProjectId": episode.novel_promotion_project_id,
              "episodeNumber": episode.episode_number,
              "name": episode.name,
              "description": episode.description,
              "novelText": episode.novel_text,
              "audioUrl": audio_url,
              "audioMediaId": episode.audio_media_id,
              "srtContent": episode.srt_content,
              "speakerVoices": parse_json_value(episode.speaker_voices.as_deref()),
              "createdAt": episode.created_at,
              "updatedAt": episode.updated_at,
            })
        })
        .collect::<Vec<_>>();
    let (characters, locations) = load_project_assets_payload(&state, &novel_data.id).await?;
    let capability_overrides =
        parse_json_value(novel_data.capability_overrides.as_deref()).unwrap_or_else(|| json!({}));

    Ok(Json(json!({
      "project": {
        "id": project.id,
        "name": project.name,
        "description": project.description,
        "mode": project.mode,
        "userId": project.user_id,
        "createdAt": project.created_at,
        "updatedAt": project.updated_at,
        "lastAccessedAt": project.last_accessed_at,
        "novelPromotionData": {
          "id": novel_data.id,
          "projectId": novel_data.project_id,
          "analysisModel": novel_data.analysis_model,
          "imageModel": novel_data.image_model,
          "videoModel": novel_data.video_model,
          "videoRatio": novel_data.video_ratio,
          "ttsRate": novel_data.tts_rate,
          "globalAssetText": novel_data.global_asset_text,
          "artStyle": novel_data.art_style,
          "artStylePrompt": novel_data.art_style_prompt,
          "characterModel": novel_data.character_model,
          "locationModel": novel_data.location_model,
          "storyboardModel": novel_data.storyboard_model,
          "editModel": novel_data.edit_model,
          "videoResolution": novel_data.video_resolution,
          "workflowMode": novel_data.workflow_mode,
          "lastEpisodeId": novel_data.last_episode_id,
          "imageResolution": novel_data.image_resolution,
          "importStatus": novel_data.import_status,
          "capabilityOverrides": capability_overrides,
          "episodes": episodes,
          "characters": characters,
          "locations": locations,
          "createdAt": novel_data.created_at,
          "updatedAt": novel_data.updated_at,
        }
      }
    })))
}

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/api/projects", axum::routing::get(list).post(create))
        .route(
            "/api/projects/{id}",
            axum::routing::get(get).patch(update).delete(delete),
        )
        .route("/api/projects/{id}/assets", axum::routing::get(assets))
        .route("/api/projects/{id}/data", axum::routing::get(data))
}
