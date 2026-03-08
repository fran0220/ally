use serde_json::{Value, json};
use sqlx::MySqlPool;

use crate::{
    api_config::UnifiedModelType,
    errors::AppError,
    media,
    runtime::{resolve_model_with_provider, resolve_provider_config},
};

use super::{VoiceDesignInput, fal::poll_fal_result, fal::submit_fal_task, http_client};

pub async fn generate_lip_sync(
    pool: &MySqlPool,
    model_key: &str,
    video_source: &str,
    audio_source: &str,
) -> Result<String, AppError> {
    let (model, provider) =
        resolve_model_with_provider(pool, model_key, Some(UnifiedModelType::Lipsync)).await?;

    match provider.provider_key.as_str() {
        "fal" => {
            let client = http_client();
            let payload = json!({
                "video_url": media::normalize_source_to_data_url(video_source).await?,
                "audio_url": media::normalize_source_to_data_url(audio_source).await?,
            });

            let request_id =
                submit_fal_task(&client, &model.model_id, &provider.api_key, payload).await?;
            poll_fal_result(&client, &model.model_id, &provider.api_key, &request_id).await
        }
        _ => Err(AppError::invalid_params(format!(
            "unsupported lip sync provider: {}",
            provider.id
        ))),
    }
}

pub async fn generate_voice_clone(
    pool: &MySqlPool,
    model_key: &str,
    reference_audio_source: &str,
    text: &str,
    emotion_prompt: Option<&str>,
    strength: Option<f64>,
) -> Result<String, AppError> {
    let (model, provider) =
        resolve_model_with_provider(pool, model_key, Some(UnifiedModelType::Audio)).await?;

    if provider.provider_key != "fal" {
        return Err(AppError::invalid_params(format!(
            "unsupported voice provider: {}",
            provider.id
        )));
    }

    let mut payload = json!({
        "audio_url": media::normalize_source_to_data_url(reference_audio_source).await?,
        "prompt": text,
        "should_use_prompt_for_emotion": true,
        "strength": strength.unwrap_or(0.4),
    });
    if let Some(prompt) = emotion_prompt
        && !prompt.trim().is_empty()
    {
        payload["emotion_prompt"] = Value::String(prompt.trim().to_string());
    }

    let client = http_client();
    let request_id = submit_fal_task(&client, &model.model_id, &provider.api_key, payload).await?;
    poll_fal_result(&client, &model.model_id, &provider.api_key, &request_id).await
}

pub async fn create_voice_design(
    pool: &MySqlPool,
    input: VoiceDesignInput,
) -> Result<Value, AppError> {
    let provider = resolve_provider_config(pool, "qwen").await?;

    let payload = json!({
        "model": "qwen-voice-design",
        "input": {
            "action": "create",
            "target_model": "qwen3-tts-vd-realtime-2025-12-16",
            "voice_prompt": input.voice_prompt,
            "preview_text": input.preview_text,
            "preferred_name": input.preferred_name,
            "language": input.language,
        },
        "parameters": {
            "sample_rate": 24000,
            "response_format": "wav",
        }
    });

    let base_url = provider
        .base_url
        .as_deref()
        .unwrap_or("https://dashscope.aliyuncs.com");
    let endpoint = format!(
        "{}/api/v1/services/audio/tts/customization",
        base_url.trim_end_matches('/')
    );

    let client = http_client();
    let response = client
        .post(endpoint)
        .bearer_auth(provider.api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|err| AppError::internal(format!("voice design request failed: {err}")))?;

    let value: Value = response
        .json()
        .await
        .map_err(|err| AppError::internal(format!("invalid voice design response: {err}")))?;

    if value.get("output").is_none() {
        let message = value
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("voice design failed");
        return Err(AppError::internal(message.to_string()));
    }

    Ok(value)
}
