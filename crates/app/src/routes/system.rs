use axum::Json;
use serde_json::json;
use waoowaoo_core::system::SERVER_BOOT_ID;

pub async fn health() -> Json<serde_json::Value> {
    Json(json!({"status": "ok"}))
}

pub async fn boot_id() -> Json<serde_json::Value> {
    Json(json!({"bootId": SERVER_BOOT_ID.as_str()}))
}
