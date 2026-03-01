use serde::Deserialize;
use serde_json::Value;
use sqlx::FromRow;
use waoowaoo_core::{
    errors::AppError,
    generators::{self, ImageGenerateOptions},
    media,
};

use crate::runtime;

const CHARACTER_PROMPT_SUFFIX: &str = "Character sheet layout on pure white background: left third close-up portrait, right two-thirds three-view lineup from left to right (front full body, side full body, back full body), consistent height and identity details.";

#[derive(Debug, Clone, Default)]
pub struct ProjectModels {
    pub analysis_model: Option<String>,
    pub character_model: Option<String>,
    pub location_model: Option<String>,
    pub storyboard_model: Option<String>,
    pub edit_model: Option<String>,
    #[allow(dead_code)]
    pub video_model: Option<String>,
    pub video_ratio: String,
    pub art_style: Option<String>,
    pub image_resolution: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UserModels {
    pub analysis_model: Option<String>,
    pub character_model: Option<String>,
    pub location_model: Option<String>,
    pub edit_model: Option<String>,
    #[allow(dead_code)]
    pub lip_sync_model: Option<String>,
}

#[derive(Debug, FromRow)]
struct ProjectModelRow {
    #[sqlx(rename = "analysisModel")]
    analysis_model: Option<String>,
    #[sqlx(rename = "characterModel")]
    character_model: Option<String>,
    #[sqlx(rename = "locationModel")]
    location_model: Option<String>,
    #[sqlx(rename = "storyboardModel")]
    storyboard_model: Option<String>,
    #[sqlx(rename = "editModel")]
    edit_model: Option<String>,
    #[sqlx(rename = "videoModel")]
    video_model: Option<String>,
    #[sqlx(rename = "videoRatio")]
    video_ratio: Option<String>,
    #[sqlx(rename = "artStyle")]
    art_style: Option<String>,
    #[sqlx(rename = "imageResolution")]
    image_resolution: Option<String>,
}

#[derive(Debug, FromRow)]
struct UserModelRow {
    #[sqlx(rename = "analysisModel")]
    analysis_model: Option<String>,
    #[sqlx(rename = "characterModel")]
    character_model: Option<String>,
    #[sqlx(rename = "locationModel")]
    location_model: Option<String>,
    #[sqlx(rename = "editModel")]
    edit_model: Option<String>,
    #[sqlx(rename = "lipSyncModel")]
    lip_sync_model: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PanelCharacterRef {
    pub name: String,
    pub appearance: Option<String>,
}

pub fn read_locale_tag(payload: &Value) -> &'static str {
    payload
        .get("meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get("locale"))
        .and_then(Value::as_str)
        .or_else(|| payload.get("locale").and_then(Value::as_str))
        .map(|item| item.trim().to_ascii_lowercase())
        .filter(|item| !item.is_empty())
        .filter(|item| item.starts_with("en"))
        .map(|_| "en")
        .unwrap_or("zh")
}

pub fn resolve_art_style_prompt(style_key: Option<&str>, locale: &str) -> Option<String> {
    let key = style_key?.trim();
    if key.is_empty() {
        return None;
    }

    let text = match (key, locale) {
        ("american-comic", "en") => "Japanese anime style",
        ("american-comic", _) => "Japanese anime style",
        ("chinese-comic", "en") => {
            "Modern premium Chinese comic style, rich details, clean sharp line art, full texture, ultra-clear 2D anime aesthetics."
        }
        ("chinese-comic", _) => {
            "Modern premium Chinese comic style, rich details, clean sharp line art, full texture, ultra-clear 2D anime aesthetics."
        }
        ("japanese-anime", "en") => {
            "Modern Japanese anime style, cel shading, clean line art, visual-novel CG look, high-quality 2D style."
        }
        ("japanese-anime", _) => {
            "Modern Japanese anime style, cel shading, clean line art, visual-novel CG look, high-quality 2D style."
        }
        ("realistic", "en") => {
            "Realistic cinematic look, real-world scene fidelity, rich transparent colors, clean and refined image quality."
        }
        ("realistic", _) => {
            "Realistic cinematic look, real-world scene fidelity, rich transparent colors, clean and refined image quality."
        }
        _ => "",
    };

    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

pub fn default_image_style_prompt(locale: &str) -> &'static str {
    if locale == "en" {
        "Match the style of provided references"
    } else {
        "Match the style of provided references"
    }
}

pub fn add_character_prompt_suffix(prompt: &str) -> String {
    let base = prompt.trim();
    if base.is_empty() {
        return CHARACTER_PROMPT_SUFFIX.to_string();
    }

    if base.contains(CHARACTER_PROMPT_SUFFIX) {
        return base.to_string();
    }

    format!("{base}, {CHARACTER_PROMPT_SUFFIX}")
}

pub fn add_location_prompt_suffix(prompt: &str) -> String {
    prompt.trim().to_string()
}

pub fn read_string(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn read_i32(payload: &Value, key: &str) -> Option<i32> {
    payload
        .get(key)
        .and_then(|value| {
            value.as_i64().or_else(|| {
                value
                    .as_str()
                    .and_then(|raw| raw.trim().parse::<i64>().ok())
            })
        })
        .map(|value| value as i32)
}

pub fn read_usize(payload: &Value, key: &str) -> Option<usize> {
    read_i32(payload, key).and_then(|value| {
        if value >= 0 {
            Some(value as usize)
        } else {
            None
        }
    })
}

pub fn parse_string_array(raw: Option<&str>) -> Vec<String> {
    raw.and_then(|value| serde_json::from_str::<Vec<String>>(value).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

pub fn parse_panel_character_refs(raw: Option<&str>) -> Vec<PanelCharacterRef> {
    let Some(value) = raw else {
        return Vec::new();
    };

    let parsed: Value = serde_json::from_str(value).unwrap_or(Value::Null);
    let Some(items) = parsed.as_array() else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(|item| {
            if let Some(name) = item.as_str() {
                let trimmed = name.trim();
                if trimmed.is_empty() {
                    return None;
                }
                return Some(PanelCharacterRef {
                    name: trimmed.to_string(),
                    appearance: None,
                });
            }

            let object = item.as_object()?;
            let name = object
                .get("name")
                .and_then(Value::as_str)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())?;
            let appearance = object
                .get("appearance")
                .and_then(Value::as_str)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());

            Some(PanelCharacterRef { name, appearance })
        })
        .collect()
}

pub fn parse_image_urls(raw: Option<&str>) -> Vec<String> {
    parse_string_array(raw)
}

pub fn clamp_count(raw: Option<i32>, fallback: i32, min: i32, max: i32) -> i32 {
    let value = raw.unwrap_or(fallback);
    value.max(min).min(max)
}

pub fn collect_extra_image_urls(payload: &Value) -> Vec<String> {
    let mut result = Vec::new();

    if let Some(items) = payload.get("extraImageUrls").and_then(Value::as_array) {
        for item in items {
            if let Some(url) = item
                .as_str()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
            {
                result.push(url.to_string());
            }
        }
    }

    if let Some(items) = payload.get("selectedAssets").and_then(Value::as_array) {
        for item in items {
            if let Some(url) = item
                .get("imageUrl")
                .and_then(Value::as_str)
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
            {
                result.push(url.to_string());
            }
        }
    }

    result
}

pub async fn get_project_models(
    project_id: &str,
    user_id: &str,
) -> Result<ProjectModels, AppError> {
    let mysql = runtime::mysql()?;

    let project = sqlx::query_as::<_, ProjectModelRow>(
        "SELECT analysisModel, characterModel, locationModel, storyboardModel, editModel, videoModel, videoRatio, artStyle, imageResolution FROM novel_promotion_projects WHERE projectId = ? LIMIT 1",
    )
    .bind(project_id)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("novel promotion project not found"))?;

    let user_pref = sqlx::query_as::<_, UserModelRow>(
        "SELECT analysisModel, characterModel, locationModel, editModel, lipSyncModel FROM user_preferences WHERE userId = ? LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(mysql)
    .await?;

    Ok(ProjectModels {
        analysis_model: project.analysis_model.or_else(|| {
            user_pref
                .as_ref()
                .and_then(|item| item.analysis_model.clone())
        }),
        character_model: project.character_model,
        location_model: project.location_model,
        storyboard_model: project.storyboard_model,
        edit_model: project.edit_model,
        video_model: project.video_model,
        video_ratio: project.video_ratio.unwrap_or_else(|| "16:9".to_string()),
        art_style: project.art_style,
        image_resolution: project.image_resolution,
    })
}

pub async fn get_user_models(user_id: &str) -> Result<UserModels, AppError> {
    let mysql = runtime::mysql()?;

    let row = sqlx::query_as::<_, UserModelRow>(
        "SELECT analysisModel, characterModel, locationModel, editModel, lipSyncModel FROM user_preferences WHERE userId = ? LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("user preference not found"))?;

    Ok(UserModels {
        analysis_model: row.analysis_model,
        character_model: row.character_model,
        location_model: row.location_model,
        edit_model: row.edit_model,
        lip_sync_model: row.lip_sync_model,
    })
}

pub async fn generate_image_to_storage(
    model_key: &str,
    prompt: &str,
    options: ImageGenerateOptions,
    key_prefix: &str,
    target_id: &str,
) -> Result<String, AppError> {
    let mysql = runtime::mysql()?;
    let source = generators::generate_image(mysql, model_key, prompt, options).await?;
    media::upload_source_to_storage(&source, key_prefix, target_id).await
}

pub fn to_fetchable_url(value: Option<&str>) -> Option<String> {
    media::to_public_media_url(value)
}

pub async fn normalize_reference_urls(urls: &[String]) -> Result<Vec<String>, AppError> {
    let mut deduped = Vec::with_capacity(urls.len());
    for url in urls {
        let value = url.trim();
        if value.is_empty() {
            continue;
        }
        if deduped.iter().any(|item| item == value) {
            continue;
        }
        deduped.push(value.to_string());
    }

    media::normalize_reference_sources_to_data_urls(&deduped).await
}
