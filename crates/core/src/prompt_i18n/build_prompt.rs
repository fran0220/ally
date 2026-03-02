use std::collections::BTreeSet;

use super::{
    BuildPromptInput, PromptId,
    catalog::prompt_catalog_entry,
    errors::{PromptI18nError, PromptI18nErrorCode},
    template_store::get_prompt_template,
};

fn is_placeholder_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn extract_placeholders(template: &str) -> Vec<String> {
    let bytes = template.as_bytes();
    let mut index = 0usize;
    let mut keys = BTreeSet::new();

    while index < bytes.len() {
        if bytes[index] != b'{' {
            index += 1;
            continue;
        }

        let double = index + 1 < bytes.len() && bytes[index + 1] == b'{';
        let start = index + if double { 2 } else { 1 };
        let mut end = start;

        while end < bytes.len() {
            if double {
                if bytes[end] == b'}' && end + 1 < bytes.len() && bytes[end + 1] == b'}' {
                    break;
                }
            } else if bytes[end] == b'}' {
                break;
            }
            end += 1;
        }

        if end >= bytes.len() {
            index += 1;
            continue;
        }

        if let Some(candidate) = template.get(start..end)
            && is_placeholder_key(candidate)
        {
            keys.insert(candidate.to_string());
        }

        index = if double { end + 2 } else { end + 1 };
    }

    keys.into_iter().collect()
}

fn replace_all_placeholders(template: String, key: &str, value: &str) -> String {
    template
        .replace(&format!("{{{{{key}}}}}"), value)
        .replace(&format!("{{{key}}}"), value)
}

fn build_placeholder_error(
    code: PromptI18nErrorCode,
    prompt_id: PromptId,
    key: &str,
    message: String,
) -> PromptI18nError {
    PromptI18nError::new(code, prompt_id, message).with_detail("key", key.to_string())
}

pub fn build_prompt(input: BuildPromptInput<'_>) -> Result<String, PromptI18nError> {
    let entry = prompt_catalog_entry(input.prompt_id);
    let template = get_prompt_template(input.prompt_id, input.locale)?;

    let placeholders = extract_placeholders(&template);
    for key in placeholders {
        if !entry.variable_keys.contains(&key.as_str()) {
            return Err(build_placeholder_error(
                PromptI18nErrorCode::PromptPlaceholderMismatch,
                input.prompt_id,
                &key,
                format!("template placeholder is not declared in catalog: {key}"),
            ));
        }
    }

    for key in input.variables.keys() {
        if !entry.variable_keys.contains(&key.as_str()) {
            return Err(build_placeholder_error(
                PromptI18nErrorCode::PromptVariableUnexpected,
                input.prompt_id,
                key,
                format!("unexpected prompt variable: {key}"),
            ));
        }
    }

    for key in entry.variable_keys {
        if !input.variables.contains_key(*key) {
            return Err(build_placeholder_error(
                PromptI18nErrorCode::PromptVariableMissing,
                input.prompt_id,
                key,
                format!("missing prompt variable: {key}"),
            ));
        }
    }

    let mut rendered = template;
    for key in entry.variable_keys {
        let value = input.variables.get(*key).ok_or_else(|| {
            build_placeholder_error(
                PromptI18nErrorCode::PromptVariableMissing,
                input.prompt_id,
                key,
                format!("missing prompt variable: {key}"),
            )
        })?;
        rendered = replace_all_placeholders(rendered, key, value);
    }

    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt_i18n::{PromptIds, PromptLocale, PromptVariables};

    #[test]
    fn build_prompt_replaces_placeholders() {
        let mut variables = PromptVariables::new();
        variables.insert("user_input".to_string(), "character draft".to_string());

        let result = build_prompt(BuildPromptInput {
            prompt_id: PromptIds::NP_CHARACTER_CREATE,
            locale: PromptLocale::En,
            variables: &variables,
        });

        // Skip if prompt template files are not available (CI environment)
        let rendered = match result {
            Ok(v) => v,
            Err(ref e) if e.code == PromptI18nErrorCode::PromptTemplateNotFound => return,
            Err(e) => panic!("unexpected error: {e:?}"),
        };

        assert!(rendered.contains("character draft"));
        assert!(!rendered.contains("{user_input}"));
        assert!(!rendered.contains("{{user_input}}"));
    }

    #[test]
    fn build_prompt_rejects_unexpected_variable() {
        let mut variables = PromptVariables::new();
        variables.insert("user_input".to_string(), "content".to_string());
        variables.insert("extra".to_string(), "boom".to_string());

        let result = build_prompt(BuildPromptInput {
            prompt_id: PromptIds::NP_CHARACTER_CREATE,
            locale: PromptLocale::Zh,
            variables: &variables,
        });

        // Skip if prompt template files are not available (CI environment)
        match result {
            Err(ref e) if e.code == PromptI18nErrorCode::PromptTemplateNotFound => return,
            Err(ref e) => {
                assert_eq!(
                    e.code,
                    PromptI18nErrorCode::PromptVariableUnexpected,
                    "error code should match"
                );
            }
            Ok(_) => panic!("expected error for unexpected variable"),
        }
    }
}
