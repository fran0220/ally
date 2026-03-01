use std::{env, path::PathBuf};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use tokio::fs;
use uuid::Uuid;

use crate::errors::AppError;

fn storage_type() -> String {
    env::var("STORAGE_TYPE")
        .unwrap_or_else(|_| "cos".to_string())
        .trim()
        .to_lowercase()
}

fn upload_dir() -> PathBuf {
    env::var("UPLOAD_DIR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./data/uploads"))
}

fn app_base_url() -> String {
    env::var("NEXTAUTH_URL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "http://localhost:3000".to_string())
        .trim_end_matches('/')
        .to_string()
}

fn normalize_storage_key(key: &str) -> String {
    key.trim_start_matches('/').trim().to_string()
}

fn content_type_to_extension(content_type: &str) -> &'static str {
    match content_type.to_ascii_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "video/mp4" => "mp4",
        "video/quicktime" => "mov",
        "video/webm" => "webm",
        "audio/wav" | "audio/x-wav" => "wav",
        "audio/mpeg" => "mp3",
        "audio/ogg" => "ogg",
        _ => "png",
    }
}

pub fn to_fetchable_url(input_url: &str) -> String {
    let trimmed = input_url.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("data:")
    {
        return trimmed.to_string();
    }

    if trimmed.starts_with('/') {
        return format!("{}{}", app_base_url(), trimmed);
    }

    if storage_type() == "local" {
        return format!("{}/api/files/{}", app_base_url(), trimmed);
    }

    trimmed.to_string()
}

pub fn to_public_media_url(input_url_or_key: Option<&str>) -> Option<String> {
    let value = input_url_or_key?.trim();
    if value.is_empty() {
        return None;
    }
    Some(to_fetchable_url(value))
}

pub fn parse_data_url(value: &str) -> Option<(String, String)> {
    let marker = ";base64,";
    if !value.starts_with("data:") {
        return None;
    }
    let marker_pos = value.find(marker)?;
    let mime_type = value.get(5..marker_pos)?.trim().to_string();
    let payload = value.get(marker_pos + marker.len()..)?.trim().to_string();
    if mime_type.is_empty() || payload.is_empty() {
        return None;
    }
    Some((mime_type, payload))
}

pub async fn download_source_bytes(source: &str) -> Result<(Vec<u8>, String), AppError> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid_params("empty media source"));
    }

    if let Some((mime_type, base64_data)) = parse_data_url(trimmed) {
        let bytes = STANDARD
            .decode(base64_data.as_bytes())
            .map_err(|err| AppError::invalid_params(format!("invalid base64 media data: {err}")))?;
        return Ok((bytes, mime_type));
    }

    let url = to_fetchable_url(trimmed);
    let response = reqwest::get(&url)
        .await
        .map_err(|err| AppError::internal(format!("failed to download source media: {err}")))?;
    if !response.status().is_success() {
        return Err(AppError::internal(format!(
            "failed to download source media: status {}",
            response.status()
        )));
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(';').next().unwrap_or(value).trim().to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string());

    let bytes = response
        .bytes()
        .await
        .map_err(|err| AppError::internal(format!("failed to read source media bytes: {err}")))?
        .to_vec();

    Ok((bytes, content_type))
}

pub async fn normalize_source_to_data_url(source: &str) -> Result<String, AppError> {
    if source.trim().starts_with("data:") {
        return Ok(source.trim().to_string());
    }

    let (bytes, content_type) = download_source_bytes(source).await?;
    Ok(format!(
        "data:{};base64,{}",
        content_type,
        STANDARD.encode(bytes)
    ))
}

pub async fn normalize_reference_sources_to_data_urls(
    references: &[String],
) -> Result<Vec<String>, AppError> {
    let mut normalized = Vec::with_capacity(references.len());
    for source in references {
        let value = source.trim();
        if value.is_empty() {
            continue;
        }
        normalized.push(normalize_source_to_data_url(value).await?);
    }
    Ok(normalized)
}

pub async fn upload_bytes_to_storage(key: &str, bytes: &[u8]) -> Result<String, AppError> {
    let storage_key = normalize_storage_key(key);
    if storage_key.is_empty() {
        return Err(AppError::invalid_params("storage key cannot be empty"));
    }

    if storage_type() != "local" {
        return Err(AppError::internal(
            "only local storage mode is currently supported by rust worker",
        ));
    }

    let root = upload_dir();
    let file_path = root.join(&storage_key);
    let parent = file_path
        .parent()
        .ok_or_else(|| AppError::internal("invalid storage path"))?;

    fs::create_dir_all(parent)
        .await
        .map_err(|err| AppError::internal(format!("failed to create upload directory: {err}")))?;
    fs::write(&file_path, bytes)
        .await
        .map_err(|err| AppError::internal(format!("failed to write upload file: {err}")))?;

    Ok(storage_key)
}

pub async fn upload_source_to_storage(
    source: &str,
    key_prefix: &str,
    target_id: &str,
) -> Result<String, AppError> {
    let (bytes, content_type) = download_source_bytes(source).await?;
    let ext = content_type_to_extension(&content_type);
    let key = format!(
        "{}/{target}-{}.{}",
        key_prefix.trim_matches('/'),
        Uuid::new_v4(),
        ext,
        target = target_id.trim()
    );
    upload_bytes_to_storage(&key, &bytes).await
}
