use serde_json::{Map, Value, json};
use sqlx::FromRow;
use waoowaoo_core::errors::AppError;
use waoowaoo_core::prompt_i18n::{PromptIds, PromptVariables};

use crate::{runtime, task_context::TaskContext};

use super::shared;

#[derive(Debug, FromRow)]
struct EpisodeTextRow {
    #[sqlx(rename = "novelText")]
    novel_text: Option<String>,
}

#[derive(Debug, FromRow)]
struct ExistingCharacterRow {
    name: String,
}

#[derive(Debug, FromRow)]
struct ExistingLocationRow {
    name: String,
}

fn read_optional_text(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn build_character_profile_data(item: &Value) -> Value {
    if let Some(profile_data) = item.get("profile_data").and_then(Value::as_object) {
        return Value::Object(profile_data.clone());
    }

    let mut profile = Map::new();
    for key in [
        "role_level",
        "archetype",
        "era_period",
        "social_class",
        "occupation",
        "costume_tier",
        "primary_identifier",
        "gender",
        "age_range",
    ] {
        if let Some(value) = item.get(key).cloned() {
            profile.insert(key.to_string(), value);
        }
    }

    let personality_tags = shared::read_string_array(item.get("personality_tags"));
    if !personality_tags.is_empty() {
        profile.insert("personality_tags".to_string(), json!(personality_tags));
    }
    let suggested_colors = shared::read_string_array(item.get("suggested_colors"));
    if !suggested_colors.is_empty() {
        profile.insert("suggested_colors".to_string(), json!(suggested_colors));
    }
    let visual_keywords = shared::read_string_array(item.get("visual_keywords"));
    if !visual_keywords.is_empty() {
        profile.insert("visual_keywords".to_string(), json!(visual_keywords));
    }
    if let Some(expected_appearances) = item
        .get("expected_appearances")
        .filter(|value| value.is_array())
    {
        profile.insert(
            "expected_appearances".to_string(),
            expected_appearances.clone(),
        );
    }

    Value::Object(profile)
}

fn read_new_characters(payload: &Value) -> Vec<Value> {
    payload
        .get("new_characters")
        .and_then(Value::as_array)
        .or_else(|| payload.get("characters").and_then(Value::as_array))
        .cloned()
        .unwrap_or_default()
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    shared::ensure_novel_project(task).await?;
    let mysql = runtime::mysql()?;
    let payload = &task.payload;
    let locale = shared::resolve_prompt_locale(payload);

    let novel_project = shared::get_novel_project(task).await?;
    let analysis_model = shared::resolve_analysis_model(task, payload).await?;

    let _ = task
        .report_progress(20, Some("progress.stage.analyzeNovelPrepare"))
        .await?;

    let first_episode = sqlx::query_as::<_, EpisodeTextRow>(
        "SELECT novelText FROM novel_promotion_episodes WHERE novelPromotionProjectId = ? ORDER BY createdAt ASC LIMIT 1",
    )
    .bind(&novel_project.id)
    .fetch_optional(mysql)
    .await?;

    let mut content = novel_project
        .global_asset_text
        .as_deref()
        .map(str::trim)
        .map(|item| item.to_string())
        .filter(|item| !item.is_empty())
        .or_else(|| {
            first_episode
                .as_ref()
                .and_then(|item| item.novel_text.as_deref())
                .map(str::trim)
                .map(|item| item.to_string())
                .filter(|item| !item.is_empty())
        })
        .ok_or_else(|| {
            AppError::invalid_params("global asset text or episode novel text is required")
        })?;
    if content.len() > 30000 {
        content.truncate(30000);
    }

    let existing_characters = sqlx::query_as::<_, ExistingCharacterRow>(
        "SELECT name FROM novel_promotion_characters WHERE novelPromotionProjectId = ?",
    )
    .bind(&novel_project.id)
    .fetch_all(mysql)
    .await?;
    let existing_locations = sqlx::query_as::<_, ExistingLocationRow>(
        "SELECT name FROM novel_promotion_locations WHERE novelPromotionProjectId = ?",
    )
    .bind(&novel_project.id)
    .fetch_all(mysql)
    .await?;

    let mut character_variables = PromptVariables::new();
    character_variables.insert("input".to_string(), content.clone());
    character_variables.insert(
        "characters_lib_info".to_string(),
        existing_characters
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    );
    let mut location_variables = PromptVariables::new();
    location_variables.insert("input".to_string(), content);
    location_variables.insert(
        "locations_lib_name".to_string(),
        existing_locations
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    );
    let characters_prompt = shared::render_prompt_template(
        payload,
        PromptIds::NP_AGENT_CHARACTER_PROFILE,
        &character_variables,
    )?;
    let locations_prompt = shared::render_prompt_template(
        payload,
        PromptIds::NP_SELECT_LOCATION,
        &location_variables,
    )?;

    let run_id = shared::create_stream_run_id(task, "analyze_novel");
    let character_meta = shared::LlmStepMeta {
        run_id: Some(run_id.clone()),
        stream_run_id: Some(run_id.clone()),
        step_id: Some("analyze_characters".to_string()),
        step_title: Some(shared::l(locale, "角色分析", "Character Analysis").to_string()),
        step_attempt: Some(1),
        step_index: Some(1),
        step_total: Some(2),
    };
    let location_meta = shared::LlmStepMeta {
        run_id: Some(run_id.clone()),
        stream_run_id: Some(run_id),
        step_id: Some("analyze_locations".to_string()),
        step_title: Some(shared::l(locale, "场景分析", "Location Analysis").to_string()),
        step_attempt: Some(1),
        step_index: Some(2),
        step_total: Some(2),
    };

    let (characters_response, locations_response) = tokio::try_join!(
        shared::chat_with_step(task, &analysis_model, &characters_prompt, &character_meta),
        shared::chat_with_step(task, &analysis_model, &locations_prompt, &location_meta),
    )?;

    let _ = task
        .report_progress(60, Some("progress.stage.analyzeNovelCharactersDone"))
        .await?;
    let _ = task
        .report_progress(70, Some("progress.stage.analyzeNovelLocationsDone"))
        .await?;

    let characters_payload = shared::parse_json_object_response(&characters_response)?;
    let locations_payload = shared::parse_json_object_response(&locations_response)?;
    let characters = read_new_characters(&characters_payload);
    let locations = locations_payload
        .get("locations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let _ = task
        .report_progress(75, Some("progress.stage.analyzeNovelPersist"))
        .await?;

    let mut known_character_names = existing_characters
        .iter()
        .map(|item| item.name.clone())
        .collect::<Vec<_>>();
    let mut created_character_count = 0usize;
    for item in &characters {
        let name = read_optional_text(item.get("name"));
        let Some(name) = name else {
            continue;
        };

        if known_character_names
            .iter()
            .any(|item| shared::name_matches_with_alias(item, &name))
        {
            continue;
        }

        let aliases = shared::read_string_array(item.get("aliases"));
        let introduction = read_optional_text(item.get("introduction"));
        let profile_data = build_character_profile_data(item);

        sqlx::query(
            "INSERT INTO novel_promotion_characters (id, novelPromotionProjectId, name, aliases, profileData, profileConfirmed, introduction, createdAt, updatedAt) VALUES (UUID(), ?, ?, ?, ?, FALSE, ?, NOW(3), NOW(3))",
        )
        .bind(&novel_project.id)
        .bind(&name)
        .bind(if aliases.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&aliases).map_err(|err| {
                AppError::internal(format!("failed to encode character aliases: {err}"))
            })?)
        })
        .bind(serde_json::to_string(&profile_data).map_err(|err| {
            AppError::internal(format!("failed to encode character profile_data: {err}"))
        })?)
        .bind(introduction)
        .execute(mysql)
        .await?;

        known_character_names.push(name);
        created_character_count += 1;
    }

    let mut known_location_names = existing_locations
        .iter()
        .map(|item| item.name.clone())
        .collect::<Vec<_>>();
    let mut created_location_count = 0usize;
    for item in &locations {
        let name = read_optional_text(item.get("name"));
        let Some(name) = name else {
            continue;
        };

        if known_location_names
            .iter()
            .any(|item| shared::name_matches_with_alias(item, &name))
        {
            continue;
        }

        let summary = read_optional_text(item.get("summary"));
        let mut descriptions = shared::read_string_array(item.get("descriptions"));
        if descriptions.is_empty()
            && let Some(single) = read_optional_text(item.get("description"))
        {
            descriptions.push(single);
        }
        let first_description = descriptions.first().cloned().unwrap_or_default();
        let invalid_summary = summary.clone().unwrap_or(first_description);
        if shared::is_invalid_location(&name, &invalid_summary) {
            continue;
        }

        let clean_descriptions = descriptions
            .iter()
            .map(|item| shared::remove_location_prompt_suffix(item))
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>();

        let location_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO novel_promotion_locations (id, novelPromotionProjectId, name, summary, createdAt, updatedAt) VALUES (?, ?, ?, ?, NOW(3), NOW(3))",
        )
        .bind(&location_id)
        .bind(&novel_project.id)
        .bind(&name)
        .bind(summary)
        .execute(mysql)
        .await?;

        for (index, description) in clean_descriptions.iter().enumerate() {
            sqlx::query(
                "INSERT INTO location_images (id, locationId, imageIndex, description, createdAt, updatedAt) VALUES (UUID(), ?, ?, ?, NOW(3), NOW(3))",
            )
            .bind(&location_id)
            .bind(index as i32)
            .bind(description)
            .execute(mysql)
            .await?;
        }

        known_location_names.push(name);
        created_location_count += 1;
    }

    let art_style_prompt = shared::resolve_art_style_prompt(
        novel_project.art_style.as_deref(),
        shared::resolve_prompt_locale(payload),
    );
    sqlx::query(
        "UPDATE novel_promotion_projects SET artStylePrompt = ?, updatedAt = NOW(3) WHERE id = ?",
    )
    .bind(art_style_prompt)
    .bind(&novel_project.id)
    .execute(mysql)
    .await?;

    let _ = task
        .report_progress(96, Some("progress.stage.analyzeNovelDone"))
        .await?;

    Ok(json!({
        "success": true,
        "characterCount": created_character_count,
        "locationCount": created_location_count,
        "model": analysis_model,
    }))
}
