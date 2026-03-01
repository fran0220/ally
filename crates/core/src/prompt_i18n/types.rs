use std::{collections::HashMap, str::FromStr};

use serde::{Deserialize, Serialize};

use super::PromptId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptLocale {
    Zh,
    En,
}

impl PromptLocale {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Zh => "zh",
            Self::En => "en",
        }
    }
}

impl FromStr for PromptLocale {
    type Err = String;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let normalized = raw.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "zh" | "zh-cn" | "zh_hans" | "zh-hans" => Ok(Self::Zh),
            "en" | "en-us" => Ok(Self::En),
            _ => Err(format!("unsupported prompt locale: {raw}")),
        }
    }
}

pub type PromptVariables = HashMap<String, String>;

#[derive(Debug, Clone, Copy)]
pub struct PromptCatalogEntry {
    pub path_stem: &'static str,
    pub variable_keys: &'static [&'static str],
}

#[derive(Debug)]
pub struct BuildPromptInput<'a> {
    pub prompt_id: PromptId,
    pub locale: PromptLocale,
    pub variables: &'a PromptVariables,
}
