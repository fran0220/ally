use axum::Json;
use rust_decimal::Decimal;
use serde_json::{Value, json};
use uuid::Uuid;
use waoowaoo_core::billing::{
    BILLING_CURRENCY, BillingParams, check_balance, decimal_to_f64, extract_billing_params,
    get_unit_price, is_billable_task_type,
};
use waoowaoo_core::errors::AppError as CoreAppError;
use waoowaoo_core::runtime::publisher::{
    TaskLifecycleMessageInput, build_task_lifecycle_message, publish_task_message,
};

use crate::{app_state::AppState, error::AppError, extractors::auth::AuthUser};

const DEFAULT_PRIORITY: i32 = 0;
const DEFAULT_MAX_ATTEMPTS: i32 = 3;

pub struct SubmitTaskArgs<'a> {
    pub project_id: &'a str,
    pub episode_id: Option<&'a str>,
    pub task_type: &'a str,
    pub target_type: &'a str,
    pub target_id: &'a str,
    pub priority: Option<i32>,
    pub max_attempts: Option<i32>,
    pub accept_language: Option<&'a str>,
    pub payload: Value,
}

#[derive(Debug, sqlx::FromRow)]
struct ProjectOwnerRow {
    #[sqlx(rename = "userId")]
    user_id: String,
}

#[derive(Debug, sqlx::FromRow)]
struct ActiveTaskRow {
    id: String,
    status: String,
}

fn normalize_optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn build_dedupe_key(
    project_id: &str,
    task_type: &str,
    target_type: &str,
    target_id: &str,
) -> String {
    format!("{project_id}:{task_type}:{target_type}:{target_id}")
}

fn normalize_locale_candidate(raw: &str) -> Option<&'static str> {
    let locale = raw
        .trim()
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if locale.is_empty() {
        return None;
    }

    if locale == "zh" || locale.starts_with("zh-") {
        return Some("zh");
    }

    if locale == "en" || locale.starts_with("en-") {
        return Some("en");
    }

    None
}

fn read_locale_from_payload(payload: &Value) -> Option<&'static str> {
    let from_meta = payload
        .get("meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get("locale"))
        .and_then(Value::as_str)
        .and_then(normalize_locale_candidate);
    if from_meta.is_some() {
        return from_meta;
    }

    payload
        .get("locale")
        .and_then(Value::as_str)
        .and_then(normalize_locale_candidate)
}

fn read_locale_from_accept_language(accept_language: Option<&str>) -> Option<&'static str> {
    accept_language
        .and_then(|raw| raw.split(',').next())
        .and_then(normalize_locale_candidate)
}

fn normalize_payload_locale(mut payload: Value, accept_language: Option<&str>) -> Value {
    let locale = read_locale_from_payload(&payload)
        .or_else(|| read_locale_from_accept_language(accept_language));
    let Some(locale) = locale else {
        return payload;
    };

    let Some(payload_object) = payload.as_object_mut() else {
        return payload;
    };

    let meta_value = payload_object
        .entry("meta".to_string())
        .or_insert_with(|| json!({}));
    if !meta_value.is_object() {
        *meta_value = json!({});
    }

    if let Some(meta) = meta_value.as_object_mut() {
        meta.insert("locale".to_string(), Value::String(locale.to_string()));
    }

    payload
}

fn billing_snapshot_from_params(params: &BillingParams) -> Value {
    json!({
        "api_type": params.api_type.clone(),
        "model": params.model.clone(),
        "quantity": decimal_to_f64(params.quantity),
        "unit": params.unit.clone(),
    })
}

async fn find_active_task_by_dedupe_key(
    state: &AppState,
    dedupe_key: &str,
) -> Result<Option<ActiveTaskRow>, AppError> {
    sqlx::query_as::<_, ActiveTaskRow>(
        "SELECT id, status FROM tasks WHERE dedupeKey = ? AND status IN ('queued', 'processing') ORDER BY createdAt DESC LIMIT 1",
    )
    .bind(dedupe_key)
    .fetch_optional(&state.mysql)
    .await
    .map_err(AppError::from)
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(db_error) => {
            db_error.is_unique_violation() || db_error.code().is_some_and(|code| code == "1062")
        }
        _ => false,
    }
}

