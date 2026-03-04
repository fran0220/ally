use std::{
    env,
    future::Future,
    time::{Duration, Instant},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::{Client, multipart};
use serde_json::{Value, json};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{
    api_config::UnifiedModelType,
    errors::AppError,
    media,
    runtime::{resolve_model_with_provider, resolve_provider_config},
};

const DEFAULT_GENERATOR_HTTP_TIMEOUT_SECS: u64 = 120;
const DEFAULT_GENERATOR_POLL_INTERVAL_MS: u64 = 3000;
const DEFAULT_GENERATOR_POLL_TIMEOUT_SECS: u64 = 20 * 60;
const DEFAULT_GENERATOR_RETRY_MAX_ATTEMPTS: u32 = 3;
const DEFAULT_GENERATOR_RETRY_BACKOFF_MS: u64 = 1000;

const DEFAULT_ARK_API_BASE_URL: &str = "https://ark.cn-beijing.volces.com/api/v3";
const DEFAULT_GOOGLE_API_BASE_URL: &str = "https://generativelanguage.googleapis.com";
const DEFAULT_MINIMAX_API_BASE_URL: &str = "https://api.minimaxi.com/v1";
const DEFAULT_VIDU_API_BASE_URL: &str = "https://api.vidu.cn/ent/v2";

const FAL_POLL_INTERVAL_MS: u64 = DEFAULT_GENERATOR_POLL_INTERVAL_MS;
const FAL_TIMEOUT_SECS: u64 = DEFAULT_GENERATOR_POLL_TIMEOUT_SECS;

enum PollOutcome<T> {
    Pending,
    Completed(T),
    Failed(String),
}

fn read_env_u64(key: &str, default: u64) -> u64 {
    env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn generator_http_timeout_secs() -> u64 {
    read_env_u64(
        "GENERATOR_HTTP_TIMEOUT_SECS",
        DEFAULT_GENERATOR_HTTP_TIMEOUT_SECS,
    )
}

fn generator_poll_interval_ms() -> u64 {
    read_env_u64(
        "GENERATOR_POLL_INTERVAL_MS",
        DEFAULT_GENERATOR_POLL_INTERVAL_MS,
    )
}

fn generator_poll_timeout_secs() -> u64 {
    read_env_u64(
        "GENERATOR_POLL_TIMEOUT_SECS",
        DEFAULT_GENERATOR_POLL_TIMEOUT_SECS,
    )
}

fn generator_retry_max_attempts() -> u32 {
    env::var("GENERATOR_RETRY_MAX_ATTEMPTS")
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_GENERATOR_RETRY_MAX_ATTEMPTS)
}

fn generator_retry_backoff_ms() -> u64 {
    read_env_u64(
        "GENERATOR_RETRY_BACKOFF_MS",
        DEFAULT_GENERATOR_RETRY_BACKOFF_MS,
    )
}

fn provider_base_url(configured: Option<&str>, env_key: &str, fallback: &str) -> String {
    configured
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.trim_end_matches('/').to_string())
        .or_else(|| {
            env::var(env_key)
                .ok()
                .map(|value| value.trim().trim_end_matches('/').to_string())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| fallback.to_string())
}

fn should_retry_status(status: reqwest::StatusCode) -> bool {
    status.is_server_error()
        || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status == reqwest::StatusCode::REQUEST_TIMEOUT
}

fn retry_backoff(attempt: u32) -> Duration {
    let base = generator_retry_backoff_ms();
    let power = attempt.saturating_sub(1).min(6);
    Duration::from_millis(base.saturating_mul(2u64.saturating_pow(power)))
}

async fn send_request_with_retry<F>(
    mut build_request: F,
    operation: &str,
) -> Result<reqwest::Response, AppError>
where
    F: FnMut() -> reqwest::RequestBuilder,
{
    let max_attempts = generator_retry_max_attempts();

    for attempt in 1..=max_attempts {
        match build_request().send().await {
            Ok(response) if response.status().is_success() => {
                return Ok(response);
            }
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                if should_retry_status(status) && attempt < max_attempts {
                    tokio::time::sleep(retry_backoff(attempt)).await;
                    continue;
                }

                return Err(AppError::internal(format!(
                    "{operation} failed ({status}): {body}"
                )));
            }
            Err(error) => {
                if attempt < max_attempts {
                    tokio::time::sleep(retry_backoff(attempt)).await;
                    continue;
                }

                return Err(AppError::internal(format!(
                    "{operation} request failed after {max_attempts} attempts: {error}"
                )));
            }
        }
    }

    Err(AppError::internal(format!(
        "{operation} failed after {max_attempts} attempts"
    )))
}

async fn poll_until_complete<T, F, Fut>(operation: &str, mut poll_once: F) -> Result<T, AppError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<PollOutcome<T>, AppError>>,
{
    let started_at = Instant::now();
    let timeout_secs = generator_poll_timeout_secs();
    let interval_ms = generator_poll_interval_ms();

    loop {
        if started_at.elapsed().as_secs() > timeout_secs {
            return Err(AppError::internal(format!(
                "{operation} polling timeout after {timeout_secs}s"
            )));
        }

        match poll_once().await? {
            PollOutcome::Completed(value) => return Ok(value),
            PollOutcome::Failed(message) => return Err(AppError::internal(message)),
            PollOutcome::Pending => {
                tokio::time::sleep(Duration::from_millis(interval_ms)).await;
            }
        }
    }
}

fn data_url_from_bytes(content_type: &str, bytes: &[u8]) -> String {
    format!("data:{content_type};base64,{}", STANDARD.encode(bytes))
}

fn parse_inline_image_data(data_url: &str) -> Result<(String, String), AppError> {
    media::parse_data_url(data_url)
        .ok_or_else(|| AppError::invalid_params("invalid data url for inline image payload"))
}

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
        .timeout(Duration::from_secs(generator_http_timeout_secs()))
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

fn extract_non_empty_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
}

