use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::types::Json;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Account {
    pub id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "type")]
    pub account_type: String,
    pub provider: String,
    #[serde(rename = "providerAccountId")]
    pub provider_account_id: String,
    pub refresh_token: Option<String>,
    pub access_token: Option<String>,
    pub expires_at: Option<i32>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub id_token: Option<String>,
    pub session_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CharacterAppearance {
    pub id: String,
    #[serde(rename = "characterId")]
    pub character_id: String,
    #[serde(rename = "appearanceIndex")]
    pub appearance_index: i32,
    #[serde(rename = "changeReason")]
    pub change_reason: String,
    pub description: Option<String>,
    pub descriptions: Option<String>,
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[serde(rename = "imageUrls")]
    pub image_urls: Option<String>,
    #[serde(rename = "selectedIndex")]
    pub selected_index: Option<i32>,
    #[serde(rename = "previousImageUrl")]
    pub previous_image_url: Option<String>,
    #[serde(rename = "previousImageUrls")]
    pub previous_image_urls: Option<String>,
    #[serde(rename = "previousDescription")]
    pub previous_description: Option<String>,
    #[serde(rename = "previousDescriptions")]
    pub previous_descriptions: Option<String>,
    #[serde(rename = "imageMediaId")]
    pub image_media_id: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LocationImage {
    pub id: String,
    #[serde(rename = "locationId")]
    pub location_id: String,
    #[serde(rename = "imageIndex")]
    pub image_index: i32,
    pub description: Option<String>,
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[serde(rename = "isSelected")]
    pub is_selected: bool,
    #[serde(rename = "previousImageUrl")]
    pub previous_image_url: Option<String>,
    #[serde(rename = "previousDescription")]
    pub previous_description: Option<String>,
    #[serde(rename = "imageMediaId")]
    pub image_media_id: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NovelPromotionCharacter {
    pub id: String,
    #[serde(rename = "novelPromotionProjectId")]
    pub novel_promotion_project_id: String,
    pub name: String,
    pub aliases: Option<String>,
    #[serde(rename = "customVoiceUrl")]
    pub custom_voice_url: Option<String>,
    #[serde(rename = "customVoiceMediaId")]
    pub custom_voice_media_id: Option<String>,
    #[serde(rename = "voiceId")]
    pub voice_id: Option<String>,
    #[serde(rename = "voiceType")]
    pub voice_type: Option<String>,
    #[serde(rename = "profileData")]
    pub profile_data: Option<String>,
    #[serde(rename = "profileConfirmed")]
    pub profile_confirmed: bool,
    pub introduction: Option<String>,
    #[serde(rename = "sourceGlobalCharacterId")]
    pub source_global_character_id: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NovelPromotionLocation {
    pub id: String,
    #[serde(rename = "novelPromotionProjectId")]
    pub novel_promotion_project_id: String,
    pub name: String,
    pub summary: Option<String>,
    #[serde(rename = "sourceGlobalLocationId")]
    pub source_global_location_id: Option<String>,
    #[serde(rename = "selectedImageId")]
    pub selected_image_id: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NovelPromotionEpisode {
    pub id: String,
    #[serde(rename = "novelPromotionProjectId")]
    pub novel_promotion_project_id: String,
    #[serde(rename = "episodeNumber")]
    pub episode_number: i32,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "novelText")]
    pub novel_text: Option<String>,
    #[serde(rename = "audioUrl")]
    pub audio_url: Option<String>,
    #[serde(rename = "audioMediaId")]
    pub audio_media_id: Option<String>,
    #[serde(rename = "srtContent")]
    pub srt_content: Option<String>,
    #[serde(rename = "speakerVoices")]
    pub speaker_voices: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct VideoEditorProject {
    pub id: String,
    #[serde(rename = "episodeId")]
    pub episode_id: String,
    #[serde(rename = "projectData")]
    pub project_data: String,
    #[serde(rename = "renderStatus")]
    pub render_status: Option<String>,
    #[serde(rename = "renderTaskId")]
    pub render_task_id: Option<String>,
    #[serde(rename = "outputUrl")]
    pub output_url: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NovelPromotionClip {
    pub id: String,
    #[serde(rename = "episodeId")]
    pub episode_id: String,
    pub start: Option<i32>,
    pub end: Option<i32>,
    pub duration: Option<i32>,
    pub summary: String,
    pub location: Option<String>,
    pub content: String,
    pub characters: Option<String>,
    #[serde(rename = "endText")]
    pub end_text: Option<String>,
    #[serde(rename = "shotCount")]
    pub shot_count: Option<i32>,
    #[serde(rename = "startText")]
    pub start_text: Option<String>,
    pub screenplay: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NovelPromotionPanel {
    pub id: String,
    #[serde(rename = "storyboardId")]
    pub storyboard_id: String,
    #[serde(rename = "panelIndex")]
    pub panel_index: i32,
    #[serde(rename = "panelNumber")]
    pub panel_number: Option<i32>,
    #[serde(rename = "shotType")]
    pub shot_type: Option<String>,
    #[serde(rename = "cameraMove")]
    pub camera_move: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub characters: Option<String>,
    #[serde(rename = "srtSegment")]
    pub srt_segment: Option<String>,
    #[serde(rename = "srtStart")]
    pub srt_start: Option<f64>,
    #[serde(rename = "srtEnd")]
    pub srt_end: Option<f64>,
    pub duration: Option<f64>,
    #[serde(rename = "imagePrompt")]
    pub image_prompt: Option<String>,
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[serde(rename = "imageMediaId")]
    pub image_media_id: Option<String>,
    #[serde(rename = "imageHistory")]
    pub image_history: Option<String>,
    #[serde(rename = "videoPrompt")]
    pub video_prompt: Option<String>,
    #[serde(rename = "firstLastFramePrompt")]
    pub first_last_frame_prompt: Option<String>,
    #[serde(rename = "videoUrl")]
    pub video_url: Option<String>,
    #[serde(rename = "videoGenerationMode")]
    pub video_generation_mode: Option<String>,
    #[serde(rename = "videoMediaId")]
    pub video_media_id: Option<String>,
    #[serde(rename = "sceneType")]
    pub scene_type: Option<String>,
    #[serde(rename = "candidateImages")]
    pub candidate_images: Option<String>,
    #[serde(rename = "linkedToNextPanel")]
    pub linked_to_next_panel: bool,
    #[serde(rename = "lipSyncTaskId")]
    pub lip_sync_task_id: Option<String>,
    #[serde(rename = "lipSyncVideoUrl")]
    pub lip_sync_video_url: Option<String>,
    #[serde(rename = "lipSyncVideoMediaId")]
    pub lip_sync_video_media_id: Option<String>,
    #[serde(rename = "sketchImageUrl")]
    pub sketch_image_url: Option<String>,
    #[serde(rename = "sketchImageMediaId")]
    pub sketch_image_media_id: Option<String>,
    #[serde(rename = "photographyRules")]
    pub photography_rules: Option<String>,
    #[serde(rename = "actingNotes")]
    pub acting_notes: Option<String>,
    #[serde(rename = "previousImageUrl")]
    pub previous_image_url: Option<String>,
    #[serde(rename = "previousImageMediaId")]
    pub previous_image_media_id: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NovelPromotionProject {
    pub id: String,
    #[serde(rename = "projectId")]
    pub project_id: String,
    #[serde(rename = "analysisModel")]
    pub analysis_model: Option<String>,
    #[serde(rename = "imageModel")]
    pub image_model: Option<String>,
    #[serde(rename = "videoModel")]
    pub video_model: Option<String>,
    #[serde(rename = "videoRatio")]
    pub video_ratio: String,
    #[serde(rename = "ttsRate")]
    pub tts_rate: String,
    #[serde(rename = "globalAssetText")]
    pub global_asset_text: Option<String>,
    #[serde(rename = "artStyle")]
    pub art_style: String,
    #[serde(rename = "artStylePrompt")]
    pub art_style_prompt: Option<String>,
    #[serde(rename = "characterModel")]
    pub character_model: Option<String>,
    #[serde(rename = "locationModel")]
    pub location_model: Option<String>,
    #[serde(rename = "storyboardModel")]
    pub storyboard_model: Option<String>,
    #[serde(rename = "editModel")]
    pub edit_model: Option<String>,
    #[serde(rename = "videoResolution")]
    pub video_resolution: String,
    #[serde(rename = "capabilityOverrides")]
    pub capability_overrides: Option<String>,
    #[serde(rename = "workflowMode")]
    pub workflow_mode: String,
    #[serde(rename = "lastEpisodeId")]
    pub last_episode_id: Option<String>,
    #[serde(rename = "imageResolution")]
    pub image_resolution: String,
    #[serde(rename = "importStatus")]
    pub import_status: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NovelPromotionShot {
    pub id: String,
    #[serde(rename = "episodeId")]
    pub episode_id: String,
    #[serde(rename = "clipId")]
    pub clip_id: Option<String>,
    #[serde(rename = "shotId")]
    pub shot_id: String,
    #[serde(rename = "srtStart")]
    pub srt_start: i32,
    #[serde(rename = "srtEnd")]
    pub srt_end: i32,
    #[serde(rename = "srtDuration")]
    pub srt_duration: f64,
    pub sequence: Option<String>,
    pub locations: Option<String>,
    pub characters: Option<String>,
    pub plot: Option<String>,
    #[serde(rename = "imagePrompt")]
    pub image_prompt: Option<String>,
    pub scale: Option<String>,
    pub module: Option<String>,
    pub focus: Option<String>,
    #[serde(rename = "zhSummarize")]
    pub zh_summarize: Option<String>,
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[serde(rename = "imageMediaId")]
    pub image_media_id: Option<String>,
    pub pov: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NovelPromotionStoryboard {
    pub id: String,
    #[serde(rename = "episodeId")]
    pub episode_id: String,
    #[serde(rename = "clipId")]
    pub clip_id: String,
    #[serde(rename = "storyboardImageUrl")]
    pub storyboard_image_url: Option<String>,
    #[serde(rename = "panelCount")]
    pub panel_count: i32,
    #[serde(rename = "storyboardTextJson")]
    pub storyboard_text_json: Option<String>,
    #[serde(rename = "imageHistory")]
    pub image_history: Option<String>,
    #[serde(rename = "candidateImages")]
    pub candidate_images: Option<String>,
    #[serde(rename = "lastError")]
    pub last_error: Option<String>,
    #[serde(rename = "photographyPlan")]
    pub photography_plan: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SupplementaryPanel {
    pub id: String,
    #[serde(rename = "storyboardId")]
    pub storyboard_id: String,
    #[serde(rename = "sourceType")]
    pub source_type: String,
    #[serde(rename = "sourcePanelId")]
    pub source_panel_id: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "imagePrompt")]
    pub image_prompt: Option<String>,
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[serde(rename = "imageMediaId")]
    pub image_media_id: Option<String>,
    pub characters: Option<String>,
    pub location: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub mode: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "lastAccessedAt")]
    pub last_accessed_at: Option<NaiveDateTime>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: String,
    #[serde(rename = "sessionToken")]
    pub session_token: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    pub expires: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    #[serde(rename = "emailVerified")]
    pub email_verified: Option<NaiveDateTime>,
    pub image: Option<String>,
    pub password: Option<String>,
    pub role: String,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserPreference {
    pub id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "analysisModel")]
    pub analysis_model: Option<String>,
    #[serde(rename = "characterModel")]
    pub character_model: Option<String>,
    #[serde(rename = "locationModel")]
    pub location_model: Option<String>,
    #[serde(rename = "storyboardModel")]
    pub storyboard_model: Option<String>,
    #[serde(rename = "editModel")]
    pub edit_model: Option<String>,
    #[serde(rename = "videoModel")]
    pub video_model: Option<String>,
    #[serde(rename = "lipSyncModel")]
    pub lip_sync_model: Option<String>,
    #[serde(rename = "videoRatio")]
    pub video_ratio: String,
    #[serde(rename = "videoResolution")]
    pub video_resolution: String,
    #[serde(rename = "artStyle")]
    pub art_style: String,
    #[serde(rename = "ttsRate")]
    pub tts_rate: String,
    #[serde(rename = "imageResolution")]
    pub image_resolution: String,
    #[serde(rename = "capabilityDefaults")]
    pub capability_defaults: Option<String>,
    #[serde(rename = "llmBaseUrl")]
    pub llm_base_url: Option<String>,
    #[serde(rename = "llmApiKey")]
    pub llm_api_key: Option<String>,
    #[serde(rename = "falApiKey")]
    pub fal_api_key: Option<String>,
    #[serde(rename = "googleAiKey")]
    pub google_ai_key: Option<String>,
    #[serde(rename = "arkApiKey")]
    pub ark_api_key: Option<String>,
    #[serde(rename = "qwenApiKey")]
    pub qwen_api_key: Option<String>,
    #[serde(rename = "customModels")]
    pub custom_models: Option<String>,
    #[serde(rename = "customProviders")]
    pub custom_providers: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SystemConfig {
    pub key: String,
    pub value: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
    #[serde(rename = "updatedBy")]
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct VerificationToken {
    pub identifier: String,
    pub token: String,
    pub expires: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NovelPromotionVoiceLine {
    pub id: String,
    #[serde(rename = "episodeId")]
    pub episode_id: String,
    #[serde(rename = "lineIndex")]
    pub line_index: i32,
    pub speaker: String,
    pub content: String,
    #[serde(rename = "voicePresetId")]
    pub voice_preset_id: Option<String>,
    #[serde(rename = "audioUrl")]
    pub audio_url: Option<String>,
    #[serde(rename = "audioMediaId")]
    pub audio_media_id: Option<String>,
    #[serde(rename = "emotionPrompt")]
    pub emotion_prompt: Option<String>,
    #[serde(rename = "emotionStrength")]
    pub emotion_strength: Option<f64>,
    #[serde(rename = "matchedPanelIndex")]
    pub matched_panel_index: Option<i32>,
    #[serde(rename = "matchedStoryboardId")]
    pub matched_storyboard_id: Option<String>,
    #[serde(rename = "audioDuration")]
    pub audio_duration: Option<i32>,
    #[serde(rename = "matchedPanelId")]
    pub matched_panel_id: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct VoicePreset {
    pub id: String,
    pub name: String,
    #[serde(rename = "audioUrl")]
    pub audio_url: String,
    #[serde(rename = "audioMediaId")]
    pub audio_media_id: Option<String>,
    pub description: Option<String>,
    pub gender: Option<String>,
    #[serde(rename = "isSystem")]
    pub is_system: bool,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    pub id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "projectId")]
    pub project_id: String,
    #[serde(rename = "episodeId")]
    pub episode_id: Option<String>,
    #[serde(rename = "type")]
    pub task_type: String,
    #[serde(rename = "targetType")]
    pub target_type: String,
    #[serde(rename = "targetId")]
    pub target_id: String,
    pub status: String,
    pub progress: i32,
    pub attempt: i32,
    #[serde(rename = "maxAttempts")]
    pub max_attempts: i32,
    pub priority: i32,
    #[serde(rename = "dedupeKey")]
    pub dedupe_key: Option<String>,
    #[serde(rename = "externalId")]
    pub external_id: Option<String>,
    pub payload: Option<Json<Value>>,
    pub result: Option<Json<Value>>,
    #[serde(rename = "errorCode")]
    pub error_code: Option<String>,
    #[serde(rename = "errorMessage")]
    pub error_message: Option<String>,
    #[serde(rename = "queuedAt")]
    pub queued_at: NaiveDateTime,
    #[serde(rename = "startedAt")]
    pub started_at: Option<NaiveDateTime>,
    #[serde(rename = "finishedAt")]
    pub finished_at: Option<NaiveDateTime>,
    #[serde(rename = "heartbeatAt")]
    pub heartbeat_at: Option<NaiveDateTime>,
    #[serde(rename = "enqueuedAt")]
    pub enqueued_at: Option<NaiveDateTime>,
    #[serde(rename = "enqueueAttempts")]
    pub enqueue_attempts: i32,
    #[serde(rename = "lastEnqueueError")]
    pub last_enqueue_error: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TaskEvent {
    pub id: i64,
    #[serde(rename = "taskId")]
    pub task_id: String,
    #[serde(rename = "projectId")]
    pub project_id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "eventType")]
    pub event_type: String,
    pub payload: Option<Json<Value>>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GlobalAssetFolder {
    pub id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    pub name: String,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GlobalCharacter {
    pub id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "folderId")]
    pub folder_id: Option<String>,
    pub name: String,
    pub aliases: Option<String>,
    #[serde(rename = "profileData")]
    pub profile_data: Option<String>,
    #[serde(rename = "profileConfirmed")]
    pub profile_confirmed: bool,
    #[serde(rename = "voiceId")]
    pub voice_id: Option<String>,
    #[serde(rename = "voiceType")]
    pub voice_type: Option<String>,
    #[serde(rename = "customVoiceUrl")]
    pub custom_voice_url: Option<String>,
    #[serde(rename = "customVoiceMediaId")]
    pub custom_voice_media_id: Option<String>,
    #[serde(rename = "globalVoiceId")]
    pub global_voice_id: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GlobalCharacterAppearance {
    pub id: String,
    #[serde(rename = "characterId")]
    pub character_id: String,
    #[serde(rename = "appearanceIndex")]
    pub appearance_index: i32,
    #[serde(rename = "changeReason")]
    pub change_reason: String,
    pub description: Option<String>,
    pub descriptions: Option<String>,
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[serde(rename = "imageMediaId")]
    pub image_media_id: Option<String>,
    #[serde(rename = "imageUrls")]
    pub image_urls: Option<String>,
    #[serde(rename = "selectedIndex")]
    pub selected_index: Option<i32>,
    #[serde(rename = "previousImageUrl")]
    pub previous_image_url: Option<String>,
    #[serde(rename = "previousImageMediaId")]
    pub previous_image_media_id: Option<String>,
    #[serde(rename = "previousImageUrls")]
    pub previous_image_urls: Option<String>,
    #[serde(rename = "previousDescription")]
    pub previous_description: Option<String>,
    #[serde(rename = "previousDescriptions")]
    pub previous_descriptions: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GlobalLocation {
    pub id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "folderId")]
    pub folder_id: Option<String>,
    pub name: String,
    pub summary: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GlobalLocationImage {
    pub id: String,
    #[serde(rename = "locationId")]
    pub location_id: String,
    #[serde(rename = "imageIndex")]
    pub image_index: i32,
    pub description: Option<String>,
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[serde(rename = "imageMediaId")]
    pub image_media_id: Option<String>,
    #[serde(rename = "isSelected")]
    pub is_selected: bool,
    #[serde(rename = "previousImageUrl")]
    pub previous_image_url: Option<String>,
    #[serde(rename = "previousImageMediaId")]
    pub previous_image_media_id: Option<String>,
    #[serde(rename = "previousDescription")]
    pub previous_description: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GlobalVoice {
    pub id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "folderId")]
    pub folder_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "voiceId")]
    pub voice_id: Option<String>,
    #[serde(rename = "voiceType")]
    pub voice_type: String,
    #[serde(rename = "customVoiceUrl")]
    pub custom_voice_url: Option<String>,
    #[serde(rename = "customVoiceMediaId")]
    pub custom_voice_media_id: Option<String>,
    #[serde(rename = "voicePrompt")]
    pub voice_prompt: Option<String>,
    pub gender: Option<String>,
    pub language: String,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MediaObject {
    pub id: String,
    #[serde(rename = "publicId")]
    pub public_id: String,
    #[serde(rename = "storageKey")]
    pub storage_key: String,
    pub sha256: Option<String>,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: Option<i64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    #[serde(rename = "durationMs")]
    pub duration_ms: Option<i32>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LegacyMediaRefBackup {
    pub id: String,
    #[serde(rename = "runId")]
    pub run_id: String,
    #[serde(rename = "tableName")]
    pub table_name: String,
    #[serde(rename = "rowId")]
    pub row_id: String,
    #[serde(rename = "fieldName")]
    pub field_name: String,
    #[serde(rename = "legacyValue")]
    pub legacy_value: String,
    pub checksum: String,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
}

// Runtime graph tables are managed outside Prisma schema.

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GraphRun {
    pub id: String,
    #[serde(rename = "projectId")]
    pub project_id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "episodeId")]
    pub episode_id: Option<String>,
    #[serde(rename = "workflowType")]
    pub workflow_type: String,
    #[serde(rename = "taskType")]
    pub task_type: Option<String>,
    #[serde(rename = "taskId")]
    pub task_id: Option<String>,
    #[serde(rename = "targetType")]
    pub target_type: String,
    #[serde(rename = "targetId")]
    pub target_id: String,
    pub status: String,
    pub input: Option<Json<Value>>,
    pub output: Option<Json<Value>>,
    #[serde(rename = "errorCode")]
    pub error_code: Option<String>,
    #[serde(rename = "errorMessage")]
    pub error_message: Option<String>,
    #[serde(rename = "cancelRequestedAt")]
    pub cancel_requested_at: Option<NaiveDateTime>,
    #[serde(rename = "queuedAt")]
    pub queued_at: NaiveDateTime,
    #[serde(rename = "startedAt")]
    pub started_at: Option<NaiveDateTime>,
    #[serde(rename = "finishedAt")]
    pub finished_at: Option<NaiveDateTime>,
    #[serde(rename = "lastSeq")]
    pub last_seq: i32,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GraphStep {
    pub id: String,
    #[serde(rename = "runId")]
    pub run_id: String,
    #[serde(rename = "stepKey")]
    pub step_key: String,
    #[serde(rename = "stepTitle")]
    pub step_title: String,
    pub status: String,
    #[serde(rename = "currentAttempt")]
    pub current_attempt: i32,
    #[serde(rename = "stepIndex")]
    pub step_index: i32,
    #[serde(rename = "stepTotal")]
    pub step_total: i32,
    #[serde(rename = "startedAt")]
    pub started_at: Option<NaiveDateTime>,
    #[serde(rename = "finishedAt")]
    pub finished_at: Option<NaiveDateTime>,
    #[serde(rename = "lastErrorCode")]
    pub last_error_code: Option<String>,
    #[serde(rename = "lastErrorMessage")]
    pub last_error_message: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GraphEvent {
    pub id: i64,
    #[serde(rename = "runId")]
    pub run_id: String,
    #[serde(rename = "projectId")]
    pub project_id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    pub seq: i32,
    #[serde(rename = "eventType")]
    pub event_type: String,
    #[serde(rename = "stepKey")]
    pub step_key: Option<String>,
    pub attempt: Option<i32>,
    pub lane: Option<String>,
    pub payload: Option<Json<Value>>,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GraphCheckpoint {
    pub id: String,
    #[serde(rename = "runId")]
    pub run_id: String,
    #[serde(rename = "nodeKey")]
    pub node_key: String,
    pub version: i32,
    #[serde(rename = "stateJson")]
    pub state_json: Json<Value>,
    #[serde(rename = "stateBytes")]
    pub state_bytes: i32,
    #[serde(rename = "createdAt")]
    pub created_at: NaiveDateTime,
}
