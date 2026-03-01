use serde_json::{Value, json};
use waoowaoo_core::errors::AppError;
use waoowaoo_core::generators::{self, VoiceDesignInput};

use crate::{handlers::image::shared, runtime, task_context::TaskContext};

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    handle_with_options(task).await
}

pub async fn handle_with_options(task: &TaskContext) -> Result<Value, AppError> {
    let mysql = runtime::mysql()?;
    let payload = &task.payload;

    let voice_prompt = shared::read_string(payload, "voicePrompt")
        .ok_or_else(|| AppError::invalid_params("voicePrompt is required"))?;
    let preview_text = shared::read_string(payload, "previewText")
        .ok_or_else(|| AppError::invalid_params("previewText is required"))?;
    if voice_prompt.chars().count() > 500 {
        return Err(AppError::invalid_params(
            "voicePrompt cannot exceed 500 characters",
        ));
    }
    let preview_len = preview_text.chars().count();
    if preview_len < 5 {
        return Err(AppError::invalid_params(
            "previewText must be at least 5 characters",
        ));
    }
    if preview_len > 200 {
        return Err(AppError::invalid_params(
            "previewText cannot exceed 200 characters",
        ));
    }

    let preferred_name =
        shared::read_string(payload, "preferredName").unwrap_or_else(|| "custom_voice".to_string());
    let language = match shared::read_string(payload, "language")
        .unwrap_or_else(|| "zh".to_string())
        .to_lowercase()
        .as_str()
    {
        "en" => "en".to_string(),
        _ => "zh".to_string(),
    };

    let _ = task
        .report_progress(25, Some("voice_design_submit"))
        .await?;

    let response = generators::create_voice_design(
        mysql,
        VoiceDesignInput {
            voice_prompt: voice_prompt.clone(),
            preview_text: preview_text.clone(),
            preferred_name: preferred_name.clone(),
            language: language.clone(),
        },
    )
    .await?;

    let output = response.get("output").cloned().unwrap_or(Value::Null);
    let voice_id = ["voice", "voice_id", "voiceId"]
        .iter()
        .find_map(|field| {
            output
                .get(field)
                .and_then(Value::as_str)
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
        })
        .ok_or_else(|| AppError::internal("voice design response missing voice id"))?;

    if let Some(character_id) = shared::read_string(payload, "characterId") {
        sqlx::query(
            "UPDATE novel_promotion_characters SET voiceId = ?, voiceType = 'qwen-designed', updatedAt = NOW(3) WHERE id = ?",
        )
        .bind(&voice_id)
        .bind(&character_id)
        .execute(mysql)
        .await?;
    }

    let global_voice_id = shared::read_string(payload, "globalVoiceId");
    if let Some(global_voice_id) = global_voice_id {
        sqlx::query(
            "UPDATE global_voices SET voiceId = ?, voiceType = 'qwen-designed', voicePrompt = ?, language = ?, updatedAt = NOW(3) WHERE id = ? AND userId = ?",
        )
        .bind(&voice_id)
        .bind(&voice_prompt)
        .bind(&language)
        .bind(&global_voice_id)
        .bind(&task.user_id)
        .execute(mysql)
        .await?;
    }

    let _ = task.report_progress(96, Some("voice_design_done")).await?;

    let target_model = output
        .get("target_model")
        .and_then(Value::as_str)
        .map(|item| item.to_string());
    let audio_base64 = output
        .get("preview_audio")
        .and_then(|preview| preview.get("data"))
        .and_then(Value::as_str)
        .map(|item| item.to_string());
    let sample_rate = output
        .get("preview_audio")
        .and_then(|preview| preview.get("sample_rate"))
        .and_then(Value::as_i64);
    let response_format = output
        .get("preview_audio")
        .and_then(|preview| preview.get("response_format"))
        .and_then(Value::as_str)
        .map(|item| item.to_string());
    let usage_count = response
        .get("usage")
        .and_then(|usage| usage.get("count"))
        .and_then(Value::as_i64);
    let request_id = response
        .get("request_id")
        .and_then(Value::as_str)
        .map(|item| item.to_string());

    Ok(json!({
        "success": true,
        "voiceId": voice_id,
        "targetModel": target_model,
        "audioBase64": audio_base64,
        "sampleRate": sample_rate,
        "responseFormat": response_format,
        "usageCount": usage_count,
        "requestId": request_id,
        "preferredName": preferred_name,
        "language": language,
        "taskType": task.task_type,
        "output": output,
    }))
}
