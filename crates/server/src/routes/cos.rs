use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use serde_json::json;

use crate::{app_state::AppState, error::AppError};

#[derive(Debug, Deserialize)]
pub struct CosImageQuery {
    pub key: String,
}

fn normalize_storage_type() -> String {
    std::env::var("STORAGE_TYPE")
        .ok()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "cos".to_string())
}

fn normalize_upload_dir() -> String {
    std::env::var("UPLOAD_DIR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "./data/uploads".to_string())
}

fn has_request_auth(headers: &HeaderMap) -> bool {
    if let Some(value) = headers.get(header::AUTHORIZATION)
        && value
            .to_str()
            .map(|raw| !raw.trim().is_empty())
            .unwrap_or(false)
    {
        return true;
    }

    if let Some(cookie_header) = headers.get(header::COOKIE)
        && let Ok(raw) = cookie_header.to_str()
    {
        for segment in raw.split(';') {
            let mut kv = segment.trim().splitn(2, '=');
            let key = kv.next().unwrap_or_default().trim();
            let value = kv.next().unwrap_or_default().trim();
            if key == "token" && !value.is_empty() {
                return true;
            }
        }
    }

    false
}

pub async fn image(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CosImageQuery>,
) -> Result<axum::response::Response, AppError> {
    if !has_request_auth(&headers) {
        return Ok((StatusCode::NOT_FOUND, Json(json!({ "error": "Not Found" }))).into_response());
    }

    let key = query.key.trim();
    if key.is_empty() {
        return Err(AppError::invalid_params("key is required"));
    }

    let storage_type = normalize_storage_type();
    if storage_type == "local" {
        let encoded = key
            .split('/')
            .map(urlencoding::encode)
            .collect::<Vec<_>>()
            .join("/");
        let url = format!("/api/files/{encoded}");
        return Ok(Redirect::temporary(&url).into_response());
    }

    if let Ok(base_url) = std::env::var("COS_PUBLIC_BASE_URL") {
        let normalized = base_url.trim().trim_end_matches('/');
        if !normalized.is_empty() {
            let encoded = key
                .split('/')
                .map(urlencoding::encode)
                .collect::<Vec<_>>()
                .join("/");
            let url = format!("{normalized}/{encoded}");
            return Ok(Redirect::temporary(&url).into_response());
        }
    }

    if let (Ok(bucket), Ok(region)) = (std::env::var("COS_BUCKET"), std::env::var("COS_REGION")) {
        let bucket = bucket.trim();
        let region = region.trim();
        if !bucket.is_empty() && !region.is_empty() {
            let encoded = key
                .split('/')
                .map(urlencoding::encode)
                .collect::<Vec<_>>()
                .join("/");
            let url = format!("https://{bucket}.cos.{region}.myqcloud.com/{encoded}");
            return Ok(Redirect::temporary(&url).into_response());
        }
    }

    let upload_dir = normalize_upload_dir();
    Ok((
        axum::http::StatusCode::NOT_IMPLEMENTED,
        Json(json!({
          "success": false,
          "code": "MISSING_CONFIG",
          "message": "COS redirect config missing. Set COS_PUBLIC_BASE_URL or COS_BUCKET/COS_REGION, or use STORAGE_TYPE=local",
          "details": {
            "storageType": storage_type,
            "uploadDir": upload_dir
          }
        })),
    )
        .into_response())
}

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/api/cos/image", axum::routing::get(image))
        .route("/api/cos/sign", axum::routing::get(image))
}
