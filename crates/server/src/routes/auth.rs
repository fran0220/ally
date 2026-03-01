use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::MySql;
use uuid::Uuid;
use waoowaoo_core::auth::{hash_password, verify_password};

use crate::error::AppError;

use crate::{app_state::AppState, extractors::auth::AuthUser};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
struct AuthUserResponse {
    id: String,
    name: String,
    role: String,
}

#[derive(Debug, Serialize)]
struct AuthResponse {
    token: String,
    user: AuthUserResponse,
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: String,
    name: String,
    role: String,
    password: Option<String>,
}

fn set_token_cookie(headers: &mut HeaderMap, token: &str) -> Result<(), AppError> {
    let cookie = format!("token={token}; HttpOnly; Path=/; SameSite=Lax");
    let value = header::HeaderValue::from_str(&cookie)
        .map_err(|err| AppError::internal(format!("failed to build auth cookie: {err}")))?;
    headers.insert(header::SET_COOKIE, value);
    Ok(())
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, AppError> {
    let name = payload.name.trim();
    if name.is_empty() {
        return Err(AppError::invalid_params("name is required"));
    }
    if payload.password.len() < 6 {
        return Err(AppError::invalid_params(
            "password must be at least 6 chars",
        ));
    }

    let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM user WHERE name = ? LIMIT 1")
        .bind(name)
        .fetch_optional(&state.mysql)
        .await?;
    if existing.is_some() {
        return Err(AppError::conflict("user already exists"));
    }

    let hash = hash_password(&payload.password)?;
    let user_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO user (id, name, password, role, createdAt, updatedAt) VALUES (?, ?, ?, 'user', NOW(3), NOW(3))",
    )
    .bind(&user_id)
    .bind(name)
    .bind(hash)
    .execute(&state.mysql)
    .await?;

    let token = state.jwt.issue_token(&user_id, name, "user")?;

    let mut headers = HeaderMap::new();
    set_token_cookie(&mut headers, &token)?;

    let response = Json(json!({
        "message": "注册成功",
        "token": token,
        "user": {
            "id": user_id,
            "name": name,
            "role": "user"
        }
    }));

    Ok((StatusCode::CREATED, headers, response))
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    if payload.username.trim().is_empty() || payload.password.is_empty() {
        return Err(AppError::invalid_params(
            "username and password are required",
        ));
    }

    let user = sqlx::query_as::<MySql, UserRow>(
        "SELECT id, name, role, password FROM user WHERE name = ? LIMIT 1",
    )
    .bind(payload.username.trim())
    .fetch_optional(&state.mysql)
    .await?
    .ok_or_else(|| AppError::unauthorized("invalid credentials"))?;

    let hash = user
        .password
        .as_deref()
        .ok_or_else(|| AppError::unauthorized("invalid credentials"))?;

    let verified = verify_password(&payload.password, hash)?;
    if !verified {
        return Err(AppError::unauthorized("invalid credentials"));
    }

    let token = state.jwt.issue_token(&user.id, &user.name, &user.role)?;

    let mut headers = HeaderMap::new();
    set_token_cookie(&mut headers, &token)?;

    let response = Json(AuthResponse {
        token,
        user: AuthUserResponse {
            id: user.id,
            name: user.name,
            role: user.role,
        },
    });

    Ok((StatusCode::OK, headers, response))
}

pub async fn refresh(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    let token = state
        .jwt
        .issue_token(&user.id, &user.username, &user.role)?;
    let mut headers = HeaderMap::new();
    set_token_cookie(&mut headers, &token)?;

    Ok((
        StatusCode::OK,
        headers,
        Json(json!({
            "token": token,
            "user": {
                "id": user.id,
                "name": user.username,
                "role": user.role,
            }
        })),
    ))
}
