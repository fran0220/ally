use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::{app_state::AppState, error::AppError, extractors::auth::AuthUser};

#[derive(Debug, Deserialize)]
pub struct ProjectListQuery {
    #[serde(default = "default_page")]
    page: i64,
    #[serde(default = "default_page_size", rename = "pageSize")]
    page_size: i64,
    #[serde(default)]
    search: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    name: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectRequest {
    name: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct ProjectRow {
    id: String,
    name: String,
    description: Option<String>,
    mode: String,
    #[sqlx(rename = "userId")]
    user_id: String,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    updated_at: NaiveDateTime,
    #[sqlx(rename = "lastAccessedAt")]
    last_accessed_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct EpisodeRow {
    id: String,
    #[sqlx(rename = "episodeNumber")]
    episode_number: i32,
    name: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct CharacterRow {
    id: String,
    name: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct LocationRow {
    id: String,
    name: String,
}

#[derive(Debug, sqlx::FromRow)]
struct ProjectOwnerRow {
    #[sqlx(rename = "userId")]
    user_id: String,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    12
}

fn normalize_page(page: i64) -> i64 {
    page.max(1)
}

fn normalize_page_size(page_size: i64) -> i64 {
    page_size.clamp(1, 100)
}

async fn verify_project_owner(
    state: &AppState,
    project_id: &str,
    user_id: &str,
) -> Result<(), AppError> {
    let owner =
        sqlx::query_as::<_, ProjectOwnerRow>("SELECT userId FROM projects WHERE id = ? LIMIT 1")
            .bind(project_id)
            .fetch_optional(&state.mysql)
            .await?;

    let Some(owner) = owner else {
        return Err(AppError::not_found("project not found"));
    };

    if owner.user_id != user_id {
        return Err(AppError::forbidden("project access denied"));
    }

    Ok(())
}

pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
    Query(params): Query<ProjectListQuery>,
) -> Result<Json<Value>, AppError> {
    let page = normalize_page(params.page);
    let page_size = normalize_page_size(params.page_size);
    let offset = (page - 1) * page_size;

    let search = params.search.unwrap_or_default().trim().to_string();

    let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
        "SELECT id, name, description, mode, userId, createdAt, updatedAt, lastAccessedAt FROM projects WHERE userId = ",
    );
    qb.push_bind(&user.id);

    if !search.is_empty() {
        let like = format!("%{search}%");
        qb.push(" AND (name LIKE ");
        qb.push_bind(like.clone());
        qb.push(" OR description LIKE ");
        qb.push_bind(like.clone());
        qb.push(")");
    }

    qb.push(" ORDER BY COALESCE(lastAccessedAt, createdAt) DESC LIMIT ");
    qb.push_bind(page_size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);

    let projects = qb
        .build_query_as::<ProjectRow>()
        .fetch_all(&state.mysql)
        .await?;

    let mut count_qb: QueryBuilder<'_, MySql> =
        QueryBuilder::new("SELECT COUNT(*) FROM projects WHERE userId = ");
    count_qb.push_bind(&user.id);

    if !search.is_empty() {
        let like = format!("%{search}%");
        count_qb.push(" AND (name LIKE ");
        count_qb.push_bind(like.clone());
        count_qb.push(" OR description LIKE ");
        count_qb.push_bind(like.clone());
        count_qb.push(")");
    }

    let total = count_qb
        .build_query_scalar::<i64>()
        .fetch_one(&state.mysql)
        .await?;

    let total_pages = if total == 0 {
        0
    } else {
        ((total + page_size - 1) / page_size).max(1)
    };

    Ok(Json(json!({
      "projects": projects,
      "pagination": {
        "page": page,
        "pageSize": page_size,
        "total": total,
        "totalPages": total_pages
      }
    })))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<CreateProjectRequest>,
) -> Result<Json<Value>, AppError> {
    let name = payload.name.trim();
    if name.is_empty() {
        return Err(AppError::invalid_params("project name is required"));
    }
    if name.len() > 100 {
        return Err(AppError::invalid_params("project name too long"));
    }
    if payload
        .description
        .as_ref()
        .map(|value| value.len() > 500)
        .unwrap_or(false)
    {
        return Err(AppError::invalid_params("project description too long"));
    }

    let project_id = Uuid::new_v4().to_string();
    let novel_id = Uuid::new_v4().to_string();
    let normalized_description = payload.description.map(|value| value.trim().to_string());

    let mut tx = state.mysql.begin().await?;

    sqlx::query(
        "INSERT INTO projects (id, name, description, mode, userId, createdAt, updatedAt) VALUES (?, ?, ?, 'novel-promotion', ?, NOW(3), NOW(3))",
    )
    .bind(&project_id)
    .bind(name)
    .bind(normalized_description.clone())
    .bind(&user.id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO novel_promotion_projects (id, projectId, videoRatio, ttsRate, artStyle, workflowMode, videoResolution, imageResolution, createdAt, updatedAt) VALUES (?, ?, '9:16', '+50%', 'american-comic', 'srt', '720p', '2K', NOW(3), NOW(3))",
    )
    .bind(&novel_id)
    .bind(&project_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let project = sqlx::query_as::<_, ProjectRow>(
        "SELECT id, name, description, mode, userId, createdAt, updatedAt, lastAccessedAt FROM projects WHERE id = ? LIMIT 1",
    )
    .bind(&project_id)
    .fetch_one(&state.mysql)
    .await?;

    Ok(Json(json!({
      "project": project
    })))
}

pub async fn get(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(&state, &project_id, &user.id).await?;

    let project = sqlx::query_as::<_, ProjectRow>(
        "SELECT id, name, description, mode, userId, createdAt, updatedAt, lastAccessedAt FROM projects WHERE id = ? LIMIT 1",
    )
    .bind(&project_id)
    .fetch_one(&state.mysql)
    .await?;

    Ok(Json(json!({ "project": project })))
}

pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
    Json(payload): Json<UpdateProjectRequest>,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(&state, &project_id, &user.id).await?;

    let mut builder: QueryBuilder<'_, MySql> = QueryBuilder::new("UPDATE projects SET ");
    let mut separated = builder.separated(", ");
    let mut touched = false;

    if let Some(name) = payload.name {
        touched = true;
        separated.push("name = ");
        separated.push_bind(name.trim().to_string());
    }

    if let Some(description) = payload.description {
        touched = true;
        separated.push("description = ");
        separated.push_bind(description.trim().to_string());
    }

    if !touched {
        return Err(AppError::invalid_params("empty update payload"));
    }

    separated.push("updatedAt = NOW(3)");
    builder.push(" WHERE id = ");
    builder.push_bind(&project_id);

    builder.build().execute(&state.mysql).await?;

    get(State(state), user, Path(project_id)).await
}

pub async fn delete(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(&state, &project_id, &user.id).await?;

    sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(&project_id)
        .execute(&state.mysql)
        .await?;

    Ok(Json(json!({
      "success": true,
      "cosFilesDeleted": 0,
      "cosFilesFailed": 0,
    })))
}

pub async fn assets(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(&state, &project_id, &user.id).await?;

    let novel_project_id: Option<(String,)> =
        sqlx::query_as("SELECT id FROM novel_promotion_projects WHERE projectId = ? LIMIT 1")
            .bind(&project_id)
            .fetch_optional(&state.mysql)
            .await?;

    let Some((novel_id,)) = novel_project_id else {
        return Err(AppError::not_found("novel promotion data not found"));
    };

    let characters = sqlx::query_as::<_, CharacterRow>(
        "SELECT id, name FROM novel_promotion_characters WHERE novelPromotionProjectId = ? ORDER BY createdAt ASC",
    )
    .bind(&novel_id)
    .fetch_all(&state.mysql)
    .await?;

    let locations = sqlx::query_as::<_, LocationRow>(
        "SELECT id, name FROM novel_promotion_locations WHERE novelPromotionProjectId = ? ORDER BY createdAt ASC",
    )
    .bind(&novel_id)
    .fetch_all(&state.mysql)
    .await?;

    Ok(Json(json!({
      "characters": characters,
      "locations": locations,
    })))
}

pub async fn data(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    verify_project_owner(&state, &project_id, &user.id).await?;

    let project = sqlx::query_as::<_, ProjectRow>(
        "SELECT id, name, description, mode, userId, createdAt, updatedAt, lastAccessedAt FROM projects WHERE id = ? LIMIT 1",
    )
    .bind(&project_id)
    .fetch_one(&state.mysql)
    .await?;

    let novel_data = sqlx::query_as::<_, (String, String, Option<String>, Option<String>, Option<String>, String, String, String)>(
        "SELECT id, projectId, analysisModel, imageModel, videoModel, videoRatio, artStyle, ttsRate FROM novel_promotion_projects WHERE projectId = ? LIMIT 1",
    )
    .bind(&project_id)
    .fetch_optional(&state.mysql)
    .await?;

    let episodes = sqlx::query_as::<_, EpisodeRow>(
        "SELECT id, episodeNumber, name FROM novel_promotion_episodes WHERE novelPromotionProjectId = (SELECT id FROM novel_promotion_projects WHERE projectId = ? LIMIT 1) ORDER BY episodeNumber ASC",
    )
    .bind(&project_id)
    .fetch_all(&state.mysql)
    .await?;

    Ok(Json(json!({
      "project": {
        "id": project.id,
        "name": project.name,
        "description": project.description,
        "mode": project.mode,
        "userId": project.user_id,
        "createdAt": project.created_at,
        "updatedAt": project.updated_at,
        "lastAccessedAt": project.last_accessed_at,
        "novelPromotionData": novel_data.map(|row| {
          json!({
            "id": row.0,
            "projectId": row.1,
            "analysisModel": row.2,
            "imageModel": row.3,
            "videoModel": row.4,
            "videoRatio": row.5,
            "artStyle": row.6,
            "ttsRate": row.7,
            "episodes": episodes,
          })
        })
      }
    })))
}

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/api/projects", axum::routing::get(list).post(create))
        .route(
            "/api/projects/{id}",
            axum::routing::get(get).patch(update).delete(delete),
        )
        .route("/api/projects/{id}/assets", axum::routing::get(assets))
        .route("/api/projects/{id}/data", axum::routing::get(data))
}
