use std::{
    env,
    future::Future,
    time::{Duration, Instant},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::Client;
use serde_json::Value;

use crate::{errors::AppError, media};

mod fal;
mod image;
mod video;
mod voice;

pub use image::generate_image;
pub use video::generate_video;
pub use voice::{create_voice_design, generate_lip_sync, generate_voice_clone};

const DEFAULT_GENERATOR_HTTP_TIMEOUT_SECS: u64 = 120;
const DEFAULT_GENERATOR_POLL_INTERVAL_MS: u64 = 3000;
const DEFAULT_GENERATOR_POLL_TIMEOUT_SECS: u64 = 20 * 60;
const DEFAULT_GENERATOR_RETRY_MAX_ATTEMPTS: u32 = 3;
const DEFAULT_GENERATOR_RETRY_BACKOFF_MS: u64 = 1000;

pub(super) const FAL_POLL_INTERVAL_MS: u64 = DEFAULT_GENERATOR_POLL_INTERVAL_MS;
pub(super) const FAL_TIMEOUT_SECS: u64 = DEFAULT_GENERATOR_POLL_TIMEOUT_SECS;

pub(super) enum PollOutcome<T> {
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

pub(super) fn generator_retry_max_attempts() -> u32 {
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

#[allow(dead_code)]
pub(super) fn provider_base_url(configured: Option<&str>, env_key: &str, fallback: &str) -> String {
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

pub(super) fn should_retry_status(status: reqwest::StatusCode) -> bool {
    status.is_server_error()
        || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status == reqwest::StatusCode::REQUEST_TIMEOUT
}

pub(super) fn retry_backoff(attempt: u32) -> Duration {
    let base = generator_retry_backoff_ms();
    let power = attempt.saturating_sub(1).min(6);
    Duration::from_millis(base.saturating_mul(2u64.saturating_pow(power)))
}

pub(super) async fn send_request_with_retry<F>(
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

pub(super) async fn poll_until_complete<T, F, Fut>(
    operation: &str,
    mut poll_once: F,
) -> Result<T, AppError>
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

pub(super) fn data_url_from_bytes(content_type: &str, bytes: &[u8]) -> String {
    format!("data:{content_type};base64,{}", STANDARD.encode(bytes))
}

pub(super) fn parse_inline_image_data(data_url: &str) -> Result<(String, String), AppError> {
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

pub(super) fn http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(generator_http_timeout_secs()))
        .build()
        .unwrap_or_else(|_| Client::new())
}

pub(super) fn extract_media_url(value: &Value) -> Option<String> {
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

pub(super) fn extract_non_empty_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
}

pub(super) fn parse_resolution_size(
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
