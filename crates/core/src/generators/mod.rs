use std::time::{Duration, Instant};

use reqwest::{Client, multipart};
use serde_json::{Value, json};
use sqlx::MySqlPool;

use crate::{
    api_config::UnifiedModelType,
    errors::AppError,
    media,
    runtime::{resolve_model_with_provider, resolve_provider_config},
};

const FAL_POLL_INTERVAL_MS: u64 = 3000;
const FAL_TIMEOUT_SECS: u64 = 20 * 60;

#[derive(Debug, Clone, Default)]
pub struct ImageGenerateOptions {
    pub reference_images: Vec<String>,
    pub aspect_ratio: Option<String>,
    pub resolution: Option<String>,
    pub output_format: Option<String>,
    pub quality: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct VideoGenerateOptions {
    pub prompt: Option<String>,
    pub duration: Option<u32>,
    pub resolution: Option<String>,
    pub aspect_ratio: Option<String>,
    pub generation_mode: Option<String>,
    pub generate_audio: Option<bool>,
    pub last_frame_image_source: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VoiceDesignInput {
    pub voice_prompt: String,
    pub preview_text: String,
    pub preferred_name: String,
    pub language: String,
}

fn http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .unwrap_or_else(|_| Client::new())
}

fn parse_fal_base_endpoint(endpoint: &str) -> String {
    let segments = endpoint.split('/').collect::<Vec<_>>();
    if segments.len() >= 2 {
        format!("{}/{}", segments[0], segments[1])
    } else {
        endpoint.to_string()
    }
}

fn extract_media_url(value: &Value) -> Option<String> {
    value
        .get("video")
        .and_then(|video| video.get("url"))
        .and_then(Value::as_str)
        .map(|item| item.to_string())
        .or_else(|| {
            value
                .get("audio")
                .and_then(|audio| audio.get("url"))
                .and_then(Value::as_str)
                .map(|item| item.to_string())
        })
        .or_else(|| {
            value
                .get("images")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(|item| item.get("url"))
                .and_then(Value::as_str)
                .map(|item| item.to_string())
        })
        .or_else(|| {
            value
                .get("resultUrl")
                .and_then(Value::as_str)
                .map(|item| item.to_string())
        })
        .or_else(|| {
            value
                .get("imageUrl")
                .and_then(Value::as_str)
                .map(|item| item.to_string())
        })
        .or_else(|| {
            value
                .get("videoUrl")
                .and_then(Value::as_str)
                .map(|item| item.to_string())
        })
}

async fn submit_fal_task(
    client: &Client,
    endpoint: &str,
    api_key: &str,
    payload: Value,
) -> Result<String, AppError> {
    let response = client
        .post(format!("https://queue.fal.run/{endpoint}"))
        .header("Authorization", format!("Key {api_key}"))
        .json(&payload)
        .send()
        .await
        .map_err(|err| AppError::internal(format!("failed to submit fal task: {err}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::internal(format!(
            "fal submit failed ({status}): {body}"
        )));
    }

    let json_value: Value = response
        .json()
        .await
        .map_err(|err| AppError::internal(format!("invalid fal submit response: {err}")))?;

    json_value
        .get("request_id")
        .and_then(Value::as_str)
        .map(|item| item.to_string())
        .ok_or_else(|| AppError::internal("fal submit response missing request_id"))
}

