use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};
use sqlx::FromRow;
use waoowaoo_core::api_config::get_system_default_models;
use waoowaoo_core::errors::{AppError, ErrorCode};
use waoowaoo_core::llm::{self, ChatMessage};
use waoowaoo_core::prompt_i18n::{self, BuildPromptInput, PromptId, PromptLocale, PromptVariables};

use crate::{
    consumer::WorkerTask, handlers::image::shared as image_shared, runtime,
    task_context::TaskContext,
};

const MAX_LLM_STREAM_CHUNK_CHARS: usize = 128;
const INVALID_LOCATION_KEYWORDS: [&str; 6] =
    ["幻想", "抽象", "无明确", "空间锚点", "未说明", "不明确"];

#[derive(Debug, FromRow)]
pub struct NovelProjectRow {
    pub id: String,
    #[sqlx(rename = "globalAssetText")]
    pub global_asset_text: Option<String>,
    #[sqlx(rename = "artStyle")]
    pub art_style: Option<String>,
}

#[derive(Debug, FromRow)]
pub struct ProjectModeRow {
    pub mode: String,
}

#[derive(Debug, Clone, Default)]
pub struct LlmStepMeta {
    pub run_id: Option<String>,
    pub stream_run_id: Option<String>,
    pub step_id: Option<String>,
    pub step_title: Option<String>,
    pub step_attempt: Option<i32>,
    pub step_index: Option<i32>,
    pub step_total: Option<i32>,
}

#[derive(Debug, Clone)]
struct NormalizedContent {
    text: Vec<char>,
    raw_start_by_norm: Vec<usize>,
    raw_end_by_norm: Vec<usize>,
}

// Keep prompt suffix handling aligned with the TypeScript worker so user-facing
// prompts never leak system-level generation instructions.
pub const CHARACTER_PROMPT_SUFFIX: &str = "角色设定图，画面分为左右两个区域：【左侧区域】占约1/3宽度，是角色的正面特写（如果是人类则展示完整正脸，如果是动物/生物则展示最具辨识度的正面形态）；【右侧区域】占约2/3宽度，是角色三视图横向排列（从左到右依次为：正面全身、侧面全身、背面全身），三视图高度一致。纯白色背景，无其他元素。";
pub const LOCATION_PROMPT_SUFFIX: &str = "";

pub fn create_stream_run_id(task: &WorkerTask, label: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or(0);
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    format!(
        "run:{}:{}:{:x}:{}",
        task.task_id,
        label.trim(),
        timestamp,
        &suffix[..8]
    )
}

pub fn read_string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

#[allow(dead_code)]
pub fn parse_aliases(raw: Option<&str>) -> Vec<String> {
    let Some(raw) = raw else {
        return Vec::new();
    };

    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    read_string_array(Some(&value))
}

pub fn build_characters_introduction(characters: &[(String, Option<String>)]) -> String {
    let introductions = characters
        .iter()
        .filter_map(|(name, introduction)| {
            let intro = introduction.as_deref()?.trim();
            if intro.is_empty() {
                return None;
            }
            Some(format!("- {name}：{intro}"))
        })
        .collect::<Vec<_>>();

    if introductions.is_empty() {
        return "暂无角色介绍".to_string();
    }
    introductions.join("\n")
}

pub fn is_invalid_location(name: &str, summary_or_description: &str) -> bool {
    INVALID_LOCATION_KEYWORDS
        .iter()
        .any(|keyword| name.contains(keyword) || summary_or_description.contains(keyword))
}

