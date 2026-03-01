use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const RUN_STATE_MAX_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Canceling,
    Canceled,
}

impl RunStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Canceling => "canceling",
            Self::Canceled => "canceled",
        }
    }

    pub fn from_db(raw: &str) -> Option<Self> {
        match raw {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "canceling" => Some(Self::Canceling),
            "canceled" => Some(Self::Canceled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunStepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Canceled,
}

impl RunStepStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
        }
    }

    pub fn from_db(raw: &str) -> Option<Self> {
        match raw {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "canceled" => Some(Self::Canceled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunEventType {
    #[serde(rename = "run.start")]
    RunStart,
    #[serde(rename = "step.start")]
    StepStart,
    #[serde(rename = "step.chunk")]
    StepChunk,
    #[serde(rename = "step.complete")]
    StepComplete,
    #[serde(rename = "step.error")]
    StepError,
    #[serde(rename = "run.complete")]
    RunComplete,
    #[serde(rename = "run.error")]
    RunError,
    #[serde(rename = "run.canceled")]
    RunCanceled,
}

impl RunEventType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RunStart => "run.start",
            Self::StepStart => "step.start",
            Self::StepChunk => "step.chunk",
            Self::StepComplete => "step.complete",
            Self::StepError => "step.error",
            Self::RunComplete => "run.complete",
            Self::RunError => "run.error",
            Self::RunCanceled => "run.canceled",
        }
    }

    pub fn from_db(raw: &str) -> Option<Self> {
        match raw {
            "run.start" => Some(Self::RunStart),
            "step.start" => Some(Self::StepStart),
            "step.chunk" => Some(Self::StepChunk),
            "step.complete" => Some(Self::StepComplete),
            "step.error" => Some(Self::StepError),
            "run.complete" => Some(Self::RunComplete),
            "run.error" => Some(Self::RunError),
            "run.canceled" => Some(Self::RunCanceled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunEventLane {
    Text,
    Reasoning,
}

impl RunEventLane {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Reasoning => "reasoning",
        }
    }

    pub fn from_db(raw: &str) -> Option<Self> {
        match raw {
            "text" => Some(Self::Text),
            "reasoning" => Some(Self::Reasoning),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEventInput {
    pub run_id: String,
    pub project_id: String,
    pub user_id: String,
    pub event_type: RunEventType,
    pub step_key: Option<String>,
    pub attempt: Option<i32>,
    pub lane: Option<RunEventLane>,
    pub payload: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEvent {
    pub id: String,
    pub run_id: String,
    pub project_id: String,
    pub user_id: String,
    pub seq: i32,
    pub event_type: RunEventType,
    pub step_key: Option<String>,
    pub attempt: Option<i32>,
    pub lane: Option<RunEventLane>,
    pub payload: Option<Value>,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRunInput {
    pub user_id: String,
    pub project_id: String,
    pub episode_id: Option<String>,
    pub workflow_type: String,
    pub task_type: Option<String>,
    pub task_id: Option<String>,
    pub target_type: String,
    pub target_id: String,
    pub input: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRunsInput {
    pub user_id: String,
    pub project_id: Option<String>,
    pub workflow_type: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub episode_id: Option<String>,
    pub statuses: Vec<RunStatus>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateRef {
    pub script_id: Option<String>,
    pub storyboard_id: Option<String>,
    pub voice_line_batch_id: Option<String>,
    pub version_hash: Option<String>,
    pub cursor: Option<String>,
}
