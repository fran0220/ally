use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};

use crate::{app_state::AppState, error::AppError};

#[derive(Debug, sqlx::FromRow)]
struct MediaObjectRow {
    id: String,
    #[sqlx(rename = "publicId")]
    public_id: String,
    #[sqlx(rename = "storageKey")]
    storage_key: String,
    #[sqlx(rename = "mimeType")]
    mime_type: Option<String>,
    #[sqlx(rename = "updatedAt")]
    updated_at: chrono::NaiveDateTime,
}

fn normalize_storage_type() -> String {
    std::env::var("STORAGE_TYPE")
        .ok()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "cos".to_string())
}

pub async fn proxy(
    State(state): State<AppState>,
    Path(public_id): Path<String>,
) -> Result<axum::response::Response, AppError> {
    let media = match sqlx::query_as::<_, MediaObjectRow>(
        "SELECT id, publicId, storageKey, mimeType, updatedAt FROM media_objects WHERE publicId = ? LIMIT 1",
    )
    .bind(&public_id)
    .fetch_optional(&state.mysql)
    .await?
    {
        Some(media) => media,
        None => {
            return Ok((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Media not found" }))).into_response())
        }
    };

    if media.storage_key.trim().is_empty() {
        return Err(AppError::internal("media storage key missing"));
    }

    let storage_type = normalize_storage_type();

    if storage_type == "local" {
        let encoded = media
            .storage_key
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
            let encoded = media
                .storage_key
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
            let encoded = media
                .storage_key
                .split('/')
                .map(urlencoding::encode)
                .collect::<Vec<_>>()
                .join("/");
            let url = format!("https://{bucket}.cos.{region}.myqcloud.com/{encoded}");
            return Ok(Redirect::temporary(&url).into_response());
        }
    }

    Ok((
        StatusCode::NOT_IMPLEMENTED,
        axum::Json(serde_json::json!({
          "success": false,
          "code": "MISSING_CONFIG",
          "message": "media proxy cannot resolve COS redirect url",
          "media": {
            "id": media.id,
            "publicId": media.public_id,
            "mimeType": media.mime_type,
            "updatedAt": media.updated_at,
          }
        })),
    )
        .into_response())
}

pub fn router() -> axum::Router<AppState> {
    axum::Router::new().route("/m/{publicId}", axum::routing::get(proxy))
}