pub fn resolve_art_style_prompt(art_style: Option<&str>, locale: PromptLocale) -> String {
    let Some(art_style) = art_style.map(str::trim).filter(|value| !value.is_empty()) else {
        return String::new();
    };

    match (art_style, locale) {
        ("american-comic", PromptLocale::En) => "Japanese anime style".to_string(),
        ("american-comic", _) => "日式动漫风格".to_string(),
        ("chinese-comic", PromptLocale::En) => {
            "Modern premium Chinese comic style, rich details, clean sharp line art, full texture, ultra-clear 2D anime aesthetics.".to_string()
        }
        ("chinese-comic", _) => "现代高质量漫画风格，动漫风格，细节丰富精致，线条锐利干净，质感饱满，超清，干净的画面风格，2D风格，动漫风格。".to_string(),
        ("japanese-anime", PromptLocale::En) => {
            "Modern Japanese anime style, cel shading, clean line art, visual-novel CG look, high-quality 2D style.".to_string()
        }
        ("japanese-anime", _) => "现代日系动漫风格，赛璐璐上色，清晰干净的线条，视觉小说CG感。高质量2D风格".to_string(),
        ("realistic", PromptLocale::En) => {
            "Realistic cinematic look, real-world scene fidelity, rich transparent colors, clean and refined image quality.".to_string()
        }
        ("realistic", _) => {
            "真实电影级画面质感，真实现实场景，色彩饱满通透，画面干净精致，真实感"
                .to_string()
        }
        _ => String::new(),
    }
}

#[allow(dead_code)]
pub fn chunk_content(text: &str, max_size: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();

    for paragraph in text
        .split("\n\n")
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        if current.len() + paragraph.len() + 2 > max_size {
            if !current.trim().is_empty() {
                chunks.push(current.trim().to_string());
            }
            current = paragraph.to_string();
        } else {
            if !current.is_empty() {
                current.push_str("\n\n");
            }
            current.push_str(paragraph);
        }
    }

    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }

    chunks
}

pub fn count_words_like_word(text: &str) -> usize {
    waoowaoo_core::episode_marker::count_words_like_word(text)
}

