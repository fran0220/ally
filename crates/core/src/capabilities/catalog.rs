use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::UNIX_EPOCH,
};

use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::Value;

use crate::{api_config::UnifiedModelType, errors::AppError};

use super::types::{
    ModelCapabilities, compose_model_key, model_type_key, provider_key, validate_model_capabilities,
};

#[derive(Debug, Clone)]
pub struct BuiltinCapabilityCatalogEntry {
    pub model_type: UnifiedModelType,
    pub provider: String,
    pub model_id: String,
    pub capabilities: Option<ModelCapabilities>,
}

#[derive(Debug, Clone)]
struct CatalogCache {
    signature: String,
    entries: Vec<BuiltinCapabilityCatalogEntry>,
    exact: HashMap<String, BuiltinCapabilityCatalogEntry>,
    by_provider_key: HashMap<String, BuiltinCapabilityCatalogEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawCatalogEntry {
    model_type: UnifiedModelType,
    provider: String,
    model_id: String,
    capabilities: Option<Value>,
}

static CATALOG_CACHE: Lazy<Mutex<Option<CatalogCache>>> = Lazy::new(|| Mutex::new(None));

const PROVIDER_ALIASES: &[(&str, &str)] = &[("gemini-compatible", "google")];

fn push_unique_dir(dirs: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !dirs.contains(&candidate) {
        dirs.push(candidate);
    }
}

fn candidate_catalog_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(cwd) = env::current_dir() {
        push_unique_dir(&mut dirs, cwd.join("standards/capabilities"));
        push_unique_dir(&mut dirs, cwd.join("../standards/capabilities"));
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    push_unique_dir(&mut dirs, manifest_dir.join("../../standards/capabilities"));
    push_unique_dir(
        &mut dirs,
        manifest_dir.join("../../../standards/capabilities"),
    );

    dirs
}

fn resolve_catalog_dir() -> Result<PathBuf, AppError> {
    for candidate in candidate_catalog_dirs() {
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }

    Err(AppError::internal(
        "capability catalog directory not found under standards/capabilities",
    ))
}

fn resolve_catalog_files(dir: &Path) -> Result<Vec<PathBuf>, AppError> {
    let mut files = fs::read_dir(dir)
        .map_err(|error| {
            AppError::internal(format!(
                "failed to read capability catalog directory {}: {error}",
                dir.display()
            ))
        })?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();

    files.sort();
    if files.is_empty() {
        return Err(AppError::internal(format!(
            "capability catalog is empty: {}",
            dir.display()
        )));
    }

    Ok(files)
}

fn build_catalog_signature(files: &[PathBuf]) -> Result<String, AppError> {
    let mut parts = Vec::with_capacity(files.len());
    for file in files {
        let stat = fs::metadata(file).map_err(|error| {
            AppError::internal(format!(
                "failed to stat capability catalog {}: {error}",
                file.display()
            ))
        })?;
        let modified = stat
            .modified()
            .ok()
            .and_then(|stamp| stamp.duration_since(UNIX_EPOCH).ok())
            .map(|elapsed| elapsed.as_millis())
            .unwrap_or(0);
        parts.push(format!("{}:{}:{}", file.display(), stat.len(), modified));
    }
    Ok(parts.join("|"))
}

fn normalize_entry(
    raw: RawCatalogEntry,
    file: &Path,
    index: usize,
) -> Result<BuiltinCapabilityCatalogEntry, AppError> {
    let provider = raw.provider.trim().to_string();
    let model_id = raw.model_id.trim().to_string();
    if provider.is_empty() || model_id.is_empty() {
        return Err(AppError::invalid_params(format!(
            "capability catalog entry missing provider/modelId: {}#{index}",
            file.display()
        )));
    }

    let issues = validate_model_capabilities(raw.model_type, raw.capabilities.as_ref());
    if let Some(issue) = issues.first() {
        return Err(AppError::invalid_params(format!(
            "capability catalog invalid at {}#{index}: {} {} {}",
            file.display(),
            issue.code.as_str(),
            issue.field,
            issue.message
        )));
    }

    let capabilities = match raw.capabilities {
        Some(value) => Some(serde_json::from_value::<ModelCapabilities>(value).map_err(
            |error| {
                AppError::invalid_params(format!(
                    "capability catalog parse error at {}#{index}: {error}",
                    file.display()
                ))
            },
        )?),
        None => None,
    };

    Ok(BuiltinCapabilityCatalogEntry {
        model_type: raw.model_type,
        provider,
        model_id,
        capabilities,
    })
}

fn build_cache(
    entries: Vec<BuiltinCapabilityCatalogEntry>,
    signature: String,
) -> Result<CatalogCache, AppError> {
    let mut exact = HashMap::new();
    let mut by_provider_key = HashMap::new();

    for entry in &entries {
        let model_key = compose_model_key(&entry.provider, &entry.model_id).ok_or_else(|| {
            AppError::invalid_params(
                "invalid provider/modelId pair while building capability catalog",
            )
        })?;
        let exact_key = format!("{}::{model_key}", model_type_key(entry.model_type));
        if exact.contains_key(&exact_key) {
            return Err(AppError::invalid_params(format!(
                "duplicate capability catalog entry: {exact_key}"
            )));
        }
        exact.insert(exact_key, entry.clone());

        let fallback_key = format!(
            "{}::{}::{}",
            model_type_key(entry.model_type),
            provider_key(&entry.provider),
            entry.model_id
        );
        by_provider_key
            .entry(fallback_key)
            .or_insert_with(|| entry.clone());
    }

    Ok(CatalogCache {
        signature,
        entries,
        exact,
        by_provider_key,
    })
}

fn load_catalog() -> Result<CatalogCache, AppError> {
    let dir = resolve_catalog_dir()?;
    let files = resolve_catalog_files(&dir)?;
    let signature = build_catalog_signature(&files)?;

    {
        let guard = CATALOG_CACHE
            .lock()
            .map_err(|_| AppError::internal("capability catalog cache poisoned"))?;
        if let Some(cache) = guard.as_ref()
            && cache.signature == signature
        {
            return Ok(cache.clone());
        }
    }

    let mut entries = Vec::new();
    for file in files {
        let content = fs::read_to_string(&file).map_err(|error| {
            AppError::internal(format!(
                "failed to read capability catalog {}: {error}",
                file.display()
            ))
        })?;
        let parsed = serde_json::from_str::<Vec<RawCatalogEntry>>(&content).map_err(|error| {
            AppError::invalid_params(format!(
                "capability catalog file must be json array: {} ({error})",
                file.display()
            ))
        })?;
        for (index, raw) in parsed.into_iter().enumerate() {
            entries.push(normalize_entry(raw, &file, index)?);
        }
    }

    let cache = build_cache(entries, signature)?;
    CATALOG_CACHE
        .lock()
        .map_err(|_| AppError::internal("capability catalog cache poisoned"))?
        .replace(cache.clone());

    Ok(cache)
}

fn resolve_provider_alias(provider_key_raw: &str) -> Option<&'static str> {
    PROVIDER_ALIASES
        .iter()
        .find_map(|(source, target)| (*source == provider_key_raw).then_some(*target))
}

