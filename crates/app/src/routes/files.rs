use std::path::{Component, Path, PathBuf};

use axum::{
    Json,
    body::Body,
    extract::{Path as AxumPath, State},
    http::{HeaderValue, Response, StatusCode, header},
    response::IntoResponse,
};
use serde_json::json;
use tokio::fs;

use crate::{app_state::AppState, error::AppError};

fn normalize_upload_dir() -> PathBuf {
    std::env::var("UPLOAD_DIR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./data/uploads"))
}

fn mime_type_by_ext(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "json" => "application/json",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
}

fn normalize_safe_path(root: &Path, raw: &str) -> Result<PathBuf, AppError> {
    let decoded = urlencoding::decode(raw)
        .map_err(|_| AppError::invalid_params("invalid encoded file path"))?
        .to_string();

    if Path::new(&decoded)
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(AppError::forbidden("path traversal is forbidden"));
    }

    let combined = root.join(decoded);
    let normalized = combined.components().fold(PathBuf::new(), |mut acc, comp| {
        acc.push(comp);
        acc
    });

    let root_abs = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    if let Ok(path_abs) = normalized.canonicalize()
        && !path_abs.starts_with(&root_abs)
    {
        return Err(AppError::forbidden("path traversal is forbidden"));
    }

    Ok(normalized)
}

pub async fn get(
    State(_state): State<AppState>,
    AxumPath(path): AxumPath<String>,
) -> Result<Response<Body>, AppError> {
    let root = normalize_upload_dir();
    let safe_path = normalize_safe_path(&root, &path)?;

    let file_bytes = match fs::read(&safe_path).await {
        Ok(bytes) => bytes,
        Err(err) => {
            if err.kind() == std::io::ErrorKind::NotFound {
                return Ok((
                    StatusCode::NOT_FOUND,
                    Json(json!({ "error": "File not found" })),
                )
                    .into_response());
            }
            return Err(AppError::internal(format!("failed to read file: {err}")));
        }
    };

    let mut response = Response::new(Body::from(file_bytes.clone()));
    *response.status_mut() = StatusCode::OK;

    let mime = mime_type_by_ext(&safe_path);
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(mime));
    response.headers_mut().insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&file_bytes.len().to_string())
            .map_err(|err| AppError::internal(format!("invalid content-length: {err}")))?,
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000"),
    );

    Ok(response)
}

pub fn router() -> axum::Router<AppState> {
    axum::Router::new().route("/api/files/{*path}", axum::routing::get(get))
}