pub fn read_string(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

pub fn read_bool(payload: &Value, key: &str) -> bool {
    payload
        .get(key)
        .and_then(|value| {
            value.as_bool().or_else(|| {
                value.as_str().map(|raw| {
                    let normalized = raw.trim().to_lowercase();
                    matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
                })
            })
        })
        .unwrap_or(false)
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
        .and_then(|value| i32::try_from(value).ok())
}

pub fn remove_character_prompt_suffix(prompt: &str) -> String {
    prompt
        .replace(CHARACTER_PROMPT_SUFFIX, "")
        .trim()
        .to_string()
}

pub fn add_character_prompt_suffix(prompt: &str) -> String {
    if prompt.trim().is_empty() {
        return CHARACTER_PROMPT_SUFFIX.to_string();
    }

    let cleaned = remove_character_prompt_suffix(prompt);
    format!(
        "{}{separator}{suffix}",
        cleaned,
        separator = if cleaned.is_empty() { "" } else { "，" },
        suffix = CHARACTER_PROMPT_SUFFIX,
    )
}

pub fn remove_location_prompt_suffix(prompt: &str) -> String {
    let without_suffix = if LOCATION_PROMPT_SUFFIX.is_empty() {
        prompt.trim().to_string()
    } else {
        prompt.replace(LOCATION_PROMPT_SUFFIX, "")
    };

    without_suffix.trim_end_matches('，').trim().to_string()
}

fn read_prompt_locale(payload: &Value) -> Option<String> {
    payload
        .get("meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get("locale"))
        .and_then(Value::as_str)
        .or_else(|| payload.get("locale").and_then(Value::as_str))
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

pub fn resolve_prompt_locale(payload: &Value) -> PromptLocale {
    read_prompt_locale(payload)
        .and_then(|item| item.parse::<PromptLocale>().ok())
        .unwrap_or(PromptLocale::Zh)
}

pub fn render_prompt_template(
    payload: &Value,
    prompt_id: PromptId,
    variables: &PromptVariables,
) -> Result<String, AppError> {
    prompt_i18n::build_prompt(BuildPromptInput {
        prompt_id,
        locale: resolve_prompt_locale(payload),
        variables,
    })
    .map_err(AppError::from)
}

fn trim_code_fence(raw: &str) -> String {
    let mut cleaned = raw.trim().to_string();
    if cleaned.starts_with("```json") {
        cleaned = cleaned.replacen("```json", "", 1);
    } else if cleaned.starts_with("```") {
        cleaned = cleaned.replacen("```", "", 1);
    }
    if cleaned.ends_with("```") {
        cleaned = cleaned.trim_end_matches("```").trim().to_string();
    }
    cleaned
}

fn normalize_json_text(raw: &str) -> String {
    trim_code_fence(raw)
        .replace(['“', '”'], "\"")
        .replace(['‘', '’'], "'")
        .chars()
        .filter(|ch| !ch.is_control() || matches!(ch, '\n' | '\r' | '\t'))
        .collect::<String>()
}

fn strip_trailing_commas(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_string = false;
    let mut escaped = false;
    let chars = input.chars().collect::<Vec<_>>();
    let mut index = 0usize;

    while index < chars.len() {
        let ch = chars[index];

        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(ch);
            index += 1;
            continue;
        }

        if ch == ',' {
            let mut lookahead = index + 1;
            while lookahead < chars.len() && chars[lookahead].is_whitespace() {
                lookahead += 1;
            }
            if lookahead < chars.len() && matches!(chars[lookahead], '}' | ']') {
                index += 1;
                continue;
            }
        }

        out.push(ch);
        index += 1;
    }

    out
}

fn escape_control_chars_in_json_strings(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_string = false;
    let mut escaped = false;

    for ch in input.chars() {
        if !in_string {
            if ch == '"' {
                in_string = true;
            }
            out.push(ch);
            continue;
        }

        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' {
            out.push(ch);
            escaped = true;
            continue;
        }

        if ch == '"' {
            in_string = false;
            out.push(ch);
            continue;
        }

        match ch {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ if ch.is_control() => {
                let code = ch as u32;
                out.push_str(&format!("\\u{:04x}", code));
            }
            _ => out.push(ch),
        }
    }

    out
}

fn parse_json_value(candidate: &str) -> Result<Value, AppError> {
    let stripped = strip_trailing_commas(candidate);
    match serde_json::from_str::<Value>(&stripped) {
        Ok(value) => Ok(value),
        Err(_) => {
            let repaired = escape_control_chars_in_json_strings(&stripped);
            Ok(serde_json::from_str::<Value>(&repaired)?)
        }
    }
}

pub fn parse_json_object_response(raw: &str) -> Result<Value, AppError> {
    let cleaned = normalize_json_text(raw);
    let start = cleaned
        .find('{')
        .ok_or_else(|| AppError::invalid_params("json object not found in llm response"))?;
    let end = cleaned
        .rfind('}')
        .ok_or_else(|| AppError::invalid_params("json object not found in llm response"))?;
    if end <= start {
        return Err(AppError::invalid_params(
            "json object boundaries are invalid in llm response",
        ));
    }

    let value = parse_json_value(&cleaned[start..=end])?;
    if !value.is_object() {
        return Err(AppError::invalid_params("json payload must be an object"));
    }
    Ok(value)
}

pub fn parse_json_array_response(raw: &str) -> Result<Vec<Value>, AppError> {
    let cleaned = normalize_json_text(raw);
    let start = cleaned
        .find('[')
        .ok_or_else(|| AppError::invalid_params("json array not found in llm response"))?;
    let end = cleaned
        .rfind(']')
        .ok_or_else(|| AppError::invalid_params("json array not found in llm response"))?;
    if end <= start {
        return Err(AppError::invalid_params(
            "json array boundaries are invalid in llm response",
        ));
    }

    let value = parse_json_value(&cleaned[start..=end])?;
    let array = value
        .as_array()
        .ok_or_else(|| AppError::invalid_params("json payload must be an array for this task"))?;
    Ok(array.clone())
}

pub fn name_matches_with_alias(a: &str, b: &str) -> bool {
    let a = a.trim().to_lowercase();
    let b = b.trim().to_lowercase();
    if a.is_empty() || b.is_empty() {
        return false;
    }
    if a == b {
        return true;
    }

    let aliases_a = a
        .split('/')
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    let aliases_b = b
        .split('/')
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();

    aliases_b.iter().any(|alias| aliases_a.contains(alias))
}

pub async fn ensure_novel_project(task: &WorkerTask) -> Result<(), AppError> {
    let mysql = runtime::mysql()?;
    let project =
        sqlx::query_as::<_, ProjectModeRow>("SELECT mode FROM projects WHERE id = ? LIMIT 1")
            .bind(&task.project_id)
            .fetch_optional(mysql)
            .await?
            .ok_or_else(|| AppError::not_found("project not found"))?;
    if project.mode != "novel-promotion" {
        return Err(AppError::invalid_params(
            "project mode must be novel-promotion",
        ));
    }
    Ok(())
}

pub async fn get_novel_project(task: &WorkerTask) -> Result<NovelProjectRow, AppError> {
    let mysql = runtime::mysql()?;
    sqlx::query_as::<_, NovelProjectRow>(
        "SELECT id, globalAssetText, artStyle FROM novel_promotion_projects WHERE projectId = ? LIMIT 1",
    )
    .bind(&task.project_id)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("novel promotion project not found"))
}