async fn poll_fal_result(
    client: &Client,
    endpoint: &str,
    api_key: &str,
    request_id: &str,
) -> Result<String, AppError> {
    let started_at = Instant::now();
    let base_endpoint = parse_fal_base_endpoint(endpoint);

    loop {
        if started_at.elapsed().as_secs() > FAL_TIMEOUT_SECS {
            return Err(AppError::internal(format!(
                "fal polling timeout after {}s",
                FAL_TIMEOUT_SECS
            )));
        }

        let status_response = client
            .get(format!(
                "https://queue.fal.run/{base_endpoint}/requests/{request_id}/status?logs=0"
            ))
            .header("Authorization", format!("Key {api_key}"))
            .send()
            .await
            .map_err(|err| AppError::internal(format!("fal status request failed: {err}")))?;

        if status_response.status().is_success() {
            let status_json: Value = status_response
                .json()
                .await
                .map_err(|err| AppError::internal(format!("invalid fal status response: {err}")))?;

            let status = status_json
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("IN_PROGRESS");

            if status == "FAILED" {
                let message = status_json
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("fal task failed");
                return Err(AppError::internal(message.to_string()));
            }

            if status == "COMPLETED" {
                let response_url = status_json
                    .get("response_url")
                    .and_then(Value::as_str)
                    .map(|item| item.to_string())
                    .unwrap_or_else(|| {
                        format!("https://queue.fal.run/{endpoint}/requests/{request_id}")
                    });

                let result_response = client
                    .get(response_url)
                    .header("Authorization", format!("Key {api_key}"))
                    .send()
                    .await
                    .map_err(|err| {
                        AppError::internal(format!("fal result request failed: {err}"))
                    })?;

                if !result_response.status().is_success() {
                    let status = result_response.status();
                    let body = result_response.text().await.unwrap_or_default();
                    return Err(AppError::internal(format!(
                        "fal result request failed ({status}): {body}"
                    )));
                }

                let result_json: Value = result_response.json().await.map_err(|err| {
                    AppError::internal(format!("invalid fal result response: {err}"))
                })?;
                return extract_media_url(&result_json)
                    .ok_or_else(|| AppError::internal("fal result missing media url"));
            }
        }

        tokio::time::sleep(Duration::from_millis(FAL_POLL_INTERVAL_MS)).await;
    }
}

fn fal_image_endpoint(model_id: &str, is_edit: bool) -> Option<String> {
    let value = model_id.trim();
    if value.is_empty() {
        return None;
    }

    let endpoint = match value {
        "banana" => {
            if is_edit {
                "fal-ai/nano-banana-pro/edit"
            } else {
                "fal-ai/nano-banana-pro"
            }
        }
        "banana-2" => {
            if is_edit {
                "fal-ai/nano-banana-2/edit"
            } else {
                "fal-ai/nano-banana-2"
            }
        }
        other => {
            if other.contains('/') {
                other
            } else {
                return None;
            }
        }
    };

    Some(endpoint.to_string())
}

fn fal_video_endpoint(model_id: &str) -> Option<String> {
    let value = model_id.trim();
    if value.is_empty() {
        return None;
    }

    let endpoint = match value {
        "fal-wan25" => "wan/v2.6/image-to-video",
        "fal-veo31" => "fal-ai/veo3.1/fast/image-to-video",
        "fal-sora2" => "fal-ai/sora-2/image-to-video",
        "fal-ai/kling-video/v2.5-turbo/pro/image-to-video" => {
            "fal-ai/kling-video/v2.5-turbo/pro/image-to-video"
        }
        "fal-ai/kling-video/v3/standard/image-to-video" => {
            "fal-ai/kling-video/v3/standard/image-to-video"
        }
        "fal-ai/kling-video/v3/pro/image-to-video" => "fal-ai/kling-video/v3/pro/image-to-video",
        other => {
            if other.contains('/') {
                other
            } else {
                return None;
            }
        }
    };

    Some(endpoint.to_string())
}

async fn generate_image_with_fal(
    api_key: &str,
    model_id: &str,
    prompt: &str,
    options: &ImageGenerateOptions,
) -> Result<String, AppError> {
    let client = http_client();
    let normalized_refs =
        media::normalize_reference_sources_to_data_urls(&options.reference_images).await?;
    let is_edit = !normalized_refs.is_empty();
    let endpoint = fal_image_endpoint(model_id, is_edit).ok_or_else(|| {
        AppError::invalid_params(format!("unsupported fal image model: {model_id}"))
    })?;

    let mut payload = json!({
        "prompt": prompt,
        "num_images": 1,
        "output_format": options.output_format.as_deref().unwrap_or("png"),
    });

    if let Some(aspect_ratio) = options.aspect_ratio.as_ref() {
        payload["aspect_ratio"] = Value::String(aspect_ratio.clone());
    }
    if let Some(resolution) = options.resolution.as_ref() {
        payload["resolution"] = Value::String(resolution.clone());
    }
    if is_edit {
        payload["image_urls"] =
            Value::Array(normalized_refs.iter().cloned().map(Value::String).collect());
    }

    let request_id = submit_fal_task(&client, &endpoint, api_key, payload).await?;
    poll_fal_result(&client, &endpoint, api_key, &request_id).await
}

