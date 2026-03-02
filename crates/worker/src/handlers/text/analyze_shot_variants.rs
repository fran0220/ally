use serde_json::{Value, json};
use sqlx::FromRow;
use waoowaoo_core::errors::AppError;
use waoowaoo_core::media;
use waoowaoo_core::prompt_i18n::{PromptIds, PromptVariables};

use crate::{runtime, task_context::TaskContext};

use super::shared;

#[derive(Debug, FromRow)]
struct PanelVariantAnalysisRow {
    id: String,
    #[sqlx(rename = "panelNumber")]
    panel_number: Option<i32>,
    description: Option<String>,
    #[sqlx(rename = "shotType")]
    shot_type: Option<String>,
    #[sqlx(rename = "cameraMove")]
    camera_move: Option<String>,
    location: Option<String>,
    characters: Option<String>,
    #[sqlx(rename = "imageUrl")]
    image_url: Option<String>,
}

fn parse_panel_characters(raw: Option<&str>) -> String {
    let Some(raw) = raw.map(str::trim).filter(|item| !item.is_empty()) else {
        return "无".to_string();
    };

    let Ok(parsed) = serde_json::from_str::<Value>(raw) else {
        return "无".to_string();
    };
    let Some(items) = parsed.as_array() else {
        return "无".to_string();
    };

    let names = items
        .iter()
        .filter_map(|item| {
            if let Some(name) = item
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                return Some(name.to_string());
            }

            let object = item.as_object()?;
            let name = object
                .get("name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())?;
            let appearance = object
                .get("appearance")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());

            if let Some(appearance) = appearance {
                Some(format!("{name}（{appearance}）"))
            } else {
                Some(name.to_string())
            }
        })
        .collect::<Vec<_>>();

    if names.is_empty() {
        "无".to_string()
    } else {
        names.join("、")
    }
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    let mysql = runtime::mysql()?;
    let payload = &task.payload;
    let panel_id = shared::read_string(payload, "panelId")
        .ok_or_else(|| AppError::invalid_params("panelId is required"))?;

    let panel = sqlx::query_as::<_, PanelVariantAnalysisRow>(
        "SELECT id, panelNumber, description, shotType, cameraMove, location, characters, imageUrl FROM novel_promotion_panels WHERE id = ? LIMIT 1",
    )
    .bind(&panel_id)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("panel not found"))?;

    let image_source = media::to_public_media_url(panel.image_url.as_deref())
        .ok_or_else(|| AppError::invalid_params("panel imageUrl is missing"))?;
    let analysis_model = shared::resolve_analysis_model(task, payload).await?;

    let _ = task
        .report_progress(20, Some("analyze_shot_variants_prepare"))
        .await?;

    let mut prompt_variables = PromptVariables::new();
    prompt_variables.insert(
        "panel_description".to_string(),
        panel
            .description
            .clone()
            .unwrap_or_else(|| "无".to_string()),
    );
    prompt_variables.insert(
        "shot_type".to_string(),
        panel
            .shot_type
            .clone()
            .unwrap_or_else(|| "中景".to_string()),
    );
    prompt_variables.insert(
        "camera_move".to_string(),
        panel
            .camera_move
            .clone()
            .unwrap_or_else(|| "固定".to_string()),
    );
    prompt_variables.insert(
        "location".to_string(),
        panel.location.clone().unwrap_or_else(|| "未知".to_string()),
    );
    prompt_variables.insert(
        "characters_info".to_string(),
        parse_panel_characters(panel.characters.as_deref()),
    );

    let prompt = shared::render_prompt_template(
        payload,
        PromptIds::NP_AGENT_SHOT_VARIANT_ANALYSIS,
        &prompt_variables,
    )?;

    let response = shared::vision_chat(
        task,
        &analysis_model,
        &prompt,
        std::slice::from_ref(&image_source),
    )
    .await?;
    let suggestions = shared::parse_json_array_response(&response)?;
    if suggestions.len() < 3 {
        return Err(AppError::invalid_params(
            "analyze_shot_variants requires at least 3 suggestions",
        ));
    }

    let _ = task
        .report_progress(96, Some("analyze_shot_variants_done"))
        .await?;

    Ok(json!({
        "success": true,
        "panelId": panel.id,
        "panelNumber": panel.panel_number,
        "suggestions": suggestions,
        "panelInfo": {
            "panelNumber": panel.panel_number,
            "imageUrl": image_source,
            "description": panel.description,
        },
        "model": analysis_model,
    }))
}