pub async fn resolve_analysis_model(
    task: &WorkerTask,
    payload: &Value,
) -> Result<String, AppError> {
    resolve_optional_analysis_model(task, payload)
        .await?
        .ok_or_else(|| AppError::invalid_params("analysis model is not configured"))
}

pub async fn resolve_optional_analysis_model(
    task: &WorkerTask,
    payload: &Value,
) -> Result<Option<String>, AppError> {
    if let Some(model) =
        read_string(payload, "analysisModel").or_else(|| read_string(payload, "model"))
    {
        return Ok(Some(model));
    }

    match image_shared::get_project_models(&task.project_id, &task.user_id).await {
        Ok(project_models) => {
            if let Some(model) = project_models.analysis_model {
                return Ok(Some(model));
            }
        }
        Err(err) if matches!(err.code, ErrorCode::NotFound) => {}
        Err(err) => return Err(err),
    }

    match image_shared::get_user_models(&task.user_id).await {
        Ok(user_models) => {
            if user_models.analysis_model.is_some() {
                return Ok(user_models.analysis_model);
            }
        }
        Err(err) if matches!(err.code, ErrorCode::NotFound) => {}
        Err(err) => return Err(err),
    };

    let mysql = runtime::mysql()?;
    let system_defaults = get_system_default_models(mysql).await?;
    Ok(if system_defaults.analysis_model.trim().is_empty() {
        None
    } else {
        Some(system_defaults.analysis_model.trim().to_string())
    })
}