async fn generate_image_with_openai_compatible(
    base_url: &str,
    api_key: &str,
    model_id: &str,
    prompt: &str,
    options: &ImageGenerateOptions,
) -> Result<String, AppError> {
    let client = http_client();
    let base_url = base_url.trim_end_matches('/');

    if options.reference_images.is_empty() {
        let mut payload = json!({
            "model": model_id,
            "prompt": prompt,
            "response_format": "b64_json",
        });
        if let Some(resolution) = options.resolution.as_ref() {
            payload["size"] = Value::String(resolution.clone());
        }
        if let Some(quality) = options.quality.as_ref() {
            payload["quality"] = Value::String(quality.clone());
        }

        let response = client
            .post(format!("{base_url}/images/generations"))
            .bearer_auth(api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|err| {
                AppError::internal(format!("openai-compatible image generate failed: {err}"))
            })?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::internal(format!(
                "openai-compatible image generate failed ({status}): {body}"
            )));
        }

        let value: Value = response
            .json()
            .await
            .map_err(|err| AppError::internal(format!("invalid image response: {err}")))?;
        if let Some(b64_json) = value
            .get("data")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(|item| item.get("b64_json"))
            .and_then(Value::as_str)
        {
            return Ok(format!("data:image/png;base64,{b64_json}"));
        }
        if let Some(url) = value
            .get("data")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(|item| item.get("url"))
            .and_then(Value::as_str)
        {
            return Ok(url.to_string());
        }

        return Err(AppError::internal(
            "openai-compatible image response missing image content",
        ));
    }

    let mut form = multipart::Form::new()
        .text("model", model_id.to_string())
        .text("prompt", prompt.to_string())
        .text("response_format", "b64_json".to_string());
    if let Some(resolution) = options.resolution.as_ref() {
        form = form.text("size", resolution.clone());
    }
    if let Some(quality) = options.quality.as_ref() {
        form = form.text("quality", quality.clone());
    }

    for (index, reference) in options.reference_images.iter().enumerate() {
        let (bytes, content_type) = media::download_source_bytes(reference).await?;
        let part = multipart::Part::bytes(bytes)
            .file_name(format!("reference-{index}.png"))
            .mime_str(&content_type)
            .map_err(|err| AppError::internal(format!("invalid reference mime type: {err}")))?;
        form = form.part("image", part);
    }

    let response = client
        .post(format!("{base_url}/images/edits"))
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|err| AppError::internal(format!("openai-compatible image edit failed: {err}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::internal(format!(
            "openai-compatible image edit failed ({status}): {body}"
        )));
    }

    let value: Value = response
        .json()
        .await
        .map_err(|err| AppError::internal(format!("invalid image edit response: {err}")))?;
    if let Some(b64_json) = value
        .get("data")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("b64_json"))
        .and_then(Value::as_str)
    {
        return Ok(format!("data:image/png;base64,{b64_json}"));
    }
    if let Some(url) = value
        .get("data")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("url"))
        .and_then(Value::as_str)
    {
        return Ok(url.to_string());
    }

    Err(AppError::internal(
        "openai-compatible image edit response missing image content",
    ))
}

