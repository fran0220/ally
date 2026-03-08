use reqwest::multipart;
use serde_json::{Value, json};
use sqlx::MySqlPool;

use crate::{
    api_config::UnifiedModelType, errors::AppError, media, runtime::resolve_model_with_provider,
};

use super::{
    ImageGenerateOptions, fal::fal_image_endpoint, fal::poll_fal_result, fal::submit_fal_task,
    http_client, parse_inline_image_data, parse_resolution_size,
};

fn is_openai_chat_image_model(model_id: &str) -> bool {
    let normalized = model_id.trim().to_ascii_lowercase();
    normalized.contains("image-preview") || normalized.contains("flash-image")
}

fn extract_openai_chat_image_url(choice: &Value) -> Option<String> {
    if let Some(url) = choice
        .pointer("/message/images/0/image_url/url")
        .and_then(Value::as_str)
    {
        return Some(url.to_string());
    }
    if let Some(url) = choice
        .pointer("/message/images/0/url")
        .and_then(Value::as_str)
    {
        return Some(url.to_string());
    }

    let content = choice.pointer("/message/content")?;
    if let Some(value) = content
        .as_str()
        .map(str::trim)
        .filter(|value| value.starts_with("data:image/"))
    {
        return Some(value.to_string());
    }

    let parts = content.as_array()?;
    for part in parts {
        if let Some(url) = part.pointer("/image_url/url").and_then(Value::as_str) {
            return Some(url.to_string());
        }
        if let Some(url) = part.get("url").and_then(Value::as_str)
            && part.get("type").and_then(Value::as_str) == Some("image_url")
        {
            return Some(url.to_string());
        }
        if let Some(data) = part.get("b64_json").and_then(Value::as_str) {
            return Some(format!("data:image/png;base64,{data}"));
        }
    }

    None
}

async fn generate_image_with_openai_chat(
    base_url: &str,
    api_key: &str,
    model_id: &str,
    prompt: &str,
    options: &ImageGenerateOptions,
) -> Result<String, AppError> {
    let client = http_client();
    let base_url = base_url.trim_end_matches('/');

    let mut content = vec![json!({ "type": "text", "text": prompt })];
    for reference in &options.reference_images {
        content.push(json!({
            "type": "image_url",
            "image_url": {
                "url": media::normalize_source_to_data_url(reference).await?
            }
        }));
    }

    let payload = json!({
        "model": model_id,
        "messages": [{ "role": "user", "content": content }],
        "max_tokens": 2000,
    });

    let response = client
        .post(format!("{base_url}/chat/completions"))
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|err| AppError::internal(format!("openai-compatible chat image failed: {err}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::internal(format!(
            "openai-compatible chat image failed ({status}): {body}"
        )));
    }

    let value: Value = response
        .json()
        .await
        .map_err(|err| AppError::internal(format!("invalid chat image response: {err}")))?;
    let choice = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .ok_or_else(|| AppError::internal("chat image response missing choices"))?;

    extract_openai_chat_image_url(choice)
        .ok_or_else(|| AppError::internal("chat image response missing image payload"))
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
    if is_openai_chat_image_model(model_id) {
        return generate_image_with_openai_chat(base_url, api_key, model_id, prompt, options).await;
    }

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
        let (mime_type, data) = parse_inline_image_data(&item)?;
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