fn normalize_google_base_url(base_url: Option<&str>) -> String {
    provider_base_url(base_url, "GOOGLE_API_BASE_URL", DEFAULT_GOOGLE_API_BASE_URL)
}

fn google_api_base(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.contains("/v1beta") || trimmed.contains("/v1") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1beta")
    }
}

fn google_model_endpoint(base_url: &str, model_id: &str, action: &str) -> String {
    format!(
        "{}/models/{}:{}",
        google_api_base(base_url),
        model_id.trim(),
        action
    )
}

fn google_resource_endpoint(base_url: &str, resource_name: &str) -> String {
    let trimmed = resource_name.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_string();
    }

    format!(
        "{}/{}",
        google_api_base(base_url),
        trimmed.trim_start_matches('/')
    )
}

fn extract_google_operation_name(value: &Value) -> Option<String> {
    extract_non_empty_string(value.get("name"))
        .or_else(|| extract_non_empty_string(value.get("operationName")))
        .or_else(|| extract_non_empty_string(value.pointer("/operation/name")))
        .or_else(|| extract_non_empty_string(value.get("id")))
}

fn extract_google_generated_video_url(value: &Value) -> Option<String> {
    extract_non_empty_string(value.pointer("/response/generatedVideos/0/video/uri"))
        .or_else(|| extract_non_empty_string(value.pointer("/response/generatedVideos/0/videoUrl")))
        .or_else(|| extract_non_empty_string(value.pointer("/generatedVideos/0/video/uri")))
        .or_else(|| extract_non_empty_string(value.pointer("/generatedVideos/0/videoUrl")))
        .or_else(|| extract_media_url(value))
}

fn normalize_ark_video_model(model_id: &str) -> (String, bool) {
    let trimmed = model_id.trim();
    if let Some(value) = trimmed.strip_suffix("-batch") {
        return (value.to_string(), true);
    }

    (trimmed.to_string(), false)
}

fn ark_seedream_size_for_aspect_ratio(aspect_ratio: &str) -> Option<&'static str> {
    match aspect_ratio {
        "1:1" => Some("4096x4096"),
        "16:9" => Some("5456x3072"),
        "9:16" => Some("3072x5456"),
        "4:3" => Some("4728x3544"),
        "3:4" => Some("3544x4728"),
        "3:2" => Some("5016x3344"),
        "2:3" => Some("3344x5016"),
        "21:9" => Some("6256x2680"),
        "9:21" => Some("2680x6256"),
        _ => None,
    }
}

fn resolve_ark_image_size(options: &ImageGenerateOptions) -> Option<String> {
    if let Some(resolution) = options
        .resolution
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if resolution.contains('x') {
            return Some(resolution.to_string());
        }

        if resolution.eq_ignore_ascii_case("4k")
            && let Some(aspect_ratio) = options
                .aspect_ratio
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            && let Some(size) = ark_seedream_size_for_aspect_ratio(aspect_ratio)
        {
            return Some(size.to_string());
        }
    }

    options
        .aspect_ratio
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(ark_seedream_size_for_aspect_ratio)
        .map(str::to_string)
}

fn normalize_minimax_model_id(model_id: &str) -> String {
    match model_id.trim().to_ascii_lowercase().as_str() {
        "minimax-hailuo-2.3" => "MiniMax-Hailuo-2.3".to_string(),
        "minimax-hailuo-2.3-fast" => "MiniMax-Hailuo-2.3-Fast".to_string(),
        "minimax-hailuo-02" => "MiniMax-Hailuo-02".to_string(),
        "t2v-01" => "T2V-01".to_string(),
        "t2v-01-director" => "T2V-01-Director".to_string(),
        _ => model_id.trim().to_string(),
    }
}

fn normalize_minimax_resolution(value: Option<&String>) -> Result<Option<String>, AppError> {
    let Some(raw) = value
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
    else {
        return Ok(None);
    };

    let normalized = raw.to_ascii_lowercase();
    if normalized.contains("512") {
        return Ok(Some("512P".to_string()));
    }
    if normalized.contains("720") {
        return Ok(Some("720P".to_string()));
    }
    if normalized.contains("768") {
        return Ok(Some("768P".to_string()));
    }
    if normalized.contains("1080") {
        return Ok(Some("1080P".to_string()));
    }

    Err(AppError::invalid_params(format!(
        "unsupported minimax resolution: {raw}"
    )))
}

