use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "text" => Some(Self::Text),
            "image" => Some(Self::Image),
            "video" => Some(Self::Video),
            "voice" => Some(Self::Voice),
            "voice-design" => Some(Self::VoiceDesign),
            "lip-sync" => Some(Self::LipSync),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum UsageUnit {
    Token,
    InputToken,
    OutputToken,
    Image,
    Video,
    Second,
    Call,
}

impl UsageUnit {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Token => "token",
            Self::InputToken => "input_token",
            Self::OutputToken => "output_token",
            Self::Image => "image",
            Self::Video => "video",
            Self::Second => "second",
            Self::Call => "call",
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BalanceSnapshot {
    pub id: String,
    #[sqlx(rename = "userId")]
    pub user_id: String,
    pub balance: Decimal,
    #[sqlx(rename = "totalSpent")]
    pub total_spent: Decimal,
    #[sqlx(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[sqlx(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ModelPrice {
    pub api_type: String,
    pub model_id: String,
    pub unit: String,
    pub unit_price: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeductRequest {
    pub task_id: String,
    pub user_id: String,
    pub project_id: String,
    pub episode_id: Option<String>,
    pub api_type: String,
    pub model: String,
    pub action: String,
    pub quantity: Decimal,
    pub unit: String,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CreditRecordType {
    Consume,
    Recharge,
    Refund,
    AdminAdjust,
}

impl CreditRecordType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Consume => "consume",
            Self::Recharge => "recharge",
            Self::Refund => "refund",
            Self::AdminAdjust => "admin_adjust",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "consume" => Some(Self::Consume),
            "recharge" => Some(Self::Recharge),
            "refund" => Some(Self::Refund),
            "admin_adjust" => Some(Self::AdminAdjust),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditRecord {
    pub id: String,
    pub user_id: String,
    #[serde(rename = "type")]
    pub record_type: CreditRecordType,
    pub amount: Decimal,
    pub balance_after: Decimal,
    pub api_type: Option<String>,
    pub model: Option<String>,
    pub action: Option<String>,
    pub quantity: Option<Decimal>,
    pub unit: Option<String>,
    pub unit_price: Option<Decimal>,
    pub project_id: Option<String>,
    pub episode_id: Option<String>,
    pub task_id: Option<String>,
    pub metadata: Option<Value>,
    pub created_at: NaiveDateTime,
}