pub async fn verify_project_access(
    state: &AppState,
    project_id: &str,
    user_id: &str,
) -> Result<(), AppError> {
    if project_id == "global-asset-hub" {
        return Ok(());
    }

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

pub async fn submit_task(
    state: &AppState,
    user: &AuthUser,
    args: SubmitTaskArgs<'_>,
) -> Result<Json<Value>, AppError> {
    let SubmitTaskArgs {
        project_id,
        episode_id,
        task_type,
        target_type,
        target_id,
        priority,
        max_attempts,
        accept_language,
        payload,
    } = args;

    let project_id = project_id.trim();
    let task_type = task_type.trim();
    let target_type = target_type.trim();
    let target_id = target_id.trim();

    if project_id.is_empty()
        || task_type.is_empty()
        || target_type.is_empty()
        || target_id.is_empty()
    {
        return Err(AppError::invalid_params(
            "project/task/target parameters cannot be empty",
        ));
    }

    let priority = priority.unwrap_or(DEFAULT_PRIORITY);
    let max_attempts = max_attempts.unwrap_or(DEFAULT_MAX_ATTEMPTS);
    if max_attempts <= 0 {
        return Err(AppError::invalid_params(
            "maxAttempts must be greater than 0",
        ));
    }

    verify_project_access(state, project_id, &user.id).await?;

    let dedupe_key = build_dedupe_key(project_id, task_type, target_type, target_id);
    if let Some(active_task) = find_active_task_by_dedupe_key(state, &dedupe_key).await? {
        return Ok(Json(json!({
          "success": true,
          "async": true,
          "taskId": active_task.id,
          "status": active_task.status,
          "deduped": true
        })));
    }

    let task_id = Uuid::new_v4().to_string();
    let payload = normalize_payload_locale(payload, accept_language);
    let mut billing_info_json: Option<Value> = None;
    if state.config.billing_enabled && is_billable_task_type(task_type) {
        if let Some(params) = extract_billing_params(task_type, &payload) {
            // The pre-check is best effort: billing is still finalized in worker when task completes.
            if let Ok(model_price) =
                get_unit_price(&state.mysql, &params.api_type, &params.model, &params.unit).await
            {
                let required_amount = (model_price.unit_price * params.quantity).round_dp(6);
                if required_amount > Decimal::ZERO {
                    let has_balance =
                        check_balance(&state.mysql, &user.id, decimal_to_f64(required_amount))
                            .await?;
                    if !has_balance {
                        return Err(CoreAppError::insufficient_balance(format!(
                            "insufficient balance for estimated task cost ({BILLING_CURRENCY} {:.4})",
                            decimal_to_f64(required_amount),
                        ))
                        .into());
                    }
                }
            }

            billing_info_json = Some(billing_snapshot_from_params(&params));
        }
    }

    let event_payload = payload.clone();
    let normalized_episode_id = normalize_optional_string(episode_id);

    let mut tx = state.mysql.begin().await?;
    sqlx::query(
        "UPDATE tasks SET dedupeKey = NULL, updatedAt = NOW(3) WHERE dedupeKey = ? AND status NOT IN ('queued', 'processing')",
    )
    .bind(&dedupe_key)
    .execute(&mut *tx)
    .await?;

    let insert_result = sqlx::query(
        "INSERT INTO tasks (id, userId, projectId, episodeId, type, targetType, targetId, status, progress, attempt, maxAttempts, priority, dedupeKey, payload, billingInfo, queuedAt, createdAt, updatedAt) VALUES (?, ?, ?, ?, ?, ?, ?, 'queued', 0, 0, ?, ?, ?, ?, ?, NOW(3), NOW(3), NOW(3))",
    )
    .bind(&task_id)
    .bind(&user.id)
    .bind(project_id)
    .bind(normalized_episode_id.clone())
    .bind(task_type)
    .bind(target_type)
    .bind(target_id)
    .bind(max_attempts)
    .bind(priority)
    .bind(&dedupe_key)
    .bind(sqlx::types::Json(payload))
    .bind(billing_info_json.clone().map(sqlx::types::Json))
    .execute(&mut *tx)
    .await;

    if let Err(error) = insert_result {
        tx.rollback().await?;
        if is_unique_violation(&error) {
            if let Some(active_task) = find_active_task_by_dedupe_key(state, &dedupe_key).await? {
                return Ok(Json(json!({
                  "success": true,
                  "async": true,
                  "taskId": active_task.id,
                  "status": active_task.status,
                  "deduped": true
                })));
            }
        }
        return Err(error.into());
    }

    let insert_event_result = sqlx::query(
        "INSERT INTO task_events (taskId, projectId, userId, eventType, payload, createdAt) VALUES (?, ?, ?, 'task.created', ?, NOW(3))",
    )
    .bind(&task_id)
    .bind(project_id)
    .bind(&user.id)
    .bind(sqlx::types::Json(event_payload.clone()))
    .execute(&mut *tx)
    .await;

    let insert_event_result = match insert_event_result {
        Ok(result) => result,
        Err(error) => {
            tx.rollback().await?;
            return Err(error.into());
        }
    };

    let event_id = i64::try_from(insert_event_result.last_insert_id())
        .map_err(|error| AppError::internal(format!("task event id overflow: {error}")))?;

    tx.commit().await?;

    let message = build_task_lifecycle_message(TaskLifecycleMessageInput {
        id: event_id.to_string(),
        event_type: "task.created",
        task_id: &task_id,
        project_id,
        user_id: &user.id,
        task_type,
        target_type,
        target_id,
        episode_id: normalized_episode_id.as_deref(),
        payload: event_payload,
    });
    publish_task_message(&state.redis, project_id, &message).await?;

    Ok(Json(json!({
      "success": true,
      "async": true,
      "taskId": task_id,
      "status": "queued",
      "deduped": false
    })))
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use serde_json::{Value, json};

    use super::{
        billing_snapshot_from_params, build_dedupe_key, normalize_locale_candidate,
        normalize_payload_locale,
    };
    use waoowaoo_core::billing::BillingParams;

    #[test]
    fn build_dedupe_key_uses_expected_format() {
        assert_eq!(
            build_dedupe_key("project-1", "image_panel", "panel", "panel-1"),
            "project-1:image_panel:panel:panel-1"
        );
    }

    #[test]
    fn normalize_locale_candidate_accepts_zh_and_en_variants() {
        assert_eq!(normalize_locale_candidate("zh-CN"), Some("zh"));
        assert_eq!(normalize_locale_candidate("en-US"), Some("en"));
        assert_eq!(normalize_locale_candidate("fr-FR"), None);
    }

    #[test]
    fn normalize_payload_locale_prefers_payload_meta_locale() {
        let payload = json!({
            "meta": {
                "locale": "zh-HK",
            },
        });

        let normalized = normalize_payload_locale(payload, Some("en-US,en;q=0.9"));
        let locale = normalized
            .get("meta")
            .and_then(|meta| meta.get("locale"))
            .and_then(|value| value.as_str());

        assert_eq!(locale, Some("zh"));
    }

    #[test]
    fn normalize_payload_locale_uses_accept_language_when_missing() {
        let payload = json!({});
        let normalized = normalize_payload_locale(payload, Some("en-US,en;q=0.9"));
        let locale = normalized
            .get("meta")
            .and_then(|meta| meta.get("locale"))
            .and_then(|value| value.as_str());

        assert_eq!(locale, Some("en"));
    }

    #[test]
    fn billing_snapshot_from_params_keeps_only_basic_billing_fields() {
        let params = BillingParams {
            api_type: "image".to_string(),
            model: "fal::banana-2".to_string(),
            quantity: Decimal::new(2, 0),
            unit: "image:2K".to_string(),
            metadata: Some(json!({ "resolution": "2K" })),
        };

        let snapshot = billing_snapshot_from_params(&params);

        assert_eq!(
            snapshot.get("api_type").and_then(Value::as_str),
            Some("image")
        );
        assert_eq!(
            snapshot.get("model").and_then(Value::as_str),
            Some("fal::banana-2")
        );
        assert_eq!(snapshot.get("quantity").and_then(Value::as_f64), Some(2.0));
        assert_eq!(
            snapshot.get("unit").and_then(Value::as_str),
            Some("image:2K")
        );
        assert_eq!(snapshot.get("metadata"), None);
    }
}