pub fn list_builtin_capability_catalog() -> Result<Vec<BuiltinCapabilityCatalogEntry>, AppError> {
    Ok(load_catalog()?.entries)
}

pub fn find_builtin_capability_catalog_entry(
    model_type: UnifiedModelType,
    provider: &str,
    model_id: &str,
) -> Result<Option<BuiltinCapabilityCatalogEntry>, AppError> {
    let cache = load_catalog()?;
    let model_key = match compose_model_key(provider, model_id) {
        Some(value) => value,
        None => return Ok(None),
    };

    let exact_key = format!("{}::{model_key}", model_type_key(model_type));
    if let Some(entry) = cache.exact.get(&exact_key) {
        return Ok(Some(entry.clone()));
    }

    let provider_key_raw = provider_key(provider);
    let fallback_key = format!(
        "{}::{}::{}",
        model_type_key(model_type),
        provider_key_raw,
        model_id.trim()
    );
    if let Some(entry) = cache.by_provider_key.get(&fallback_key) {
        return Ok(Some(entry.clone()));
    }

    if let Some(alias_target) = resolve_provider_alias(&provider_key_raw) {
        let alias_key = format!(
            "{}::{}::{}",
            model_type_key(model_type),
            alias_target,
            model_id.trim()
        );
        if let Some(entry) = cache.by_provider_key.get(&alias_key) {
            return Ok(Some(entry.clone()));
        }
    }

    Ok(None)
}

pub fn find_builtin_capabilities(
    model_type: UnifiedModelType,
    provider: &str,
    model_id: &str,
) -> Result<Option<ModelCapabilities>, AppError> {
    Ok(
        find_builtin_capability_catalog_entry(model_type, provider, model_id)?
            .and_then(|entry| entry.capabilities),
    )
}

#[cfg(test)]
#[allow(dead_code)]
pub fn reset_builtin_capability_catalog_cache_for_test() {
    if let Ok(mut cache) = CATALOG_CACHE.lock() {
        cache.take();
    }
}