fn normalize_openai_video_model(model_id: &str) -> String {
    let trimmed = model_id.trim();
    if trimmed.is_empty() {
        "sora-2".to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_openai_video_duration(duration: Option<u32>) -> Result<Option<String>, AppError> {
    match duration {
        Some(4 | 8 | 12) => Ok(duration.map(|value| value.to_string())),
        Some(value) => Err(AppError::invalid_params(format!(
            "unsupported openai-compatible video duration: {value}"
        ))),
        None => Ok(None),
    }
}

fn normalize_openai_video_aspect_ratio(
    aspect_ratio: Option<&String>,
) -> Result<Option<String>, AppError> {
    let Some(raw) = aspect_ratio
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };

    let allowed = [
        "16:9", "9:16", "4:3", "3:4", "3:2", "2:3", "21:9", "9:21", "1:1", "auto",
    ];
    if allowed.contains(&raw) {
        return Ok(Some(raw.to_string()));
    }

    Err(AppError::invalid_params(format!(
        "unsupported openai-compatible video aspect ratio: {raw}"
    )))
}

fn resolve_openai_video_size(
    resolution: Option<&String>,
    aspect_ratio: Option<&String>,
) -> Result<Option<String>, AppError> {
    let orientation = match normalize_openai_video_aspect_ratio(aspect_ratio)?
        .as_deref()
        .unwrap_or("16:9")
    {
        "9:16" | "3:4" | "2:3" | "9:21" => "portrait",
        _ => "landscape",
    };

    let Some(raw) = resolution
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };

    let normalized = match raw {
        "720x1280" | "1280x720" | "1024x1792" | "1792x1024" => Some(raw.to_string()),
        "720p" => Some(if orientation == "portrait" {
            "720x1280".to_string()
        } else {
            "1280x720".to_string()
        }),
        "1080p" => Some(if orientation == "portrait" {
            "1024x1792".to_string()
        } else {
            "1792x1024".to_string()
        }),
        _ => None,
    };

    normalized.map(Some).ok_or_else(|| {
        AppError::invalid_params(format!(
            "unsupported openai-compatible video resolution: {raw}"
        ))
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

async fn generate_image_with_ark(
    base_url: &str,
    api_key: &str,
    model_id: &str,
    prompt: &str,
    options: &ImageGenerateOptions,
) -> Result<String, AppError> {
    let client = http_client();
    let endpoint = format!("{}/images/generations", base_url.trim_end_matches('/'));

    let normalized_refs =
        media::normalize_reference_sources_to_data_urls(&options.reference_images).await?;
    let mut payload = json!({
        "model": model_id,
        "prompt": prompt,
        "sequential_image_generation": "disabled",
        "response_format": "url",
        "stream": false,
        "watermark": false,
    });
    if let Some(size) = resolve_ark_image_size(options) {
        payload["size"] = Value::String(size);
    }
    if !normalized_refs.is_empty() {
        payload["image"] = Value::Array(
            normalized_refs
                .into_iter()
                .map(Value::String)
                .collect::<Vec<_>>(),
        );
    }

    let response = send_request_with_retry(
        || client.post(&endpoint).bearer_auth(api_key).json(&payload),
        "ark image generation",
    )
    .await?;

    let value: Value = response
        .json()
        .await
        .map_err(|error| AppError::internal(format!("invalid ark image response: {error}")))?;

    if let Some(url) = extract_non_empty_string(value.pointer("/data/0/url")) {
        return Ok(url);
    }
    if let Some(b64_json) = extract_non_empty_string(value.pointer("/data/0/b64_json")) {
        return Ok(format!("data:image/png;base64,{b64_json}"));
    }

    Err(AppError::internal(
        "ark image response missing image payload",
    ))
}

fn extract_google_batch_image_data(value: &Value) -> Option<String> {
    let candidates = value
        .pointer("/dest/inlinedResponses/0/response/candidates")
        .and_then(Value::as_array)
        .or_else(|| {
            value
                .pointer("/response/candidates")
                .and_then(Value::as_array)
        })?;

    let first_candidate = candidates.first()?;
    let parts = first_candidate
        .pointer("/content/parts")
        .and_then(Value::as_array)?;

    for part in parts {
        if let Some(data) = extract_non_empty_string(part.pointer("/inlineData/data")) {
            let mime_type = extract_non_empty_string(part.pointer("/inlineData/mimeType"))
                .unwrap_or_else(|| "image/png".to_string());
            return Some(format!("data:{mime_type};base64,{data}"));
        }
    }

    None
}

async fn generate_image_with_google_imagen(
    base_url: &str,
    api_key: &str,
    model_id: &str,
    prompt: &str,
    options: &ImageGenerateOptions,
) -> Result<String, AppError> {
    let client = http_client();
    let endpoint = google_model_endpoint(base_url, model_id, "generateImages");

    let mut config = json!({
        "numberOfImages": 1,
    });
    if let Some(aspect_ratio) = options.aspect_ratio.as_ref() {
        config["aspectRatio"] = Value::String(aspect_ratio.clone());
    }

    let payload = json!({
        "prompt": prompt,
        "config": config,
    });

    let response = send_request_with_retry(
        || {
            client
                .post(&endpoint)
                .header("x-goog-api-key", api_key)
                .bearer_auth(api_key)
                .json(&payload)
        },
        "google imagen request",
    )
    .await?;

    let value: Value = response
        .json()
        .await
        .map_err(|error| AppError::internal(format!("invalid imagen response: {error}")))?;

    if let Some(image_bytes) =
        extract_non_empty_string(value.pointer("/generatedImages/0/image/imageBytes"))
    {
        return Ok(format!("data:image/png;base64,{image_bytes}"));
    }

    Err(AppError::internal(
        "imagen response missing generated image bytes",
    ))
}

async fn generate_image_with_google_batch(
    base_url: &str,
    api_key: &str,
    model_id: &str,
    prompt: &str,
    options: &ImageGenerateOptions,
) -> Result<String, AppError> {
    let client = http_client();
    let endpoint = format!("{}/batches", google_api_base(base_url));
    let batch_model_id = model_id.trim().trim_end_matches("-batch");
    let batch_model_id = if batch_model_id.is_empty() {
        model_id.trim()
    } else {
        batch_model_id
    };

    let normalized_refs =
        media::normalize_reference_sources_to_data_urls(&options.reference_images).await?;
    let mut parts = Vec::with_capacity(normalized_refs.len() + 1);
    for item in normalized_refs {
        let (mime_type, data) = parse_inline_image_data(&item)?;
        parts.push(json!({
            "inlineData": {
                "mimeType": mime_type,
                "data": data,
            }
        }));
    }
    parts.push(json!({ "text": prompt }));

    let mut request_config = json!({
        "responseModalities": ["TEXT", "IMAGE"],
    });
    if options.aspect_ratio.is_some() || options.resolution.is_some() {
        request_config["imageConfig"] = json!({
            "aspectRatio": options.aspect_ratio,
            "imageSize": options.resolution,
        });
    }

    let inlined_requests = vec![json!({
        "contents": [{ "parts": parts }],
        "config": request_config,
    })];
    let display_name = format!("rust-image-{}", Uuid::new_v4());

    let payload_variants = vec![
        json!({
            "model": batch_model_id,
            "src": inlined_requests.clone(),
            "config": {
                "displayName": display_name.clone(),
            }
        }),
        json!({
            "model": format!("models/{batch_model_id}"),
            "src": {
                "inlinedRequests": inlined_requests,
            },
            "config": {
                "displayName": display_name,
            }
        }),
    ];

    let mut batch_name: Option<String> = None;
    let mut submit_error: Option<AppError> = None;

    for payload in payload_variants {
        match send_request_with_retry(
            || {
                client
                    .post(&endpoint)
                    .header("x-goog-api-key", api_key)
                    .bearer_auth(api_key)
                    .json(&payload)
            },
            "google batch submit",
        )
        .await
        {
            Ok(response) => {
                let value: Value = response.json().await.map_err(|error| {
                    AppError::internal(format!("invalid google batch submit response: {error}"))
                })?;
                if let Some(name) = extract_non_empty_string(value.get("name")) {
                    batch_name = Some(name);
                    break;
                }
                submit_error = Some(AppError::internal("google batch response missing name"));
            }
            Err(error) => {
                submit_error = Some(error);
            }
        }
    }

    let batch_name = batch_name.ok_or_else(|| {
        submit_error
            .unwrap_or_else(|| AppError::internal("google batch submit failed unexpectedly"))
    })?;

    poll_until_complete("google batch image", || async {
        let status_endpoint = google_resource_endpoint(base_url, &batch_name);
        let response = send_request_with_retry(
            || {
                client
                    .get(&status_endpoint)
                    .header("x-goog-api-key", api_key)
                    .bearer_auth(api_key)
            },
            "google batch status",
        )
        .await?;

        let value: Value = response.json().await.map_err(|error| {
            AppError::internal(format!("invalid google batch status response: {error}"))
        })?;
        let state = extract_non_empty_string(value.get("state"))
            .unwrap_or_else(|| "JOB_STATE_PENDING".to_string())
            .to_ascii_uppercase();

        if state == "JOB_STATE_SUCCEEDED" || state == "SUCCEEDED" {
            if let Some(image_url) = extract_google_batch_image_data(&value) {
                return Ok(PollOutcome::Completed(image_url));
            }

            return Ok(PollOutcome::Failed(
                "google batch task completed but missing image payload".to_string(),
            ));
        }

        if [
            "JOB_STATE_FAILED",
            "JOB_STATE_CANCELLED",
            "JOB_STATE_EXPIRED",
            "FAILED",
        ]
        .contains(&state.as_str())
        {
            let error = extract_non_empty_string(value.pointer("/error/message"))
                .or_else(|| extract_non_empty_string(value.get("error")))
                .unwrap_or_else(|| format!("google batch task failed: {state}"));
            return Ok(PollOutcome::Failed(error));
        }

        Ok(PollOutcome::Pending)
    })
    .await
}

async fn generate_video_with_ark(
    base_url: &str,
    api_key: &str,
    model_id: &str,
    image_source: &str,
    options: &VideoGenerateOptions,
) -> Result<String, AppError> {
    let client = http_client();
    let endpoint = format!(
        "{}/contents/generations/tasks",
        base_url.trim_end_matches('/')
    );
    let (real_model, batch_mode) = normalize_ark_video_model(model_id);

    let first_frame = media::normalize_source_to_data_url(image_source).await?;
    let mut content = Vec::new();
    if let Some(prompt) = options.prompt.as_ref().map(|item| item.trim())
        && !prompt.is_empty()
    {
        content.push(json!({
            "type": "text",
            "text": prompt,
        }));
    }

    if let Some(last_frame) = options.last_frame_image_source.as_ref() {
        content.push(json!({
            "type": "image_url",
            "image_url": {
                "url": first_frame,
            },
            "role": "first_frame",
        }));
        content.push(json!({
            "type": "image_url",
            "image_url": {
                "url": media::normalize_source_to_data_url(last_frame).await?,
            },
            "role": "last_frame",
        }));
    } else {
        content.push(json!({
            "type": "image_url",
            "image_url": {
                "url": first_frame,
            }
        }));
    }

    let mut payload = json!({
        "model": real_model,
        "content": content,
    });
    if let Some(resolution) = options.resolution.as_ref() {
        payload["resolution"] = Value::String(resolution.clone());
    }
    if let Some(aspect_ratio) = options.aspect_ratio.as_ref() {
        payload["ratio"] = Value::String(aspect_ratio.clone());
    }
    if let Some(duration) = options.duration {
        payload["duration"] = Value::Number(duration.into());
    }
    if let Some(generate_audio) = options.generate_audio {
        payload["generate_audio"] = Value::Bool(generate_audio);
    }
    if batch_mode {
        payload["service_tier"] = Value::String("flex".to_string());
        payload["execution_expires_after"] = Value::Number(86_400.into());
    }

    let response = send_request_with_retry(
        || client.post(&endpoint).bearer_auth(api_key).json(&payload),
        "ark video submit",
    )
    .await?;

    let value: Value = response.json().await.map_err(|error| {
        AppError::internal(format!("invalid ark video submit response: {error}"))
    })?;
    let task_id = extract_non_empty_string(value.get("id"))
        .or_else(|| extract_non_empty_string(value.get("task_id")))
        .ok_or_else(|| AppError::internal("ark video submit response missing task id"))?;

    poll_until_complete("ark video", || async {
        let status_endpoint = format!(
            "{}/contents/generations/tasks/{task_id}",
            base_url.trim_end_matches('/')
        );
        let response = send_request_with_retry(
            || client.get(&status_endpoint).bearer_auth(api_key),
            "ark video status",
        )
        .await?;

        let value: Value = response.json().await.map_err(|error| {
            AppError::internal(format!("invalid ark video status response: {error}"))
        })?;
        let status = extract_non_empty_string(value.get("status"))
            .unwrap_or_else(|| "processing".to_string())
            .to_ascii_lowercase();

        if status == "succeeded" || status == "completed" {
            if let Some(video_url) = extract_non_empty_string(value.pointer("/content/video_url"))
                .or_else(|| extract_non_empty_string(value.pointer("/content/video/url")))
                .or_else(|| extract_media_url(&value))
            {
                return Ok(PollOutcome::Completed(video_url));
            }

            return Ok(PollOutcome::Failed(
                "ark video task completed but missing video url".to_string(),
            ));
        }

        if status == "failed" {
            let message = extract_non_empty_string(value.pointer("/error/message"))
                .or_else(|| extract_non_empty_string(value.get("error")))
                .unwrap_or_else(|| "ark video generation failed".to_string());
            return Ok(PollOutcome::Failed(message));
        }

        Ok(PollOutcome::Pending)
    })
    .await
}

async fn generate_video_with_google(
    base_url: &str,
    api_key: &str,
    model_id: &str,
    image_source: &str,
    options: &VideoGenerateOptions,
) -> Result<String, AppError> {
    let client = http_client();
    let endpoint = google_model_endpoint(base_url, model_id, "generateVideos");

    let first_frame_data_url = media::normalize_source_to_data_url(image_source).await?;
    let (first_frame_mime, first_frame_data) = parse_inline_image_data(&first_frame_data_url)?;

    let mut payload = json!({
        "prompt": options.prompt.clone().unwrap_or_default(),
        "image": {
            "mimeType": first_frame_mime,
            "imageBytes": first_frame_data,
        },
    });
    let mut config = serde_json::Map::new();
    if let Some(aspect_ratio) = options.aspect_ratio.as_ref() {
        config.insert(
            "aspectRatio".to_string(),
            Value::String(aspect_ratio.clone()),
        );
    }
    if let Some(resolution) = options.resolution.as_ref() {
        config.insert("resolution".to_string(), Value::String(resolution.clone()));
    }
    if let Some(duration) = options.duration {
        config.insert(
            "durationSeconds".to_string(),
            Value::Number(duration.into()),
        );
    }
    if let Some(last_frame_source) = options.last_frame_image_source.as_ref() {
        let last_frame_data_url = media::normalize_source_to_data_url(last_frame_source).await?;
        let (last_frame_mime, last_frame_data) = parse_inline_image_data(&last_frame_data_url)?;
        config.insert(
            "lastFrame".to_string(),
            json!({
                "mimeType": last_frame_mime,
                "imageBytes": last_frame_data,
            }),
        );
    }
    if !config.is_empty() {
        payload["config"] = Value::Object(config);
    }

    let response = send_request_with_retry(
        || {
            client
                .post(&endpoint)
                .header("x-goog-api-key", api_key)
                .bearer_auth(api_key)
                .json(&payload)
        },
        "google video submit",
    )
    .await?;

    let value: Value = response.json().await.map_err(|error| {
        AppError::internal(format!("invalid google video submit response: {error}"))
    })?;
    let operation_name = extract_google_operation_name(&value)
        .ok_or_else(|| AppError::internal("google video submit response missing operation name"))?;

    poll_until_complete("google video", || async {
        let operation_endpoint = google_resource_endpoint(base_url, &operation_name);
        let response = send_request_with_retry(
            || {
                client
                    .get(&operation_endpoint)
                    .header("x-goog-api-key", api_key)
                    .bearer_auth(api_key)
            },
            "google video status",
        )
        .await?;

        let value: Value = response.json().await.map_err(|error| {
            AppError::internal(format!("invalid google video status response: {error}"))
        })?;

        let done = value.get("done").and_then(Value::as_bool).unwrap_or(false);
        if !done {
            return Ok(PollOutcome::Pending);
        }

        if value.get("error").is_some() {
            let message = extract_non_empty_string(value.pointer("/error/message"))
                .or_else(|| extract_non_empty_string(value.get("error")))
                .unwrap_or_else(|| "google video generation failed".to_string());
            return Ok(PollOutcome::Failed(message));
        }

        if let Some(filtered_count) = value
            .pointer("/response/raiMediaFilteredCount")
            .and_then(Value::as_i64)
            && filtered_count > 0
        {
            return Ok(PollOutcome::Failed(format!(
                "google video was filtered by RAI policy ({filtered_count})"
            )));
        }

        if let Some(video_url) = extract_google_generated_video_url(&value) {
            return Ok(PollOutcome::Completed(video_url));
        }

        Ok(PollOutcome::Failed(
            "google video task completed but missing video uri".to_string(),
        ))
    })
    .await
}

async fn generate_video_with_minimax(
    base_url: &str,
    api_key: &str,
    model_id: &str,
    image_source: &str,
    options: &VideoGenerateOptions,
) -> Result<String, AppError> {
    let client = http_client();
    let submit_endpoint = format!("{}/video_generation", base_url.trim_end_matches('/'));
    let query_base = base_url.trim_end_matches('/').to_string();

    if options.generate_audio == Some(true) {
        return Err(AppError::invalid_params(
            "minimax video does not support generate_audio=true",
        ));
    }

    let mut payload = json!({
        "model": normalize_minimax_model_id(model_id),
        "prompt": options.prompt.clone().unwrap_or_default(),
        "prompt_optimizer": true,
    });
    if let Some(duration) = options.duration {
        payload["duration"] = Value::Number(duration.into());
    }
    if let Some(resolution) = normalize_minimax_resolution(options.resolution.as_ref())? {
        payload["resolution"] = Value::String(resolution);
    }
    if !image_source.trim().is_empty() {
        payload["first_frame_image"] =
            Value::String(media::normalize_source_to_data_url(image_source).await?);
    }
    if let Some(last_frame) = options.last_frame_image_source.as_ref() {
        payload["last_frame_image"] =
            Value::String(media::normalize_source_to_data_url(last_frame).await?);
    }

    let response = send_request_with_retry(
        || {
            client
                .post(&submit_endpoint)
                .bearer_auth(api_key)
                .json(&payload)
        },
        "minimax video submit",
    )
    .await?;
    let value: Value = response
        .json()
        .await
        .map_err(|error| AppError::internal(format!("invalid minimax submit response: {error}")))?;

    if value
        .pointer("/base_resp/status_code")
        .and_then(Value::as_i64)
        != Some(0)
    {
        let error = extract_non_empty_string(value.pointer("/base_resp/status_msg"))
            .unwrap_or_else(|| "minimax submit failed".to_string());
        return Err(AppError::internal(error));
    }

    let task_id = extract_non_empty_string(value.get("task_id"))
        .ok_or_else(|| AppError::internal("minimax submit response missing task_id"))?;

    poll_until_complete("minimax video", || async {
        let query_endpoint = format!("{query_base}/query/video_generation?task_id={task_id}");
        let query_response = send_request_with_retry(
            || client.get(&query_endpoint).bearer_auth(api_key),
            "minimax video status",
        )
        .await?;
        let query_value: Value = query_response.json().await.map_err(|error| {
            AppError::internal(format!("invalid minimax status response: {error}"))
        })?;

        if query_value
            .pointer("/base_resp/status_code")
            .and_then(Value::as_i64)
            != Some(0)
        {
            let error = extract_non_empty_string(query_value.pointer("/base_resp/status_msg"))
                .unwrap_or_else(|| "minimax status query failed".to_string());
            return Ok(PollOutcome::Failed(error));
        }

        let status = extract_non_empty_string(query_value.get("status"))
            .unwrap_or_else(|| "Processing".to_string());
        if status == "Success" {
            let file_id = extract_non_empty_string(query_value.get("file_id"))
                .ok_or_else(|| AppError::internal("minimax success response missing file_id"))?;
            let file_endpoint = format!("{query_base}/files/retrieve?file_id={file_id}");
            let file_response = send_request_with_retry(
                || client.get(&file_endpoint).bearer_auth(api_key),
                "minimax file retrieve",
            )
            .await?;
            let file_value: Value = file_response.json().await.map_err(|error| {
                AppError::internal(format!("invalid minimax file retrieve response: {error}"))
            })?;

            if let Some(download_url) =
                extract_non_empty_string(file_value.pointer("/file/download_url"))
            {
                return Ok(PollOutcome::Completed(download_url));
            }

            return Ok(PollOutcome::Failed(
                "minimax file retrieve response missing download_url".to_string(),
            ));
        }

        if status == "Failed" {
            let error = extract_non_empty_string(query_value.get("error_message"))
                .or_else(|| extract_non_empty_string(query_value.pointer("/base_resp/status_msg")))
                .unwrap_or_else(|| "minimax video generation failed".to_string());
            return Ok(PollOutcome::Failed(error));
        }

        Ok(PollOutcome::Pending)
    })
    .await
}

async fn poll_vidu_task(
    client: &Client,
    base_url: &str,
    api_key: &str,
    task_id: &str,
    operation: &str,
) -> Result<String, AppError> {
    let status_base = base_url.trim_end_matches('/').to_string();
    poll_until_complete(operation, || async {
        let status_endpoint = format!("{status_base}/tasks/{task_id}/creations");
        let response = send_request_with_retry(
            || {
                client
                    .get(&status_endpoint)
                    .header("Authorization", format!("Token {api_key}"))
            },
            "vidu task status",
        )
        .await?;
        let value: Value = response.json().await.map_err(|error| {
            AppError::internal(format!("invalid vidu status response: {error}"))
        })?;

        let state = extract_non_empty_string(value.get("state"))
            .unwrap_or_else(|| "processing".to_string())
            .to_ascii_lowercase();

        if state == "success" || state == "completed" {
            if let Some(video_url) = extract_non_empty_string(value.pointer("/creations/0/url"))
                .or_else(|| extract_non_empty_string(value.pointer("/creations/0/video_url")))
            {
                return Ok(PollOutcome::Completed(video_url));
            }

            return Ok(PollOutcome::Failed(
                "vidu task completed but missing media url".to_string(),
            ));
        }

        if state == "failed" {
            let message = extract_non_empty_string(value.get("err_code"))
                .or_else(|| extract_non_empty_string(value.get("error")))
                .unwrap_or_else(|| "vidu generation failed".to_string());
            return Ok(PollOutcome::Failed(format!("Vidu: {message}")));
        }

        Ok(PollOutcome::Pending)
    })
    .await
}

async fn generate_video_with_vidu(
    base_url: &str,
    api_key: &str,
    model_id: &str,
    image_source: &str,
    options: &VideoGenerateOptions,
) -> Result<String, AppError> {
    let client = http_client();
    let mode = options
        .generation_mode
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            if options.last_frame_image_source.is_some() {
                "firstlastframe"
            } else {
                "normal"
            }
        });

    let endpoint_suffix = if mode == "firstlastframe" {
        "start-end2video"
    } else {
        "img2video"
    };
    let endpoint = format!("{}/{}", base_url.trim_end_matches('/'), endpoint_suffix);

    let first_frame_data_url = media::normalize_source_to_data_url(image_source).await?;
    let mut images = vec![first_frame_data_url];
    if mode == "firstlastframe" {
        let last_frame_source = options.last_frame_image_source.as_ref().ok_or_else(|| {
            AppError::invalid_params("vidu firstlastframe mode requires last_frame_image_source")
        })?;
        images.push(media::normalize_source_to_data_url(last_frame_source).await?);
    }

    let mut payload = json!({
        "model": model_id,
        "images": images,
        "duration": options.duration.unwrap_or(5),
        "resolution": options
            .resolution
            .clone()
            .unwrap_or_else(|| "720p".to_string()),
    });
    if let Some(prompt) = options.prompt.as_ref().map(|item| item.trim())
        && !prompt.is_empty()
    {
        payload["prompt"] = Value::String(prompt.to_string());
    }
    if let Some(aspect_ratio) = options.aspect_ratio.as_ref() {
        payload["aspect_ratio"] = Value::String(aspect_ratio.clone());
    }
    if let Some(generate_audio) = options.generate_audio {
        payload["audio"] = Value::Bool(generate_audio);
    }

    let response = send_request_with_retry(
        || {
            client
                .post(&endpoint)
                .header("Authorization", format!("Token {api_key}"))
                .json(&payload)
        },
        "vidu video submit",
    )
    .await?;
    let value: Value = response
        .json()
        .await
        .map_err(|error| AppError::internal(format!("invalid vidu submit response: {error}")))?;

    if extract_non_empty_string(value.get("state"))
        .map(|value| value.eq_ignore_ascii_case("failed"))
        .unwrap_or(false)
    {
        let message = extract_non_empty_string(value.get("err_code"))
            .or_else(|| extract_non_empty_string(value.get("error")))
            .unwrap_or_else(|| "vidu submit failed".to_string());
        return Err(AppError::internal(format!("Vidu: {message}")));
    }

    let task_id = extract_non_empty_string(value.get("task_id"))
        .ok_or_else(|| AppError::internal("vidu submit response missing task_id"))?;

    poll_vidu_task(&client, base_url, api_key, &task_id, "vidu video").await
}

fn should_fallback_to_openai_video_create(status: reqwest::StatusCode, body: &str) -> bool {
    status == reqwest::StatusCode::NOT_FOUND
        || status == reqwest::StatusCode::METHOD_NOT_ALLOWED
        || body.to_ascii_lowercase().contains("get_channel_failed")
}

struct OpenAiVideoCreateRequest<'a> {
    base_url: &'a str,
    api_key: &'a str,
    model: &'a str,
    prompt: &'a str,
    duration: Option<&'a String>,
    size: Option<&'a String>,
    image_source: &'a str,
}

