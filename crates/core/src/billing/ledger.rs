use rust_decimal::Decimal;
use serde_json::{Map, Value, json};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::errors::{AppError, ErrorCode};

use super::{
    money::{decimal_from_f64, decimal_to_f64, normalize_money},
    pricing::get_unit_price,
    types::{BalanceSnapshot, CreditRecord, CreditRecordType, DeductRequest, UsageUnit},
};

#[derive(Debug, Clone, Default)]
pub struct AddCreditsOptions {
    pub reason: Option<String>,
    pub operator_id: Option<String>,
    pub external_order_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub record_type: Option<CreditRecordType>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct CreditRecordRow {
    id: String,
    user_id: String,
    #[sqlx(rename = "record_type")]
    record_type: String,
    amount: Decimal,
    balance_after: Decimal,
    api_type: Option<String>,
    model: Option<String>,
    action: Option<String>,
    quantity: Option<Decimal>,
    unit: Option<String>,
    unit_price: Option<Decimal>,
    project_id: Option<String>,
    episode_id: Option<String>,
    task_id: Option<String>,
    metadata: Option<sqlx::types::Json<Value>>,
    created_at: chrono::NaiveDateTime,
}

fn trim_or_none(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn is_duplicate_key_error(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(db_error) => {
            db_error.is_unique_violation() || db_error.code().is_some_and(|code| code == "1062")
        }
        _ => false,
    }
}

fn map_record_type(raw: &str) -> Result<CreditRecordType, AppError> {
    CreditRecordType::parse(raw)
        .ok_or_else(|| AppError::internal(format!("invalid credit record type: {raw}")))
}

fn map_credit_record(row: CreditRecordRow) -> Result<CreditRecord, AppError> {
    Ok(CreditRecord {
        id: row.id,
        user_id: row.user_id,
        record_type: map_record_type(&row.record_type)?,
        amount: row.amount,
        balance_after: row.balance_after,
        api_type: row.api_type,
        model: row.model,
        action: row.action,
        quantity: row.quantity,
        unit: row.unit,
        unit_price: row.unit_price,
        project_id: row.project_id,
        episode_id: row.episode_id,
        task_id: row.task_id,
        metadata: row.metadata.map(|value| value.0),
        created_at: row.created_at,
    })
}

fn read_decimal_value(value: Option<&Value>) -> Option<Decimal> {
    let value = value?;
    if let Some(raw) = value.as_str() {
        return raw.trim().parse::<Decimal>().ok().map(normalize_money);
    }
    value
        .as_f64()
        .and_then(decimal_from_f64)
        .map(normalize_money)
}

fn parse_text_breakdown(
    metadata: Option<&Value>,
    fallback_total: Decimal,
) -> Result<(Decimal, Decimal), AppError> {
    let (input_tokens, output_tokens) = if let Some(Value::Object(map)) = metadata {
        (
            read_decimal_value(map.get("inputTokens")).unwrap_or(Decimal::ZERO),
            read_decimal_value(map.get("outputTokens")).unwrap_or(Decimal::ZERO),
        )
    } else {
        (fallback_total, Decimal::ZERO)
    };

    let input_tokens = normalize_money(input_tokens.max(Decimal::ZERO));
    let output_tokens = normalize_money(output_tokens.max(Decimal::ZERO));
    if input_tokens + output_tokens <= Decimal::ZERO {
        return Err(AppError::invalid_params(
            "text billing requires positive input/output tokens",
        ));
    }

    Ok((input_tokens, output_tokens))
}

fn sanitize_positive_quantity(quantity: Decimal) -> Result<Decimal, AppError> {
    let quantity = normalize_money(quantity);
    if quantity <= Decimal::ZERO {
        return Err(AppError::invalid_params(
            "billing quantity must be greater than zero",
        ));
    }
    Ok(quantity)
}

fn normalize_metadata(metadata: Option<Value>) -> Option<Value> {
    match metadata {
        Some(Value::Object(map)) if map.is_empty() => None,
        Some(value) => Some(value),
        None => None,
    }
}

async fn get_credit_record_by_id(
    pool: &MySqlPool,
    record_id: &str,
) -> Result<CreditRecord, AppError> {
    let row = sqlx::query_as::<_, CreditRecordRow>(
        "SELECT id, user_id, type AS record_type, amount, balance_after, api_type, model, action, quantity, unit, unit_price, project_id, episode_id, task_id, metadata, created_at FROM credit_records WHERE id = ? LIMIT 1",
    )
    .bind(record_id)
    .fetch_one(pool)
    .await?;

    map_credit_record(row)
}

async fn get_credit_record_by_task_and_type(
    pool: &MySqlPool,
    task_id: &str,
    record_type: CreditRecordType,
) -> Result<Option<CreditRecord>, AppError> {
    let task_id = task_id.trim();
    if task_id.is_empty() {
        return Ok(None);
    }

    let row = sqlx::query_as::<_, CreditRecordRow>(
        "SELECT id, user_id, type AS record_type, amount, balance_after, api_type, model, action, quantity, unit, unit_price, project_id, episode_id, task_id, metadata, created_at FROM credit_records WHERE task_id = ? AND type = ? LIMIT 1",
    )
    .bind(task_id)
    .bind(record_type.as_str())
    .fetch_optional(pool)
    .await?;

    row.map(map_credit_record).transpose()
}

async fn get_credit_record_by_idempotency(
    pool: &MySqlPool,
    user_id: &str,
    record_type: CreditRecordType,
    idempotency_key: &str,
) -> Result<Option<CreditRecord>, AppError> {
    let key = idempotency_key.trim();
    if key.is_empty() {
        return Ok(None);
    }

    let row = sqlx::query_as::<_, CreditRecordRow>(
        "SELECT id, user_id, type AS record_type, amount, balance_after, api_type, model, action, quantity, unit, unit_price, project_id, episode_id, task_id, metadata, created_at FROM credit_records WHERE user_id = ? AND type = ? AND idempotency_key = ? LIMIT 1",
    )
    .bind(user_id)
    .bind(record_type.as_str())
    .bind(key)
    .fetch_optional(pool)
    .await?;

    row.map(map_credit_record).transpose()
}

async fn ensure_user_balance_row_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    user_id: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO user_balances (id, userId, balance, totalSpent, createdAt, updatedAt) VALUES (?, ?, 0, 0, NOW(3), NOW(3)) ON DUPLICATE KEY UPDATE updatedAt = updatedAt",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn ensure_user_balance_row(pool: &MySqlPool, user_id: &str) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO user_balances (id, userId, balance, totalSpent, createdAt, updatedAt) VALUES (?, ?, 0, 0, NOW(3), NOW(3)) ON DUPLICATE KEY UPDATE updatedAt = updatedAt",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_balance(pool: &MySqlPool, user_id: &str) -> Result<BalanceSnapshot, AppError> {
    ensure_user_balance_row(pool, user_id).await?;

    sqlx::query_as::<_, BalanceSnapshot>(
        "SELECT id, userId, balance, totalSpent, createdAt, updatedAt FROM user_balances WHERE userId = ? LIMIT 1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn check_balance(
    pool: &MySqlPool,
    user_id: &str,
    required_amount: f64,
) -> Result<bool, AppError> {
    let required = decimal_from_f64(required_amount)
        .ok_or_else(|| AppError::invalid_params("required amount must be a finite number"))?;
    let required = normalize_money(required);
    if required <= Decimal::ZERO {
        return Ok(true);
    }

    let balance = get_balance(pool, user_id).await?;
    Ok(balance.balance >= required)
}

async fn resolve_non_text_price(
    pool: &MySqlPool,
    api_type: &str,
    model: &str,
    requested_unit: &str,
) -> Result<(String, Decimal), AppError> {
    let requested_unit = requested_unit.trim();
    if requested_unit.is_empty() {
        return Err(AppError::invalid_params("billing unit is required"));
    }

    let mut candidates = vec![requested_unit.to_string()];
    if let Some((base_unit, _)) = requested_unit.split_once(':')
        && !base_unit.trim().is_empty()
    {
        candidates.push(base_unit.trim().to_string());
    }

    candidates.sort();
    candidates.dedup();

    let mut last_not_found: Option<AppError> = None;
    for unit in candidates {
        match get_unit_price(pool, api_type, model, &unit).await {
            Ok(price) => return Ok((price.unit, normalize_money(price.unit_price))),
            Err(error) if error.code.as_str() == ErrorCode::NotFound.as_str() => {
                last_not_found = Some(error);
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_not_found.unwrap_or_else(|| {
        AppError::not_found(format!(
            "billing pricing not found for api_type={api_type}, model={model}, unit={requested_unit}",
        ))
    }))
}

pub async fn deduct_credits(
    pool: &MySqlPool,
    request: &DeductRequest,
) -> Result<CreditRecord, AppError> {
    let task_id = request.task_id.trim();
    let user_id = request.user_id.trim();
    let project_id = request.project_id.trim();
    let api_type = request.api_type.trim();
    let action = request.action.trim();
    let model = request.model.trim();

    if task_id.is_empty()
        || user_id.is_empty()
        || project_id.is_empty()
        || api_type.is_empty()
        || action.is_empty()
        || model.is_empty()
    {
        return Err(AppError::invalid_params(
            "task_id, user_id, project_id, api_type, action and model are required",
        ));
    }

    if let Some(existing) =
        get_credit_record_by_task_and_type(pool, task_id, CreditRecordType::Consume).await?
    {
        return Ok(existing);
    }

    let requested_quantity = sanitize_positive_quantity(request.quantity)?;
    let episode_id = trim_or_none(request.episode_id.clone());

    let (quantity, unit, unit_price, amount, metadata) = if api_type == "text" {
        let (input_tokens, output_tokens) =
            parse_text_breakdown(request.metadata.as_ref(), requested_quantity)?;

        let input_price =
            get_unit_price(pool, api_type, model, UsageUnit::InputToken.as_str()).await?;
        let output_price =
            get_unit_price(pool, api_type, model, UsageUnit::OutputToken.as_str()).await?;

        let amount = normalize_money(
            (input_tokens * input_price.unit_price) + (output_tokens * output_price.unit_price),
        );
        if amount <= Decimal::ZERO {
            return Err(AppError::invalid_params(
                "calculated billing amount must be positive",
            ));
        }

        let quantity = normalize_money(input_tokens + output_tokens);
        let unit_price = if quantity > Decimal::ZERO {
            normalize_money(amount / quantity)
        } else {
            Decimal::ZERO
        };

        let mut metadata_map = request
            .metadata
            .as_ref()
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        metadata_map.insert(
            "inputTokens".to_string(),
            Value::String(input_tokens.to_string()),
        );
        metadata_map.insert(
            "outputTokens".to_string(),
            Value::String(output_tokens.to_string()),
        );

        (
            quantity,
            UsageUnit::Token.as_str().to_string(),
            unit_price,
            amount,
            normalize_metadata(Some(Value::Object(metadata_map))),
        )
    } else {
        let (resolved_unit, resolved_unit_price) =
            resolve_non_text_price(pool, api_type, model, &request.unit).await?;
        let amount = normalize_money(resolved_unit_price * requested_quantity);
        if amount <= Decimal::ZERO {
            return Err(AppError::invalid_params(
                "calculated billing amount must be positive",
            ));
        }

        (
            requested_quantity,
            resolved_unit,
            resolved_unit_price,
            amount,
            normalize_metadata(request.metadata.clone()),
        )
    };

    let mut tx = pool.begin().await?;
    ensure_user_balance_row_tx(&mut tx, user_id).await?;

    if let Some(existing_row) = sqlx::query_as::<_, CreditRecordRow>(
        "SELECT id, user_id, type AS record_type, amount, balance_after, api_type, model, action, quantity, unit, unit_price, project_id, episode_id, task_id, metadata, created_at FROM credit_records WHERE task_id = ? AND type = 'consume' LIMIT 1 FOR UPDATE",
    )
    .bind(task_id)
    .fetch_optional(&mut *tx)
    .await?
    {
        tx.rollback().await?;
        return map_credit_record(existing_row);
    }

    let updated = sqlx::query(
        "UPDATE user_balances SET balance = balance - ?, totalSpent = totalSpent + ?, updatedAt = NOW(3) WHERE userId = ? AND balance >= ?",
    )
    .bind(amount)
    .bind(amount)
    .bind(user_id)
    .bind(amount)
    .execute(&mut *tx)
    .await?;

    if updated.rows_affected() == 0 {
        let available = sqlx::query_scalar::<_, Decimal>(
            "SELECT balance FROM user_balances WHERE userId = ? LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?
        .unwrap_or(Decimal::ZERO);

        tx.rollback().await?;
        return Err(insufficient_balance_error(amount, available));
    }

    let balance_after = sqlx::query_scalar::<_, Decimal>(
        "SELECT balance FROM user_balances WHERE userId = ? LIMIT 1",
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    let record_id = Uuid::new_v4().to_string();
    let inserted = sqlx::query(
        "INSERT INTO credit_records (id, user_id, type, amount, balance_after, api_type, model, action, quantity, unit, unit_price, project_id, episode_id, task_id, metadata, created_at) VALUES (?, ?, 'consume', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3))",
    )
    .bind(&record_id)
    .bind(user_id)
    .bind(amount)
    .bind(balance_after)
    .bind(api_type)
    .bind(model)
    .bind(action)
    .bind(quantity)
    .bind(&unit)
    .bind(unit_price)
    .bind(project_id)
    .bind(episode_id)
    .bind(task_id)
    .bind(metadata.map(sqlx::types::Json))
    .execute(&mut *tx)
    .await;

    if let Err(error) = inserted {
        if is_duplicate_key_error(&error)
            && let Some(existing) =
                get_credit_record_by_task_and_type(pool, task_id, CreditRecordType::Consume).await?
        {
            tx.rollback().await?;
            return Ok(existing);
        }
        tx.rollback().await?;
        return Err(error.into());
    }

    tx.commit().await?;
    get_credit_record_by_id(pool, &record_id).await
}

pub async fn add_credits(
    pool: &MySqlPool,
    user_id: &str,
    amount: Decimal,
    options: Option<AddCreditsOptions>,
) -> Result<CreditRecord, AppError> {
    let amount = sanitize_positive_quantity(amount)?;
    let record_type = options
        .as_ref()
        .and_then(|item| item.record_type)
        .unwrap_or(CreditRecordType::Recharge);

    if let Some(key) = options
        .as_ref()
        .and_then(|item| item.idempotency_key.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        && let Some(existing) =
            get_credit_record_by_idempotency(pool, user_id, record_type, key).await?
    {
        return Ok(existing);
    }

    let reason = options
        .as_ref()
        .and_then(|item| trim_or_none(item.reason.clone()))
        .unwrap_or_else(|| "balance recharge".to_string());

    let mut metadata = options
        .as_ref()
        .and_then(|item| item.metadata.clone())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    metadata.insert("reason".to_string(), Value::String(reason));

    let mut tx = pool.begin().await?;
    ensure_user_balance_row_tx(&mut tx, user_id).await?;

    if let Some(key) = options
        .as_ref()
        .and_then(|item| item.idempotency_key.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if let Some(existing_row) = sqlx::query_as::<_, CreditRecordRow>(
            "SELECT id, user_id, type AS record_type, amount, balance_after, api_type, model, action, quantity, unit, unit_price, project_id, episode_id, task_id, metadata, created_at FROM credit_records WHERE user_id = ? AND type = ? AND idempotency_key = ? LIMIT 1 FOR UPDATE",
        )
        .bind(user_id)
        .bind(record_type.as_str())
        .bind(key)
        .fetch_optional(&mut *tx)
        .await?
        {
            tx.rollback().await?;
            return map_credit_record(existing_row);
        }
    }

    sqlx::query(
        "UPDATE user_balances SET balance = balance + ?, updatedAt = NOW(3) WHERE userId = ?",
    )
    .bind(amount)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    let balance_after = sqlx::query_scalar::<_, Decimal>(
        "SELECT balance FROM user_balances WHERE userId = ? LIMIT 1",
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    let record_id = Uuid::new_v4().to_string();
    let inserted = sqlx::query(
        "INSERT INTO credit_records (id, user_id, type, amount, balance_after, operator_id, external_order_id, idempotency_key, metadata, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3))",
    )
    .bind(&record_id)
    .bind(user_id)
    .bind(record_type.as_str())
    .bind(amount)
    .bind(balance_after)
    .bind(options.as_ref().and_then(|item| trim_or_none(item.operator_id.clone())))
    .bind(options.as_ref().and_then(|item| trim_or_none(item.external_order_id.clone())))
    .bind(options.as_ref().and_then(|item| trim_or_none(item.idempotency_key.clone())))
    .bind(normalize_metadata(Some(Value::Object(metadata))).map(sqlx::types::Json))
    .execute(&mut *tx)
    .await;

    if let Err(error) = inserted {
        if is_duplicate_key_error(&error)
            && let Some(key) = options
                .as_ref()
                .and_then(|item| item.idempotency_key.as_deref())
                .map(str::trim)
                .filter(|value| !value.is_empty())
            && let Some(existing) =
                get_credit_record_by_idempotency(pool, user_id, record_type, key).await?
        {
            tx.rollback().await?;
            return Ok(existing);
        }
        tx.rollback().await?;
        return Err(error.into());
    }

    tx.commit().await?;
    get_credit_record_by_id(pool, &record_id).await
}

pub async fn refund_credits(
    pool: &MySqlPool,
    task_id: &str,
) -> Result<Option<CreditRecord>, AppError> {
    let task_id = task_id.trim();
    if task_id.is_empty() {
        return Err(AppError::invalid_params("task_id is required for refund"));
    }

    let mut tx = pool.begin().await?;
    let consume = sqlx::query_as::<_, CreditRecordRow>(
        "SELECT id, user_id, type AS record_type, amount, balance_after, api_type, model, action, quantity, unit, unit_price, project_id, episode_id, task_id, metadata, created_at FROM credit_records WHERE task_id = ? AND type = 'consume' LIMIT 1 FOR UPDATE",
    )
    .bind(task_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(consume) = consume else {
        tx.rollback().await?;
        return Ok(None);
    };

    if let Some(existing_refund) = sqlx::query_as::<_, CreditRecordRow>(
        "SELECT id, user_id, type AS record_type, amount, balance_after, api_type, model, action, quantity, unit, unit_price, project_id, episode_id, task_id, metadata, created_at FROM credit_records WHERE task_id = ? AND type = 'refund' LIMIT 1 FOR UPDATE",
    )
    .bind(task_id)
    .fetch_optional(&mut *tx)
    .await?
    {
        tx.rollback().await?;
        return map_credit_record(existing_refund).map(Some);
    }

    sqlx::query(
        "UPDATE user_balances SET balance = balance + ?, totalSpent = CASE WHEN totalSpent >= ? THEN totalSpent - ? ELSE 0 END, updatedAt = NOW(3) WHERE userId = ?",
    )
    .bind(consume.amount)
    .bind(consume.amount)
    .bind(consume.amount)
    .bind(&consume.user_id)
    .execute(&mut *tx)
    .await?;

    let balance_after = sqlx::query_scalar::<_, Decimal>(
        "SELECT balance FROM user_balances WHERE userId = ? LIMIT 1",
    )
    .bind(&consume.user_id)
    .fetch_one(&mut *tx)
    .await?;

    let mut metadata = Map::new();
    metadata.insert(
        "sourceRecordId".to_string(),
        Value::String(consume.id.clone()),
    );
    metadata.insert(
        "reason".to_string(),
        Value::String("task_failed".to_string()),
    );
    if let Some(source_metadata) = consume.metadata.as_ref().map(|value| value.0.clone()) {
        metadata.insert("sourceMetadata".to_string(), source_metadata);
    }

    let record_id = Uuid::new_v4().to_string();
    let inserted = sqlx::query(
        "INSERT INTO credit_records (id, user_id, type, amount, balance_after, api_type, model, action, quantity, unit, unit_price, project_id, episode_id, task_id, metadata, created_at) VALUES (?, ?, 'refund', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(3))",
    )
    .bind(&record_id)
    .bind(&consume.user_id)
    .bind(consume.amount)
    .bind(balance_after)
    .bind(consume.api_type)
    .bind(consume.model)
    .bind(consume.action)
    .bind(consume.quantity)
    .bind(consume.unit)
    .bind(consume.unit_price)
    .bind(consume.project_id)
    .bind(consume.episode_id)
    .bind(consume.task_id)
    .bind(Some(sqlx::types::Json(Value::Object(metadata))))
    .execute(&mut *tx)
    .await;

    if let Err(error) = inserted {
        if is_duplicate_key_error(&error)
            && let Some(existing) =
                get_credit_record_by_task_and_type(pool, task_id, CreditRecordType::Refund).await?
        {
            tx.rollback().await?;
            return Ok(Some(existing));
        }
        tx.rollback().await?;
        return Err(error.into());
    }

    tx.commit().await?;
    get_credit_record_by_id(pool, &record_id).await.map(Some)
}

pub fn insufficient_balance_error(required: Decimal, available: Decimal) -> AppError {
    AppError::new(
        ErrorCode::InsufficientBalance,
        format!(
            "insufficient balance: required {:.6}, available {:.6}",
            decimal_to_f64(required),
            decimal_to_f64(available)
        ),
    )
    .with_details(json!({
        "required": decimal_to_f64(required),
        "available": decimal_to_f64(available),
    }))
}