pub async fn chat(_task: &WorkerTask, model: &str, prompt: &str) -> Result<String, AppError> {
    let mysql = runtime::mysql()?;
    llm::chat_completion(
        mysql,
        model,
        &[ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
        Some(0.7),
    )
    .await
}

fn read_reasoning_enabled(payload: &Value) -> bool {
    payload
        .get("reasoning")
        .and_then(|value| {
            value.as_bool().or_else(|| {
                value.as_str().map(|raw| {
                    let normalized = raw.trim().to_lowercase();
                    matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
                })
            })
        })
        .unwrap_or(true)
}

fn read_reasoning_effort(payload: &Value) -> Option<String> {
    read_string(payload, "reasoningEffort")
        .or_else(|| read_string(payload, "reasoning_effort"))
        .map(|raw| raw.trim().to_ascii_lowercase())
        .filter(|raw| matches!(raw.as_str(), "minimal" | "low" | "medium" | "high"))
}

fn push_stream_chunk(payload: &mut Map<String, Value>, meta: &LlmStepMeta) {
    if let Some(run_id) = meta.run_id.as_ref() {
        payload.insert("runId".to_string(), Value::String(run_id.clone()));
    }
    if let Some(stream_run_id) = meta.stream_run_id.as_ref() {
        payload.insert(
            "streamRunId".to_string(),
            Value::String(stream_run_id.clone()),
        );
    }
    if let Some(step_id) = meta.step_id.as_ref() {
        payload.insert("stepId".to_string(), Value::String(step_id.clone()));
    }
    if let Some(step_title) = meta.step_title.as_ref() {
        payload.insert("stepTitle".to_string(), Value::String(step_title.clone()));
    }
    if let Some(step_attempt) = meta.step_attempt {
        payload.insert("stepAttempt".to_string(), Value::from(step_attempt.max(1)));
    }
    if let Some(step_index) = meta.step_index {
        payload.insert("stepIndex".to_string(), Value::from(step_index.max(1)));
    }
    if let Some(step_total) = meta.step_total {
        payload.insert("stepTotal".to_string(), Value::from(step_total.max(1)));
    }
}

fn split_stream_chunks(text: &str, max_chars: usize) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut count = 0usize;

    for ch in text.chars() {
        current.push(ch);
        count += 1;
        if count >= max_chars {
            chunks.push(current);
            current = String::new();
            count = 0;
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

async fn report_stream_done(
    task: &TaskContext,
    meta: &LlmStepMeta,
    kind: llm::ChatStreamChunkKind,
    usage: Option<&llm::ChatStreamUsage>,
) -> Result<(), AppError> {
    let mut payload = Map::new();
    payload.insert("kind".to_string(), Value::String(kind.as_str().to_string()));
    payload.insert("lane".to_string(), Value::String(kind.lane().to_string()));
    payload.insert("delta".to_string(), Value::String(String::new()));
    payload.insert("done".to_string(), Value::Bool(true));
    if let Some(usage) = usage {
        payload.insert(
            "usage".to_string(),
            json!({
                "promptTokens": usage.prompt_tokens,
                "completionTokens": usage.completion_tokens,
            }),
        );
    }
    push_stream_chunk(&mut payload, meta);
    task.report_stream_chunk(Value::Object(payload)).await
}

pub async fn chat_with_stream_reporting(
    task: &TaskContext,
    model: &str,
    prompt: &str,
    meta: &LlmStepMeta,
) -> Result<String, AppError> {
    let _ = task
        .report_progress(65, Some("progress.runtime.stage.llmSubmit"))
        .await?;

    let mysql = runtime::mysql()?;
    let reasoning_enabled = read_reasoning_enabled(&task.payload);
    let reasoning_effort = read_reasoning_effort(&task.payload);
    let stream_result = llm::chat_completion_stream(
        mysql,
        model,
        &[ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
        llm::ChatStreamOptions {
            temperature: Some(0.7),
            reasoning: reasoning_enabled,
            reasoning_effort,
            chunk_timeout: None,
        },
        |chunk| async move {
            for piece in split_stream_chunks(&chunk.delta, MAX_LLM_STREAM_CHUNK_CHARS) {
                let mut payload = Map::new();
                payload.insert(
                    "kind".to_string(),
                    Value::String(chunk.kind.as_str().to_string()),
                );
                payload.insert(
                    "lane".to_string(),
                    Value::String(chunk.kind.lane().to_string()),
                );
                payload.insert("delta".to_string(), Value::String(piece));
                payload.insert("done".to_string(), Value::Bool(false));
                push_stream_chunk(&mut payload, meta);
                task.report_stream_chunk(Value::Object(payload)).await?;
            }
            Ok(())
        },
    )
    .await;

    match stream_result {
        Ok(result) => {
            if !result.reasoning.trim().is_empty() {
                report_stream_done(task, meta, llm::ChatStreamChunkKind::Reasoning, None).await?;
            }
            report_stream_done(
                task,
                meta,
                llm::ChatStreamChunkKind::Text,
                Some(&result.usage),
            )
            .await?;

            let _ = task
                .report_progress(90, Some("progress.runtime.stage.llmCompleted"))
                .await?;
            Ok(result.text)
        }
        Err(err) => {
            let _ = task
                .report_progress(90, Some("progress.runtime.stage.llmFailed"))
                .await?;
            Err(err)
        }
    }
}

pub async fn chat_with_step(
    task: &TaskContext,
    model: &str,
    prompt: &str,
    meta: &LlmStepMeta,
) -> Result<String, AppError> {
    chat_with_stream_reporting(task, model, prompt, meta).await
}

fn normalize_char(ch: char) -> String {
    let normalized = match ch {
        '\u{3000}' => ' ',
        _ => {
            let code = ch as u32;
            if (0xff01..=0xff5e).contains(&code) {
                char::from_u32(code - 0xfee0).unwrap_or(ch)
            } else {
                ch
            }
        }
    };

    match normalized {
        '，' => ",".to_string(),
        '。' => ".".to_string(),
        '！' => "!".to_string(),
        '？' => "?".to_string(),
        '；' => ";".to_string(),
        '：' => ":".to_string(),
        '（' => "(".to_string(),
        '）' => ")".to_string(),
        '【' => "[".to_string(),
        '】' => "]".to_string(),
        '《' => "<".to_string(),
        '》' => ">".to_string(),
        '「' | '」' | '『' | '』' | '“' | '”' => "\"".to_string(),
        '‘' | '’' => "'".to_string(),
        '、' => ",".to_string(),
        '…' => "...".to_string(),
        _ => normalized.to_string(),
    }
    .to_lowercase()
}

fn build_normalized_content(raw: &str) -> NormalizedContent {
    let mut text = Vec::new();
    let mut raw_start_by_norm = Vec::new();
    let mut raw_end_by_norm = Vec::new();

    for (index, ch) in raw.char_indices() {
        let transformed = normalize_char(ch);
        let next_index = index + ch.len_utf8();
        for item in transformed.chars() {
            if item.is_whitespace() {
                continue;
            }
            text.push(item);
            raw_start_by_norm.push(index);
            raw_end_by_norm.push(next_index);
        }
    }

    NormalizedContent {
        text,
        raw_start_by_norm,
        raw_end_by_norm,
    }
}

fn find_norm_index_for_raw(normalized: &NormalizedContent, raw_index: usize) -> usize {
    if normalized.raw_start_by_norm.is_empty() {
        return 0;
    }

    let mut left = 0usize;
    let mut right = normalized.raw_start_by_norm.len();
    while left < right {
        let mid = left + ((right - left) >> 1);
        if normalized.raw_start_by_norm[mid] < raw_index {
            left = mid + 1;
        } else {
            right = mid;
        }
    }
    left
}

fn find_subsequence(haystack: &[char], needle: &[char], from_index: usize) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() || from_index >= haystack.len() {
        return None;
    }

    let max_start = haystack.len().saturating_sub(needle.len());
    let mut index = from_index;
    while index <= max_start {
        if haystack[index..index + needle.len()] == needle[..] {
            return Some(index);
        }
        index += 1;
    }

    None
}

pub fn match_text_marker(
    content: &str,
    marker_text: &str,
    from_index: usize,
) -> Option<(usize, usize)> {
    let marker = marker_text.trim();
    if marker.is_empty() {
        return None;
    }

    if let Some(relative) = content
        .get(from_index..)
        .and_then(|slice| slice.find(marker))
    {
        let start = from_index + relative;
        let end = start + marker.len();
        return Some((start, end));
    }

    let normalized = build_normalized_content(content);
    let query = build_normalized_content(marker).text;
    if query.is_empty() {
        return None;
    }

    let from_norm = find_norm_index_for_raw(&normalized, from_index);
    let norm_index = find_subsequence(&normalized.text, &query, from_norm)?;
    let start = normalized.raw_start_by_norm[norm_index];
    if start < from_index {
        return None;
    }
    let end = normalized.raw_end_by_norm[norm_index + query.len() - 1];
    Some((start, end))
}

pub fn match_text_boundary(
    content: &str,
    start_marker: &str,
    end_marker: &str,
    from_index: usize,
) -> Option<(usize, usize)> {
    let (start_index, start_end) = match_text_marker(content, start_marker, from_index)?;
    let (_, end_index) = match_text_marker(content, end_marker, start_end)?;
    if end_index <= start_index {
        return None;
    }
    Some((start_index, end_index))
}

pub async fn vision_chat(
    _task: &WorkerTask,
    model: &str,
    prompt: &str,
    image_sources: &[String],
) -> Result<String, AppError> {
    let mysql = runtime::mysql()?;
    llm::vision_completion(mysql, model, prompt, image_sources).await
}

pub fn read_episode_id(task: &WorkerTask) -> Option<String> {
    read_string(&task.payload, "episodeId").or_else(|| task.episode_id.clone())
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use waoowaoo_core::prompt_i18n::PromptLocale;

    use super::{
        build_characters_introduction, count_words_like_word, match_text_boundary,
        parse_json_array_response, parse_json_object_response, read_reasoning_effort,
        read_reasoning_enabled, resolve_art_style_prompt,
    };

    #[test]
    fn parse_json_object_response_handles_code_fence_and_trailing_comma() {
        let raw = "```json\n{\n  \"name\": \"Alice\",\n  \"aliases\": [\"A\",],\n}\n```";
        let parsed = parse_json_object_response(raw).expect("parse json object");
        assert_eq!(parsed["name"], "Alice");
        assert_eq!(parsed["aliases"][0], "A");
    }

    #[test]
    fn parse_json_array_response_handles_embedded_array() {
        let raw = "prefix [ {\"lineIndex\":1}, {\"lineIndex\":2} ] suffix";
        let parsed = parse_json_array_response(raw).expect("parse array");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["lineIndex"], 1);
    }

    #[test]
    fn match_text_boundary_supports_exact_and_normalized_markers() {
        let content = "第一段。\n\n【场景A】 主角 走进房间。\n\n他抬头看向窗外。";
        let matched = match_text_boundary(content, "【场景A】主角走进房间", "抬头看向窗外", 0)
            .expect("match boundary");
        let slice = &content[matched.0..matched.1];
        assert!(slice.contains("主角"));
        assert!(slice.contains("窗外"));
    }

    #[test]
    fn count_words_like_word_matches_expected_behavior() {
        assert_eq!(count_words_like_word("hello world 你好"), 4);
    }

    #[test]
    fn build_characters_introduction_skips_empty_rows() {
        let intro = build_characters_introduction(&[
            ("林墨".to_string(), Some("男主".to_string())),
            ("路人".to_string(), Some("   ".to_string())),
        ]);
        assert_eq!(intro, "- 林墨：男主");
    }

    #[test]
    fn resolve_art_style_prompt_matches_locale() {
        let zh = resolve_art_style_prompt(Some("realistic"), PromptLocale::Zh);
        let en = resolve_art_style_prompt(Some("realistic"), PromptLocale::En);
        assert!(zh.contains("真实电影级"));
        assert!(en.contains("Realistic cinematic"));
    }

    #[test]
    fn read_reasoning_enabled_defaults_to_true() {
        assert!(read_reasoning_enabled(&json!({})));
        assert!(read_reasoning_enabled(&json!({ "reasoning": "true" })));
        assert!(!read_reasoning_enabled(&json!({ "reasoning": false })));
    }

    #[test]
    fn read_reasoning_effort_accepts_only_valid_values() {
        assert_eq!(
            read_reasoning_effort(&json!({ "reasoningEffort": "MEDIUM" })),
            Some("medium".to_string())
        );
        assert_eq!(
            read_reasoning_effort(&json!({ "reasoning_effort": "minimal" })),
            Some("minimal".to_string())
        );
        assert_eq!(
            read_reasoning_effort(&json!({ "reasoningEffort": "extreme" })),
            None
        );
    }
}