async fn create_openai_video_task_with_official_api(
    client: &Client,
    request: &OpenAiVideoCreateRequest<'_>,
) -> Result<Option<String>, AppError> {
    let endpoint = format!("{}/videos", request.base_url.trim_end_matches('/'));
    let max_attempts = generator_retry_max_attempts();

    for attempt in 1..=max_attempts {
        let mut form = multipart::Form::new()
            .text("model", request.model.to_string())
            .text("prompt", request.prompt.to_string());
        if let Some(duration) = request.duration {
            form = form.text("seconds", duration.clone());
        }
        if let Some(size) = request.size {
            form = form.text("size", size.clone());
        }
        if !request.image_source.trim().is_empty() {
            let (bytes, content_type) = media::download_source_bytes(request.image_source).await?;
            let part = multipart::Part::bytes(bytes)
                .file_name("input-reference.png")
                .mime_str(&content_type)
                .map_err(|error| {
                    AppError::invalid_params(format!(
                        "invalid openai video input mime type: {error}"
                    ))
                })?;
            form = form.part("input_reference", part);
        }

        match client
            .post(&endpoint)
            .bearer_auth(request.api_key)
            .multipart(form)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                let value: Value = response.json().await.map_err(|error| {
                    AppError::internal(format!("invalid openai video create response: {error}"))
                })?;
                let video_id = extract_non_empty_string(value.get("id")).ok_or_else(|| {
                    AppError::internal("openai-compatible video create response missing id")
                })?;
                return Ok(Some(video_id));
            }
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                if should_fallback_to_openai_video_create(status, &body) {
                    return Ok(None);
                }
                if should_retry_status(status) && attempt < max_attempts {
                    tokio::time::sleep(retry_backoff(attempt)).await;
                    continue;
                }

                return Err(AppError::internal(format!(
                    "openai-compatible video create failed ({status}): {body}"
                )));
            }
            Err(error) => {
                if attempt < max_attempts {
                    tokio::time::sleep(retry_backoff(attempt)).await;
                    continue;
                }

                return Err(AppError::internal(format!(
                    "openai-compatible video create request failed: {error}"
                )));
            }
        }
    }

    Ok(None)
}

