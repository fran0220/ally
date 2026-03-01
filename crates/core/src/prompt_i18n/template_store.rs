use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use once_cell::sync::Lazy;

use super::{
    PromptId, PromptLocale,
    catalog::prompt_catalog_entry,
    errors::{PromptI18nError, PromptI18nErrorCode},
};

static TEMPLATE_CACHE: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn build_cache_key(prompt_id: PromptId, locale: PromptLocale) -> String {
    format!("{}:{}", prompt_id.as_str(), locale.as_str())
}

fn push_unique_dir(dirs: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !dirs.contains(&candidate) {
        dirs.push(candidate);
    }
}

fn candidate_prompt_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(cwd) = env::current_dir() {
        push_unique_dir(&mut dirs, cwd.join("lib/prompts"));
        push_unique_dir(&mut dirs, cwd.join("../lib/prompts"));
    }

    // When compiled from crates/core, this points to <repo>/lib/prompts.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    push_unique_dir(&mut dirs, manifest_dir.join("../../lib/prompts"));
    push_unique_dir(&mut dirs, manifest_dir.join("../../../lib/prompts"));

    dirs
}

fn resolve_template_path(path_stem: &str, locale: PromptLocale) -> Result<PathBuf, Vec<PathBuf>> {
    let relative = format!("{path_stem}.{}.txt", locale.as_str());
    let mut candidates = Vec::new();

    for dir in candidate_prompt_dirs() {
        let candidate = dir.join(&relative);
        if candidate.is_file() {
            return Ok(candidate);
        }
        candidates.push(candidate);
    }

    Err(candidates)
}

fn read_template(path: &Path) -> Result<String, std::io::Error> {
    fs::read_to_string(path)
}

pub fn get_prompt_template(
    prompt_id: PromptId,
    locale: PromptLocale,
) -> Result<String, PromptI18nError> {
    let cache_key = build_cache_key(prompt_id, locale);

    if let Some(template) = TEMPLATE_CACHE
        .lock()
        .map_err(|_| {
            PromptI18nError::new(
                PromptI18nErrorCode::PromptTemplateNotFound,
                prompt_id,
                "prompt template cache poisoned",
            )
        })?
        .get(&cache_key)
        .cloned()
    {
        return Ok(template);
    }

    let entry = prompt_catalog_entry(prompt_id);
    let template_path = match resolve_template_path(entry.path_stem, locale) {
        Ok(path) => path,
        Err(candidates) => {
            let mut error = PromptI18nError::new(
                PromptI18nErrorCode::PromptTemplateNotFound,
                prompt_id,
                "prompt template file not found",
            )
            .with_detail("pathStem", entry.path_stem)
            .with_detail("locale", locale.as_str());

            for (index, candidate) in candidates.into_iter().enumerate() {
                error = error.with_detail(
                    format!("candidatePath{index}"),
                    candidate.display().to_string(),
                );
            }
            return Err(error);
        }
    };

    let template = read_template(&template_path).map_err(|error| {
        PromptI18nError::new(
            PromptI18nErrorCode::PromptTemplateNotFound,
            prompt_id,
            format!("failed to read prompt template: {error}"),
        )
        .with_detail("templatePath", template_path.display().to_string())
    })?;

    TEMPLATE_CACHE
        .lock()
        .map_err(|_| {
            PromptI18nError::new(
                PromptI18nErrorCode::PromptTemplateNotFound,
                prompt_id,
                "prompt template cache poisoned",
            )
        })?
        .insert(cache_key, template.clone());

    Ok(template)
}