async fn generate_image_with_gemini_compatible(
    base_url: &str,
    api_key: &str,
    model_id: &str,
    prompt: &str,
    options: &ImageGenerateOptions,
) -> Result<String, AppError> {
    let client = http_client();

    let base_url = base_url.trim_end_matches('/');
    let endpoint = if base_url.contains("/v1beta") {
        format!("{base_url}/models/{model_id}:generateContent")
    } else {
        format!("{base_url}/v1beta/models/{model_id}:generateContent")
    };

    let normalized_refs =
        media::normalize_reference_sources_to_data_urls(&options.reference_images).await?;
    let mut parts = Vec::with_capacity(normalized_refs.len() + 1);
    for item in normalized_refs {
        let (mime_type, data) = media::parse_data_url(&item)
            .ok_or_else(|| AppError::invalid_params("invalid data url reference image"))?;
        parts.push(json!({
            "inlineData": {
                "mimeType": mime_type,
                "data": data,
            }
        }));
    }
    parts.push(json!({ "text": prompt }));

    let mut config = json!({
        "responseModalities": ["IMAGE", "TEXT"],
    });
    if options.aspect_ratio.is_some() || options.resolution.is_some() {
        config["imageConfig"] = json!({
            "aspectRatio": options.aspect_ratio,
            "imageSize": options.resolution,
        });
    }

    let payload = json!({
        "contents": [{ "parts": parts }],
        "config": config,
    });

    let response = client
        .post(endpoint)
        .header("x-goog-api-key", api_key)
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|err| AppError::internal(format!("gemini-compatible request failed: {err}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::internal(format!(
            "gemini-compatible image generate failed ({status}): {body}"
        )));
    }

    let value: Value = response
        .json()
        .await
        .map_err(|err| AppError::internal(format!("invalid gemini response: {err}")))?;

    let parts = value
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("content"))
        .and_then(|item| item.get("parts"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    for part in parts {
        if let Some(inline) = part.get("inlineData") {
            let mime_type = inline
                .get("mimeType")
                .and_then(Value::as_str)
                .unwrap_or("image/png");
            if let Some(data) = inline.get("data").and_then(Value::as_str) {
                return Ok(format!("data:{mime_type};base64,{data}"));
            }
        }
    }

    Err(AppError::internal(
        "gemini-compatible response missing image payload",
    ))
}

fn parse_resolution_size(
    resolution: Option<&String>,
    aspect_ratio: Option<&String>,
) -> Option<String> {
    let resolution = resolution?.trim();
    let aspect = aspect_ratio.map(|value| value.as_str()).unwrap_or("16:9");
    match resolution {
        "720p" => {
            if aspect == "9:16" {
                Some("720x1280".to_string())
            } else {
                Some("1280x720".to_string())
            }
        }
        "1080p" => {
            if aspect == "9:16" {
                Some("1024x1792".to_string())
            } else {
                Some("1792x1024".to_string())
            }
        }
        value if value.contains('x') => Some(value.to_string()),
        _ => None,
    }
}

pub async fn generate_image(
    pool: &MySqlPool,
    model_key: &str,
    prompt: &str,
    mut options: ImageGenerateOptions,
) -> Result<String, AppError> {
    let (model, provider) =
        resolve_model_with_provider(pool, model_key, Some(UnifiedModelType::Image)).await?;

    if options.resolution.is_none() {
        options.resolution =
            parse_resolution_size(options.resolution.as_ref(), options.aspect_ratio.as_ref());
    }

    match provider.provider_key.as_str() {
        "fal" => {
            generate_image_with_fal(&provider.api_key, &model.model_id, prompt, &options).await
        }
        "openai-compatible" => {
            let base_url = provider
                .base_url
                .as_deref()
                .ok_or_else(|| AppError::invalid_params("openai-compatible baseUrl is required"))?;
            generate_image_with_openai_compatible(
                base_url,
                &provider.api_key,
                &model.model_id,
                prompt,
                &options,
            )
            .await
        }
        "gemini-compatible" => {
            let base_url = provider
                .base_url
                .as_deref()
                .ok_or_else(|| AppError::invalid_params("gemini-compatible baseUrl is required"))?;
            generate_image_with_gemini_compatible(
                base_url,
                &provider.api_key,
                &model.model_id,
                prompt,
                &options,
            )
            .await
        }
        _ => Err(AppError::invalid_params(format!(
            "unsupported image provider: {}",
            provider.id
        ))),
    }
}

async fn generate_video_with_fal(
    api_key: &str,
    model_id: &str,
    image_source: &str,
    options: &VideoGenerateOptions,
) -> Result<String, AppError> {
    let endpoint = fal_video_endpoint(model_id).ok_or_else(|| {
        AppError::invalid_params(format!("unsupported fal video model: {model_id}"))
    })?;
    let client = http_client();

    let image_data_url = media::normalize_source_to_data_url(image_source).await?;
    let mut payload = json!({
        "image_url": image_data_url,
        "prompt": options.prompt.clone().unwrap_or_default(),
    });

    match model_id {
        "fal-wan25" => {
            if let Some(resolution) = options.resolution.as_ref() {
                payload["resolution"] = Value::String(resolution.clone());
            }
            if let Some(duration) = options.duration {
                payload["duration"] = Value::String(duration.to_string());
            }
        }
        "fal-veo31" => {
            payload["generate_audio"] = Value::Bool(options.generate_audio.unwrap_or(false));
            if let Some(aspect_ratio) = options.aspect_ratio.as_ref() {
                payload["aspect_ratio"] = Value::String(aspect_ratio.clone());
            }
            if let Some(duration) = options.duration {
                payload["duration"] = Value::String(format!("{duration}s"));
            }
        }
        "fal-sora2" => {
            payload["delete_video"] = Value::Bool(false);
            if let Some(aspect_ratio) = options.aspect_ratio.as_ref() {
                payload["aspect_ratio"] = Value::String(aspect_ratio.clone());
            }
            if let Some(duration) = options.duration {
                payload["duration"] = Value::Number(duration.into());
            }
        }
        "fal-ai/kling-video/v2.5-turbo/pro/image-to-video" => {
            payload["negative_prompt"] =
                Value::String("blur, distort, and low quality".to_string());
            payload["cfg_scale"] = json!(0.5);
            if let Some(duration) = options.duration {
                payload["duration"] = Value::String(duration.to_string());
            }
        }
        "fal-ai/kling-video/v3/standard/image-to-video"
        | "fal-ai/kling-video/v3/pro/image-to-video" => {
            payload["start_image_url"] = payload["image_url"].clone();
            payload.as_object_mut().map(|item| item.remove("image_url"));
            payload["generate_audio"] = Value::Bool(options.generate_audio.unwrap_or(false));
            if let Some(aspect_ratio) = options.aspect_ratio.as_ref() {
                payload["aspect_ratio"] = Value::String(aspect_ratio.clone());
            }
            if let Some(duration) = options.duration {
                payload["duration"] = Value::String(duration.to_string());
            }
        }
        _ => {}
    }

    if let Some(last_frame) = options.last_frame_image_source.as_ref() {
        payload["last_frame_image_url"] =
            Value::String(media::normalize_source_to_data_url(last_frame).await?);
    }

    let request_id = submit_fal_task(&client, &endpoint, api_key, payload).await?;
    poll_fal_result(&client, &endpoint, api_key, &request_id).await
}

pub async fn generate_video(
    pool: &MySqlPool,
    model_key: &str,
    image_source: &str,
    options: VideoGenerateOptions,
) -> Result<String, AppError> {
    let (model, provider) =
        resolve_model_with_provider(pool, model_key, Some(UnifiedModelType::Video)).await?;

    match provider.provider_key.as_str() {
        "fal" => {
            generate_video_with_fal(&provider.api_key, &model.model_id, image_source, &options)
                .await
        }
        _ => Err(AppError::invalid_params(format!(
            "unsupported video provider: {}",
            provider.id
        ))),
    }
}

pub async fn generate_lip_sync(
    pool: &MySqlPool,
    model_key: &str,
    video_source: &str,
    audio_source: &str,
) -> Result<String, AppError> {
    let (model, provider) =
        resolve_model_with_provider(pool, model_key, Some(UnifiedModelType::Lipsync)).await?;

    if provider.provider_key != "fal" {
        return Err(AppError::invalid_params(format!(
            "unsupported lip sync provider: {}",
            provider.id
        )));
    }

    let client = http_client();
    let payload = json!({
        "video_url": media::normalize_source_to_data_url(video_source).await?,
        "audio_url": media::normalize_source_to_data_url(audio_source).await?,
    });

    let request_id = submit_fal_task(&client, &model.model_id, &provider.api_key, payload).await?;
    poll_fal_result(&client, &model.model_id, &provider.api_key, &request_id).await
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
