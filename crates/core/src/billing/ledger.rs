use chrono::Utc;
use rust_decimal::Decimal;
use serde_json::{Map, Value, json};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::errors::{AppError, ErrorCode};

use super::{
    money::{decimal_from_f64, decimal_to_f64, normalize_money},
    types::{BalanceSnapshot, BillingApiType, FreezeSnapshot, UsageUnit},
};

const VIRTUAL_PROJECT_IDS: &[&str] = &["asset-hub", "global-asset-hub", "system"];

#[derive(Debug, Clone)]
pub struct FreezeBalanceOptions {
    pub source: Option<String>,
    pub task_id: Option<String>,
    pub request_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct ConfirmChargeInput {
    pub project_id: String,
    pub action: String,
    pub api_type: BillingApiType,
    pub model: String,
    pub quantity: f64,
    pub unit: UsageUnit,
    pub metadata: Option<Value>,
    pub episode_id: Option<String>,
    pub task_type: Option<String>,
    pub charged_amount: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct AddBalanceOptions {
    pub reason: Option<String>,
    pub operator_id: Option<String>,
    pub external_order_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub tx_type: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct FreezeRow {
    id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
    amount: Decimal,
    status: String,
}

fn parse_amount(amount: f64) -> Result<Decimal, AppError> {
    let amount = decimal_from_f64(amount)
        .ok_or_else(|| AppError::invalid_params("billing amount must be a finite number"))?;
    Ok(normalize_money(amount))
}

fn trim_or_none(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn is_project_scoped(project_id: &str) -> bool {
    let project_id = project_id.trim();
    !project_id.is_empty() && !VIRTUAL_PROJECT_IDS.contains(&project_id)
}

fn build_billing_meta(input: &ConfirmChargeInput, charged_amount: Decimal) -> String {
    let model_short = input
        .model
        .rsplit("::")
        .next()
        .unwrap_or(input.model.as_str())
        .to_string();

    let mut payload = Map::new();
    payload.insert(
        "quantity".to_string(),
        Value::from(
            normalize_money(parse_amount(input.quantity).unwrap_or(Decimal::ZERO)).to_string(),
        ),
    );
    payload.insert(
        "unit".to_string(),
        Value::String(input.unit.as_str().to_string()),
    );
    payload.insert("model".to_string(), Value::String(model_short));
    payload.insert(
        "apiType".to_string(),
        Value::String(input.api_type.as_str().to_string()),
    );
    payload.insert(
        "chargedCost".to_string(),
        Value::from(decimal_to_f64(charged_amount)),
    );

    if let Some(Value::Object(metadata)) = input.metadata.as_ref() {
        for (key, value) in metadata {
            if !payload.contains_key(key) {
                payload.insert(key.clone(), value.clone());
            }
        }
    }

    Value::Object(payload).to_string()
}

async fn ensure_user_balance_row(pool: &MySqlPool, user_id: &str) -> Result<(), AppError> {
    let row_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO user_balances (id, userId, balance, frozenAmount, totalSpent, createdAt, updatedAt) VALUES (?, ?, 0, 0, 0, NOW(3), NOW(3)) ON DUPLICATE KEY UPDATE updatedAt = updatedAt",
    )
    .bind(row_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_balance(pool: &MySqlPool, user_id: &str) -> Result<BalanceSnapshot, AppError> {
    ensure_user_balance_row(pool, user_id).await?;

    sqlx::query_as::<_, BalanceSnapshot>(
        "SELECT id, userId, balance, frozenAmount, totalSpent, createdAt, updatedAt FROM user_balances WHERE userId = ? LIMIT 1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn get_freeze_by_idempotency_key(
    pool: &MySqlPool,
    idempotency_key: &str,
) -> Result<Option<FreezeSnapshot>, AppError> {
    let key = idempotency_key.trim();
    if key.is_empty() {
        return Ok(None);
    }

    sqlx::query_as::<_, FreezeSnapshot>(
        "SELECT id, userId, amount, status FROM balance_freezes WHERE idempotencyKey = ? LIMIT 1",
    )
    .bind(key)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)
}

pub async fn check_balance(
    pool: &MySqlPool,
    user_id: &str,
    required_amount: f64,
) -> Result<bool, AppError> {
    let required = parse_amount(required_amount)?;
    let balance = get_balance(pool, user_id).await?;
    Ok(balance.balance >= required)
}

pub async fn freeze_balance(
    pool: &MySqlPool,
    user_id: &str,
    amount: f64,
    options: Option<FreezeBalanceOptions>,
) -> Result<Option<String>, AppError> {
    let normalized_amount = parse_amount(amount)?;
    if normalized_amount <= Decimal::ZERO {
        return Ok(None);
    }

    let source = trim_or_none(options.as_ref().and_then(|item| item.source.clone()));
    let task_id = trim_or_none(options.as_ref().and_then(|item| item.task_id.clone()));
    let request_id = trim_or_none(options.as_ref().and_then(|item| item.request_id.clone()));
    let idempotency_key = trim_or_none(
        options
            .as_ref()
            .and_then(|item| item.idempotency_key.clone()),
    );
    let metadata_json = options
        .as_ref()
        .and_then(|item| item.metadata.as_ref().map(Value::to_string));

    if let Some(key) = idempotency_key.as_deref()
        && let Some(existing) = get_freeze_by_idempotency_key(pool, key).await?
    {
        return Ok(Some(existing.id));
    }

    let mut tx = pool.begin().await?;

    let balance_row_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO user_balances (id, userId, balance, frozenAmount, totalSpent, createdAt, updatedAt) VALUES (?, ?, 0, 0, 0, NOW(3), NOW(3)) ON DUPLICATE KEY UPDATE updatedAt = updatedAt",
    )
    .bind(balance_row_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    let updated = sqlx::query(
        "UPDATE user_balances SET balance = balance - ?, frozenAmount = frozenAmount + ?, updatedAt = NOW(3) WHERE userId = ? AND balance >= ?",
    )
    .bind(normalized_amount)
    .bind(normalized_amount)
    .bind(user_id)
    .bind(normalized_amount)
    .execute(&mut *tx)
    .await?;

    if updated.rows_affected() == 0 {
        tx.rollback().await?;
        return Ok(None);
    }

    let freeze_id = Uuid::new_v4().to_string();
    let created = sqlx::query(
        "INSERT INTO balance_freezes (id, userId, amount, status, source, taskId, requestId, idempotencyKey, metadata, createdAt, updatedAt) VALUES (?, ?, ?, 'pending', ?, ?, ?, ?, ?, NOW(3), NOW(3))",
    )
    .bind(&freeze_id)
    .bind(user_id)
    .bind(normalized_amount)
    .bind(source)
    .bind(task_id)
    .bind(request_id)
    .bind(idempotency_key.clone())
    .bind(metadata_json)
    .execute(&mut *tx)
    .await;

    if let Err(error) = created {
        if let sqlx::Error::Database(db_error) = &error
            && db_error.code().is_some_and(|code| code == "1062")
            && let Some(key) = idempotency_key.as_deref()
            && let Some(existing) = get_freeze_by_idempotency_key(pool, key).await?
        {
            tx.rollback().await?;
            return Ok(Some(existing.id));
        }
        return Err(error.into());
    }

    tx.commit().await?;
    Ok(Some(freeze_id))
}

pub async fn confirm_charge_with_record(
    pool: &MySqlPool,
    freeze_id: &str,
    input: &ConfirmChargeInput,
) -> Result<bool, AppError> {
    let mut tx = pool.begin().await?;

    let freeze = sqlx::query_as::<_, FreezeRow>(
        "SELECT id, userId, amount, status FROM balance_freezes WHERE id = ? LIMIT 1 FOR UPDATE",
    )
    .bind(freeze_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::invalid_params("invalid freeze record"))?;

    if freeze.status == "confirmed" {
        tx.rollback().await?;
        return Ok(true);
    }

    if freeze.status != "pending" {
        return Err(AppError::conflict("freeze is not pending"));
    }

    let charged_amount = match input.charged_amount {
        Some(value) => parse_amount(value)?,
        None => freeze.amount,
    };
    if charged_amount < Decimal::ZERO || charged_amount > freeze.amount {
        return Err(AppError::invalid_params("invalid charged amount"));
    }

    let refund_amount = normalize_money(freeze.amount - charged_amount);
    let switched = sqlx::query(
        "UPDATE balance_freezes SET status = 'confirmed', updatedAt = NOW(3) WHERE id = ? AND status = 'pending'",
    )
    .bind(freeze_id)
    .execute(&mut *tx)
    .await?;

    if switched.rows_affected() == 0 {
        let latest = sqlx::query_as::<_, FreezeRow>(
            "SELECT id, userId, amount, status FROM balance_freezes WHERE id = ? LIMIT 1",
        )
        .bind(freeze_id)
        .fetch_optional(&mut *tx)
        .await?;
        if latest
            .as_ref()
            .is_some_and(|record| record.status == "confirmed")
        {
            tx.rollback().await?;
            return Ok(true);
        }
        return Err(AppError::conflict("freeze is not pending"));
    }

    sqlx::query(
        "UPDATE user_balances SET frozenAmount = frozenAmount - ?, totalSpent = totalSpent + ?, balance = balance + ?, updatedAt = NOW(3) WHERE userId = ?",
    )
    .bind(freeze.amount)
    .bind(charged_amount)
    .bind(refund_amount)
    .bind(&freeze.user_id)
    .execute(&mut *tx)
    .await?;

    let balance = sqlx::query_as::<_, BalanceSnapshot>(
        "SELECT id, userId, balance, frozenAmount, totalSpent, createdAt, updatedAt FROM user_balances WHERE userId = ? LIMIT 1",
    )
    .bind(&freeze.user_id)
    .fetch_one(&mut *tx)
    .await?;

    if charged_amount > Decimal::ZERO {
        let metadata_raw = input.metadata.as_ref().map(Value::to_string);
        let description = if is_project_scoped(&input.project_id) {
            format!("{} - {}", input.action, input.model)
        } else {
            format!("{} - {} (Asset Hub)", input.action, input.model)
        };

        if is_project_scoped(&input.project_id) {
            sqlx::query(
                "INSERT INTO usage_costs (id, projectId, userId, apiType, model, action, quantity, unit, cost, metadata, createdAt) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3))",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(input.project_id.trim())
            .bind(&freeze.user_id)
            .bind(input.api_type.as_str())
            .bind(&input.model)
            .bind(&input.action)
            .bind(input.quantity.max(0.0).round() as i64)
            .bind(input.unit.as_str())
            .bind(charged_amount)
            .bind(metadata_raw.clone())
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            "INSERT INTO balance_transactions (id, userId, type, amount, balanceAfter, description, relatedId, freezeId, projectId, episodeId, taskType, billingMeta, createdAt) VALUES (?, ?, 'consume', ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3))",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&freeze.user_id)
        .bind(-charged_amount)
        .bind(balance.balance)
        .bind(description)
        .bind(Some(freeze.id.clone()))
        .bind(Some(freeze.id.clone()))
        .bind(trim_or_none(Some(input.project_id.clone())))
        .bind(trim_or_none(input.episode_id.clone()))
        .bind(trim_or_none(input.task_type.clone()))
        .bind(build_billing_meta(input, charged_amount))
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(true)
}

pub async fn rollback_freeze(pool: &MySqlPool, freeze_id: &str) -> Result<bool, AppError> {
    let mut tx = pool.begin().await?;

    let freeze = sqlx::query_as::<_, FreezeRow>(
        "SELECT id, userId, amount, status FROM balance_freezes WHERE id = ? LIMIT 1 FOR UPDATE",
    )
    .bind(freeze_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::invalid_params("invalid freeze record"))?;

    if freeze.status == "rolled_back" {
        tx.rollback().await?;
        return Ok(true);
    }
    if freeze.status != "pending" {
        return Err(AppError::conflict("freeze is not pending"));
    }

    let switched = sqlx::query(
        "UPDATE balance_freezes SET status = 'rolled_back', updatedAt = NOW(3) WHERE id = ? AND status = 'pending'",
    )
    .bind(freeze_id)
    .execute(&mut *tx)
    .await?;

    if switched.rows_affected() == 0 {
        let latest = sqlx::query_as::<_, FreezeRow>(
            "SELECT id, userId, amount, status FROM balance_freezes WHERE id = ? LIMIT 1",
        )
        .bind(freeze_id)
        .fetch_optional(&mut *tx)
        .await?;

        if latest
            .as_ref()
            .is_some_and(|record| record.status == "rolled_back")
        {
            tx.rollback().await?;
            return Ok(true);
        }

        return Err(AppError::conflict("freeze is not pending"));
    }

    sqlx::query(
        "UPDATE user_balances SET balance = balance + ?, frozenAmount = frozenAmount - ?, updatedAt = NOW(3) WHERE userId = ?",
    )
    .bind(freeze.amount)
    .bind(freeze.amount)
    .bind(freeze.user_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(true)
}

pub async fn increase_pending_freeze_amount(
    pool: &MySqlPool,
    freeze_id: &str,
    delta: f64,
) -> Result<bool, AppError> {
    let normalized_delta = parse_amount(delta)?;
    if normalized_delta == Decimal::ZERO {
        return Ok(true);
    }

    let mut tx = pool.begin().await?;
    let freeze = sqlx::query_as::<_, FreezeRow>(
        "SELECT id, userId, amount, status FROM balance_freezes WHERE id = ? LIMIT 1 FOR UPDATE",
    )
    .bind(freeze_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::invalid_params("invalid freeze record"))?;

    if freeze.status == "confirmed" {
        tx.rollback().await?;
        return Ok(true);
    }
    if freeze.status != "pending" {
        return Err(AppError::conflict("freeze is not pending"));
    }

    let updated = sqlx::query(
        "UPDATE user_balances SET balance = balance - ?, frozenAmount = frozenAmount + ?, updatedAt = NOW(3) WHERE userId = ? AND balance >= ?",
    )
    .bind(normalized_delta)
    .bind(normalized_delta)
    .bind(&freeze.user_id)
    .bind(normalized_delta)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() == 0 {
        tx.rollback().await?;
        return Ok(false);
    }

    let switched = sqlx::query(
        "UPDATE balance_freezes SET amount = amount + ?, updatedAt = NOW(3) WHERE id = ? AND status = 'pending'",
    )
    .bind(normalized_delta)
    .bind(freeze_id)
    .execute(&mut *tx)
    .await?;
    if switched.rows_affected() == 0 {
        return Err(AppError::conflict("freeze is not pending"));
    }

    tx.commit().await?;
    Ok(true)
}

pub async fn record_shadow_usage(
    pool: &MySqlPool,
    user_id: &str,
    input: &ConfirmChargeInput,
) -> Result<bool, AppError> {
    ensure_user_balance_row(pool, user_id).await?;
    let balance = get_balance(pool, user_id).await?;

    let cost = input.charged_amount.unwrap_or(0.0).max(0.0);
    let metadata = input
        .metadata
        .as_ref()
        .map(Value::to_string)
        .unwrap_or_default();
    let description = format!(
        "[SHADOW] {} - {} - ¥{:.4}{}",
        input.action,
        input.model,
        cost,
        if metadata.is_empty() {
            String::new()
        } else {
            format!(" | {metadata}")
        }
    );

    sqlx::query(
        "INSERT INTO balance_transactions (id, userId, type, amount, balanceAfter, description, projectId, episodeId, taskType, billingMeta, createdAt) VALUES (?, ?, 'shadow_consume', 0, ?, ?, ?, ?, ?, ?, NOW(3))",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(balance.balance)
    .bind(description)
    .bind(trim_or_none(Some(input.project_id.clone())))
    .bind(trim_or_none(input.episode_id.clone()))
    .bind(trim_or_none(input.task_type.clone()))
    .bind(build_billing_meta(input, parse_amount(cost).unwrap_or(Decimal::ZERO)))
    .execute(pool)
    .await?;

    Ok(true)
}

pub async fn add_balance(
    pool: &MySqlPool,
    user_id: &str,
    amount: f64,
    options: Option<AddBalanceOptions>,
) -> Result<bool, AppError> {
    let amount = parse_amount(amount)?;
    if amount <= Decimal::ZERO {
        return Err(AppError::invalid_params("amount must be greater than zero"));
    }

    let tx_type = options
        .as_ref()
        .and_then(|item| item.tx_type.clone())
        .and_then(|item| trim_or_none(Some(item)))
        .unwrap_or_else(|| "recharge".to_string());

    let mut tx = pool.begin().await?;

    if let Some(key) = options
        .as_ref()
        .and_then(|item| item.idempotency_key.as_deref())
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM balance_transactions WHERE userId = ? AND type = ? AND idempotencyKey = ?",
        )
        .bind(user_id)
        .bind(&tx_type)
        .bind(key)
        .fetch_one(&mut *tx)
        .await?;
        if existing > 0 {
            tx.rollback().await?;
            return Ok(true);
        }
    }

    let row_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO user_balances (id, userId, balance, frozenAmount, totalSpent, createdAt, updatedAt) VALUES (?, ?, ?, 0, 0, NOW(3), NOW(3)) ON DUPLICATE KEY UPDATE balance = balance + VALUES(balance), updatedAt = NOW(3)",
    )
    .bind(row_id)
    .bind(user_id)
    .bind(amount)
    .execute(&mut *tx)
    .await?;

    let balance = sqlx::query_as::<_, BalanceSnapshot>(
        "SELECT id, userId, balance, frozenAmount, totalSpent, createdAt, updatedAt FROM user_balances WHERE userId = ? LIMIT 1",
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    let reason = options
        .as_ref()
        .and_then(|item| item.reason.as_deref())
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .unwrap_or("balance recharge")
        .to_string();

    let audit = json!({
        "reason": options.as_ref().and_then(|item| item.reason.clone()),
        "operatorId": options.as_ref().and_then(|item| item.operator_id.clone()),
        "externalOrderId": options.as_ref().and_then(|item| item.external_order_id.clone()),
        "idempotencyKey": options.as_ref().and_then(|item| item.idempotency_key.clone()),
    });

    sqlx::query(
        "INSERT INTO balance_transactions (id, userId, type, amount, balanceAfter, description, operatorId, externalOrderId, idempotencyKey, createdAt) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3))",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(tx_type)
    .bind(amount)
    .bind(balance.balance)
    .bind(format!("{reason} | audit={audit}"))
    .bind(options.as_ref().and_then(|item| item.operator_id.clone()))
    .bind(options.as_ref().and_then(|item| item.external_order_id.clone()))
    .bind(options.as_ref().and_then(|item| item.idempotency_key.clone()))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(true)
}

pub fn insufficient_balance_error(required: Decimal, available: Decimal) -> AppError {
    AppError::new(
        ErrorCode::InsufficientBalance,
        format!(
            "insufficient balance: required {:.4}, available {:.4}",
            decimal_to_f64(required),
            decimal_to_f64(available)
        ),
    )
    .with_details(json!({
        "required": decimal_to_f64(required),
        "available": decimal_to_f64(available),
    }))
}

pub fn now_millis_iso() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
