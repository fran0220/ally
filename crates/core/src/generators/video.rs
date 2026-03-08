use reqwest::multipart;
use serde_json::{Value, json};
use sqlx::MySqlPool;

use crate::{
    api_config::UnifiedModelType, errors::AppError, media, runtime::resolve_model_with_provider,
};

use super::{
    PollOutcome, VideoGenerateOptions, data_url_from_bytes, extract_media_url,
    extract_non_empty_string, fal::fal_video_endpoint, fal::poll_fal_result, fal::submit_fal_task,
    generator_retry_max_attempts, http_client, poll_until_complete, provider_base_url,
    retry_backoff, send_request_with_retry, should_retry_status,
};

const DEFAULT_JIMENG_BASE_URL: &str = "http://185.200.65.233:5100";

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

fn resolve_jimeng_poll_endpoint(base_url: &str, poll_url: Option<&str>, task_id: &str) -> String {
    if let Some(url) = poll_url.map(str::trim).filter(|value| !value.is_empty()) {
        if url.starts_with("http://") || url.starts_with("https://") {
            return url.to_string();
        }

        let path = if url.starts_with('/') {
            url.to_string()
        } else {
            format!("/{url}")
        };
        return format!("{}{}", base_url.trim_end_matches('/'), path);
    }

    format!("{}/api/v1/tasks/{task_id}", base_url.trim_end_matches('/'))
}

fn extract_jimeng_video_url(value: &Value) -> Option<String> {
    extract_media_url(value)
        .or_else(|| extract_non_empty_string(value.pointer("/video_url")))
        .or_else(|| extract_non_empty_string(value.pointer("/videoUrl")))
        .or_else(|| extract_non_empty_string(value.pointer("/result/video_url")))
        .or_else(|| extract_non_empty_string(value.pointer("/result/url")))
        .or_else(|| extract_non_empty_string(value.pointer("/output/video_url")))
        .or_else(|| extract_non_empty_string(value.pointer("/output/url")))
        .or_else(|| extract_non_empty_string(value.pointer("/url")))
}

async fn generate_video_with_jimeng(
    base_url: &str,
    api_key: &str,
    model_id: &str,
    image_source: &str,
    options: &VideoGenerateOptions,
) -> Result<String, AppError> {
    let client = http_client();
    let endpoint = format!("{}/v1/videos/generations", base_url.trim_end_matches('/'));

    let prompt = options
        .prompt
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::invalid_params("jimeng video prompt is required"))?;
    let ratio = options
        .aspect_ratio
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("16:9");

    let mut payload = json!({
        "prompt": prompt,
        "model": model_id,
        "duration": options.duration.unwrap_or(5),
        "ratio": ratio,
    });

    if !image_source.trim().is_empty() {
        payload["image_url"] =
            Value::String(media::normalize_source_to_data_url(image_source).await?);
    }

    let response = send_request_with_retry(
        || client.post(&endpoint).bearer_auth(api_key).json(&payload),
        "jimeng video submit",
    )
    .await?;

    let value: Value = response
        .json()
        .await
        .map_err(|error| AppError::internal(format!("invalid jimeng submit response: {error}")))?;
    let task = value.get("task").unwrap_or(&value);
    let task_id = extract_non_empty_string(task.get("id"))
        .or_else(|| extract_non_empty_string(value.get("id")))
        .ok_or_else(|| AppError::internal("jimeng submit response missing task id"))?;
    let poll_endpoint = resolve_jimeng_poll_endpoint(
        base_url,
        task.get("poll_url")
            .and_then(Value::as_str)
            .or_else(|| value.get("poll_url").and_then(Value::as_str)),
        &task_id,
    );

    poll_until_complete("jimeng video", || async {
        let response = send_request_with_retry(
            || client.get(&poll_endpoint).bearer_auth(api_key),
            "jimeng video status",
        )
        .await?;
        let value: Value = response.json().await.map_err(|error| {
            AppError::internal(format!("invalid jimeng status response: {error}"))
        })?;

        let task = value.get("task").unwrap_or(&value);
        let status = extract_non_empty_string(task.get("status"))
            .or_else(|| extract_non_empty_string(value.get("status")))
            .unwrap_or_else(|| "queued".to_string())
            .to_ascii_lowercase();

        if ["queued", "pending", "processing", "running", "in_progress"].contains(&status.as_str())
        {
            return Ok(PollOutcome::Pending);
        }

        if ["failed", "error", "canceled", "cancelled"].contains(&status.as_str()) {
            let message = extract_non_empty_string(task.pointer("/error/message"))
                .or_else(|| extract_non_empty_string(task.get("error_message")))
                .or_else(|| extract_non_empty_string(value.pointer("/error/message")))
                .or_else(|| extract_non_empty_string(value.get("error_message")))
                .unwrap_or_else(|| "jimeng video generation failed".to_string());
            return Ok(PollOutcome::Failed(message));
        }

        if ["completed", "succeeded", "success"].contains(&status.as_str()) {
            if let Some(video_url) =
                extract_jimeng_video_url(task).or_else(|| extract_jimeng_video_url(&value))
            {
                return Ok(PollOutcome::Completed(video_url));
            }

            return Ok(PollOutcome::Failed(
                "jimeng task completed but video url is missing".to_string(),
            ));
        }

        Ok(PollOutcome::Pending)
    })
    .await
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
    client: &reqwest::Client,
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
    client: &reqwest::Client,
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
        "jimeng" => {
            let base_url = provider_base_url(
                provider.base_url.as_deref(),
                "JIMENG_API_BASE_URL",
                DEFAULT_JIMENG_BASE_URL,
            );
            generate_video_with_jimeng(
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