async fn create_openai_video_task_with_fallback_api(
    client: &Client,
    request: &OpenAiVideoCreateRequest<'_>,
) -> Result<String, AppError> {
    let endpoint = format!("{}/video/create", request.base_url.trim_end_matches('/'));
    let mut payload = json!({
        "model": request.model,
        "prompt": request.prompt,
    });
    if let Some(duration) = request.duration {
        payload["seconds"] = Value::String(duration.clone());
    }
    if let Some(size) = request.size {
        payload["size"] = Value::String(size.clone());
    }
    if !request.image_source.trim().is_empty() {
        payload["image_url"] = Value::String(request.image_source.to_string());
    }

    let response = send_request_with_retry(
        || {
            client
                .post(&endpoint)
                .bearer_auth(request.api_key)
                .json(&payload)
        },
        "openai-compatible video fallback submit",
    )
    .await?;

    let value: Value = response
        .json()
        .await
        .map_err(|error| AppError::internal(format!("invalid fallback video response: {error}")))?;
    extract_non_empty_string(value.get("id"))
        .ok_or_else(|| AppError::internal("openai-compatible fallback response missing id"))
}

async fn generate_video_with_openai_compatible(
    base_url: &str,
    api_key: &str,
    model_id: &str,
    image_source: &str,
    options: &VideoGenerateOptions,
) -> Result<String, AppError> {
    let client = http_client();

    let prompt = options
        .prompt
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::invalid_params("openai-compatible video prompt is required"))?;
    let model = normalize_openai_video_model(model_id);
    let duration = normalize_openai_video_duration(options.duration)?;
    let size =
        resolve_openai_video_size(options.resolution.as_ref(), options.aspect_ratio.as_ref())?;

    let create_request = OpenAiVideoCreateRequest {
        base_url,
        api_key,
        model: &model,
        prompt,
        duration: duration.as_ref(),
        size: size.as_ref(),
        image_source,
    };

    let video_id = if let Some(video_id) =
        create_openai_video_task_with_official_api(&client, &create_request).await?
    {
        video_id
    } else {
        create_openai_video_task_with_fallback_api(&client, &create_request).await?
    };

    poll_until_complete("openai-compatible video", || async {
        let status_endpoint = format!("{}/videos/{video_id}", base_url.trim_end_matches('/'));
        let response = send_request_with_retry(
            || client.get(&status_endpoint).bearer_auth(api_key),
            "openai-compatible video status",
        )
        .await?;
        let value: Value = response.json().await.map_err(|error| {
            AppError::internal(format!(
                "invalid openai-compatible video status response: {error}"
            ))
        })?;

        let status = extract_non_empty_string(value.get("status"))
            .unwrap_or_else(|| "queued".to_string())
            .to_ascii_lowercase();

        if ["queued", "in_progress", "processing"].contains(&status.as_str()) {
            return Ok(PollOutcome::Pending);
        }

        if status == "failed" {
            let message = extract_non_empty_string(value.pointer("/error/message"))
                .or_else(|| extract_non_empty_string(value.get("error")))
                .unwrap_or_else(|| "openai-compatible video generation failed".to_string());
            return Ok(PollOutcome::Failed(message));
        }

        if status == "completed" {
            if let Some(video_url) = extract_non_empty_string(value.get("video_url"))
                .or_else(|| extract_non_empty_string(value.get("videoUrl")))
            {
                return Ok(PollOutcome::Completed(video_url));
            }

            let content_endpoint = format!(
                "{}/videos/{video_id}/content",
                base_url.trim_end_matches('/')
            );
            let content_response = send_request_with_retry(
                || client.get(&content_endpoint).bearer_auth(api_key),
                "openai-compatible video content",
            )
            .await?;

            let content_type = content_response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(|value| value.split(';').next().unwrap_or(value).trim().to_string())
                .unwrap_or_else(|| "video/mp4".to_string());
            let bytes = content_response.bytes().await.map_err(|error| {
                AppError::internal(format!(
                    "failed to read openai-compatible video bytes: {error}"
                ))
            })?;
            return Ok(PollOutcome::Completed(data_url_from_bytes(
                &content_type,
                bytes.as_ref(),
            )));
        }

        Ok(PollOutcome::Pending)
    })
    .await
}

