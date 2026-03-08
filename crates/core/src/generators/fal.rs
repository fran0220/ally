use std::time::{Duration, Instant};

use reqwest::Client;
use serde_json::Value;

use crate::errors::AppError;

use super::{FAL_POLL_INTERVAL_MS, FAL_TIMEOUT_SECS, extract_media_url};

pub(super) fn parse_fal_base_endpoint(endpoint: &str) -> String {
    let segments = endpoint.split('/').collect::<Vec<_>>();
    if segments.len() >= 2 {
        format!("{}/{}", segments[0], segments[1])
    } else {
        endpoint.to_string()
    }
}

pub(super) async fn submit_fal_task(
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

pub(super) async fn poll_fal_result(
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

pub(super) fn fal_image_endpoint(model_id: &str, is_edit: bool) -> Option<String> {
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

pub(super) fn fal_video_endpoint(model_id: &str) -> Option<String> {
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
