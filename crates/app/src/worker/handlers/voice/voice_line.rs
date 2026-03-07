use serde_json::{Value, json};
use sqlx::FromRow;
use waoowaoo_core::api_config::{self, UnifiedModelType};
use waoowaoo_core::errors::AppError;
use waoowaoo_core::generators;
use waoowaoo_core::media;

use crate::{handlers::image::shared, runtime, task_context::TaskContext};

#[derive(Debug, FromRow)]
struct VoiceLineTaskRow {
    id: String,
    #[sqlx(rename = "episodeId")]
    episode_id: String,
    speaker: String,
    content: String,
    #[sqlx(rename = "emotionPrompt")]
    emotion_prompt: Option<String>,
    #[sqlx(rename = "emotionStrength")]
    emotion_strength: Option<f64>,
}

#[derive(Debug, FromRow)]
struct CharacterVoiceRow {
    name: String,
    #[sqlx(rename = "customVoiceUrl")]
    custom_voice_url: Option<String>,
}

#[derive(Debug, FromRow)]
struct EpisodeSpeakerVoicesRow {
    #[sqlx(rename = "speakerVoices")]
    speaker_voices: Option<String>,
}

fn match_character_reference<'a>(
    speaker: &str,
    characters: &'a [CharacterVoiceRow],
) -> Option<&'a str> {
    let speaker_trimmed = speaker.trim();
    if speaker_trimmed.is_empty() {
        return None;
    }

    if let Some(found) = characters
        .iter()
        .find(|item| item.name.trim().eq_ignore_ascii_case(speaker_trimmed))
        .and_then(|item| item.custom_voice_url.as_deref())
    {
        return Some(found);
    }

    characters
        .iter()
        .find(|item| {
            let name = item.name.trim().to_lowercase();
            let speaker_lc = speaker_trimmed.to_lowercase();
            !name.is_empty() && (name.contains(&speaker_lc) || speaker_lc.contains(&name))
        })
        .and_then(|item| item.custom_voice_url.as_deref())
}

fn parse_speaker_voice_reference(raw: Option<&str>, speaker: &str) -> Option<String> {
    let value = raw?.trim();
    if value.is_empty() {
        return None;
    }

    let parsed: Value = serde_json::from_str(value).ok()?;
    let object = parsed.as_object()?;
    let entry = object.get(speaker)?;

    if let Some(audio_url) = entry.as_str() {
        let trimmed = audio_url.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    entry
        .get("audioUrl")
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

async fn resolve_audio_model(payload: &Value) -> Result<String, AppError> {
    if let Some(model) =
        shared::read_string(payload, "audioModel").or_else(|| shared::read_string(payload, "model"))
    {
        return Ok(model);
    }

    let mysql = runtime::mysql()?;
    let enabled_audio_models = api_config::get_system_models_raw(mysql)
        .await?
        .into_iter()
        .filter(|model| model.enabled && model.model_type == UnifiedModelType::Audio)
        .collect::<Vec<_>>();

    if enabled_audio_models.is_empty() {
        return Err(AppError::invalid_params("audio model is not configured"));
    }
    if enabled_audio_models.len() > 1 {
        return Err(AppError::invalid_params(
            "audioModel is required when multiple audio models are enabled",
        ));
    }

    Ok(enabled_audio_models[0].model_key.clone())
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    let mysql = runtime::mysql()?;
    let payload = &task.payload;

    let line_id = shared::read_string(payload, "lineId")
        .or_else(|| shared::read_string(payload, "targetId"))
        .or_else(|| {
            if !task.target_id.trim().is_empty() {
                Some(task.target_id.clone())
            } else {
                None
            }
        })
        .ok_or_else(|| AppError::invalid_params("lineId is required"))?;

    let line = sqlx::query_as::<_, VoiceLineTaskRow>(
        "SELECT id, episodeId, speaker, content, emotionPrompt, emotionStrength FROM novel_promotion_voice_lines WHERE id = ? LIMIT 1",
    )
    .bind(&line_id)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("voice line not found"))?;

    let _ = task
        .report_progress(20, Some("generate_voice_submit"))
        .await?;

    let text = line.content.trim();
    if text.is_empty() {
        return Err(AppError::invalid_params("voice line content is empty"));
    }

    let characters = sqlx::query_as::<_, CharacterVoiceRow>(
        "SELECT c.name, c.customVoiceUrl FROM novel_promotion_characters c INNER JOIN novel_promotion_projects np ON np.id = c.novelPromotionProjectId WHERE np.projectId = ?",
    )
    .bind(&task.project_id)
    .fetch_all(mysql)
    .await?;

    let episode_speaker_voices = sqlx::query_as::<_, EpisodeSpeakerVoicesRow>(
        "SELECT speakerVoices FROM novel_promotion_episodes WHERE id = ? LIMIT 1",
    )
    .bind(&line.episode_id)
    .fetch_optional(mysql)
    .await?;

    let payload_reference = shared::read_string(payload, "referenceAudioUrl");
    let character_reference = match_character_reference(&line.speaker, &characters)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty());
    let speaker_reference = parse_speaker_voice_reference(
        episode_speaker_voices
            .as_ref()
            .and_then(|item| item.speaker_voices.as_deref()),
        &line.speaker,
    );

    let reference_audio = payload_reference
        .or(character_reference)
        .or(speaker_reference)
        .ok_or_else(|| AppError::invalid_params("reference audio is required for voice line"))?;
    let reference_audio = media::to_public_media_url(Some(&reference_audio))
        .ok_or_else(|| AppError::invalid_params("invalid reference audio"))?;

    let audio_model = resolve_audio_model(payload).await?;

    let generated_source = generators::generate_voice_clone(
        mysql,
        &audio_model,
        &reference_audio,
        text,
        line.emotion_prompt.as_deref(),
        line.emotion_strength,
    )
    .await?;

    let storage_key =
        media::upload_source_to_storage(&generated_source, "voice-line", &line.id).await?;

    let _ = task
        .report_progress(95, Some("generate_voice_persist"))
        .await?;

    sqlx::query(
        "UPDATE novel_promotion_voice_lines SET audioUrl = ?, updatedAt = NOW(3) WHERE id = ?",
    )
    .bind(&storage_key)
    .bind(&line.id)
    .execute(mysql)
    .await?;

    let audio_url = media::to_public_media_url(Some(&storage_key)).unwrap_or(storage_key.clone());

    Ok(json!({
        "lineId": line.id,
        "episodeId": line.episode_id,
        "speaker": line.speaker,
        "audioUrl": audio_url,
        "storageKey": storage_key,
        "model": audio_model,
    }))
}