async fn generate_lip_sync_with_vidu(
    base_url: &str,
    api_key: &str,
    video_source: &str,
    audio_source: &str,
) -> Result<String, AppError> {
    let client = http_client();
    let endpoint = format!("{}/lip-sync", base_url.trim_end_matches('/'));
    let payload = json!({
        "video_url": video_source,
        "audio_url": audio_source,
    });

    let response = send_request_with_retry(
        || {
            client
                .post(&endpoint)
                .header("Authorization", format!("Token {api_key}"))
                .json(&payload)
        },
        "vidu lip-sync submit",
    )
    .await?;
    let value: Value = response
        .json()
        .await
        .map_err(|error| AppError::internal(format!("invalid vidu lip-sync response: {error}")))?;

    if extract_non_empty_string(value.get("state"))
        .map(|item| item.eq_ignore_ascii_case("failed"))
        .unwrap_or(false)
    {
        let message = extract_non_empty_string(value.get("err_code"))
            .or_else(|| extract_non_empty_string(value.get("error")))
            .unwrap_or_else(|| "vidu lip-sync submit failed".to_string());
        return Err(AppError::internal(format!("Vidu: {message}")));
    }

    let task_id = extract_non_empty_string(value.get("task_id"))
        .ok_or_else(|| AppError::internal("vidu lip-sync response missing task_id"))?;
    poll_vidu_task(&client, base_url, api_key, &task_id, "vidu lip-sync").await
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
        "ark" => {
            let base_url = provider_base_url(
                provider.base_url.as_deref(),
                "ARK_API_BASE_URL",
                DEFAULT_ARK_API_BASE_URL,
            );
            generate_image_with_ark(
                &base_url,
                &provider.api_key,
                &model.model_id,
                prompt,
                &options,
            )
            .await
        }
        "google" | "google-batch" | "imagen" => {
            let base_url = normalize_google_base_url(provider.base_url.as_deref());
            if model.model_id.trim().starts_with("imagen-") || provider.provider_key == "imagen" {
                generate_image_with_google_imagen(
                    &base_url,
                    &provider.api_key,
                    &model.model_id,
                    prompt,
                    &options,
                )
                .await
            } else if model.model_id.trim().ends_with("-batch")
                || model.model_id.trim() == "gemini-3-pro-image-preview-batch"
                || provider.provider_key == "google-batch"
            {
                generate_image_with_google_batch(
                    &base_url,
                    &provider.api_key,
                    &model.model_id,
                    prompt,
                    &options,
                )
                .await
            } else {
                generate_image_with_gemini_compatible(
                    &base_url,
                    &provider.api_key,
                    &model.model_id,
                    prompt,
                    &options,
                )
                .await
            }
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
        "ark" => {
            let base_url = provider_base_url(
                provider.base_url.as_deref(),
                "ARK_API_BASE_URL",
                DEFAULT_ARK_API_BASE_URL,
            );
            generate_video_with_ark(
                &base_url,
                &provider.api_key,
                &model.model_id,
                image_source,
                &options,
            )
            .await
        }
        "google" | "gemini-compatible" => {
            let base_url = normalize_google_base_url(provider.base_url.as_deref());
            generate_video_with_google(
                &base_url,
                &provider.api_key,
                &model.model_id,
                image_source,
                &options,
            )
            .await
        }
        "minimax" => {
            let base_url = provider_base_url(
                provider.base_url.as_deref(),
                "MINIMAX_API_BASE_URL",
                DEFAULT_MINIMAX_API_BASE_URL,
            );
            generate_video_with_minimax(
                &base_url,
                &provider.api_key,
                &model.model_id,
                image_source,
                &options,
            )
            .await
        }
        "vidu" => {
            let base_url = provider_base_url(
                provider.base_url.as_deref(),
                "VIDU_API_BASE_URL",
                DEFAULT_VIDU_API_BASE_URL,
            );
            generate_video_with_vidu(
                &base_url,
                &provider.api_key,
                &model.model_id,
                image_source,
                &options,
            )
            .await
        }
        "openai-compatible" => {
            let base_url = provider
                .base_url
                .as_deref()
                .ok_or_else(|| AppError::invalid_params("openai-compatible baseUrl is required"))?;
            generate_video_with_openai_compatible(
                base_url,
                &provider.api_key,
                &model.model_id,
                image_source,
                &options,
            )
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
        "vidu" => {
            let base_url = provider_base_url(
                provider.base_url.as_deref(),
                "VIDU_API_BASE_URL",
                DEFAULT_VIDU_API_BASE_URL,
            );
            generate_lip_sync_with_vidu(&base_url, &provider.api_key, video_source, audio_source)
                .await
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
