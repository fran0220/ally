use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum BillingMode {
    #[default]
    Off,
    Shadow,
    Enforce,
}

impl BillingMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::Shadow => "SHADOW",
            Self::Enforce => "ENFORCE",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BillingApiType {
    Text,
    Image,
    Video,
    Voice,
    VoiceDesign,
    LipSync,
}

impl BillingApiType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::Video => "video",
            Self::Voice => "voice",
            Self::VoiceDesign => "voice-design",
            Self::LipSync => "lip-sync",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UsageUnit {
    Token,
    Image,
    Video,
    Second,
    Call,
}

impl UsageUnit {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Token => "token",
            Self::Image => "image",
            Self::Video => "video",
            Self::Second => "second",
            Self::Call => "call",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BillingStatus {
    Skipped,
    Quoted,
    Frozen,
    Settled,
    RolledBack,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskBillingInfo {
    pub billable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_type: Option<BillingApiType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<UsageUnit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_frozen_cost: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pricing_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billing_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freeze_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode_snapshot: Option<BillingMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<BillingStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charged_cost: Option<f64>,
}

impl TaskBillingInfo {
    pub fn billable(
        task_type: impl Into<String>,
        api_type: BillingApiType,
        model: impl Into<String>,
        quantity: f64,
        unit: UsageUnit,
    ) -> Self {
        Self {
            billable: true,
            source: Some("task".to_string()),
            task_type: Some(task_type.into()),
            api_type: Some(api_type),
            model: Some(model.into()),
            quantity: Some(quantity),
            unit: Some(unit),
            max_frozen_cost: None,
            pricing_version: None,
            action: None,
            metadata: None,
            billing_key: None,
            freeze_id: None,
            mode_snapshot: None,
            status: Some(BillingStatus::Quoted),
            charged_cost: None,
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BalanceSnapshot {
    pub id: String,
    #[sqlx(rename = "userId")]
    pub user_id: String,
    pub balance: Decimal,
    #[sqlx(rename = "frozenAmount")]
    pub frozen_amount: Decimal,
    #[sqlx(rename = "totalSpent")]
    pub total_spent: Decimal,
    #[sqlx(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FreezeSnapshot {
    pub id: String,
    #[sqlx(rename = "userId")]
    pub user_id: String,
    pub amount: Decimal,
    pub status: String,
}
