import type { AdminAiConfig, AdminDefaultModels, ModelType } from '../../../api/admin';

const ALLOWED_PROVIDER_BASE_KEYS = ['fal', 'qwen', 'openai-compatible', 'gemini-compatible'] as const;

type AllowedProviderBaseKey = (typeof ALLOWED_PROVIDER_BASE_KEYS)[number];
const DEFAULT_MODEL_FIELD_TYPES: Record<keyof AdminDefaultModels, ModelType> = {
  analysisModel: 'llm',
  characterModel: 'image',
  locationModel: 'image',
  storyboardModel: 'image',
  editModel: 'image',
  videoModel: 'video',
  lipSyncModel: 'lipsync',
};

function providerBaseKey(value: string): string {
  return value.split(':')[0] ?? value;
}

function isAllowedProviderBaseKey(value: string): value is AllowedProviderBaseKey {
  return ALLOWED_PROVIDER_BASE_KEYS.includes(value as AllowedProviderBaseKey);
}

function parseModelKeyStrict(value: string): [provider: string, modelId: string] | null {
  const trimmed = value.trim();
  const parts = trimmed.split('::');
  if (parts.length !== 2) {
    return null;
  }

  const provider = parts[0]?.trim() ?? '';
  const modelId = parts[1]?.trim() ?? '';
  if (!provider || !modelId) {
    return null;
  }

  return [provider, modelId];
}

export function validateAdminAiConfig(config: AdminAiConfig | null): Record<string, string> {
  if (!config) {
    return {};
  }

  const errors: Record<string, string> = {};

  for (const [index, provider] of config.providers.entries()) {
    const id = provider.id.trim();
    const name = provider.name.trim();
    const baseKey = providerBaseKey(id);
    const mode = provider.apiMode?.trim() ?? '';

    if (!id) {
      errors[`providers[${index}].id`] = 'Provider ID is required.';
    }

    if (!name) {
      errors[`providers[${index}].name`] = 'Provider name is required.';
    }

    if (id && !isAllowedProviderBaseKey(baseKey)) {
      errors[`providers[${index}].id`] = 'Provider key is not allowed.';
    }

    if (mode && mode !== 'gemini-sdk') {
      errors[`providers[${index}].apiMode`] = 'API mode must be gemini-sdk when provided.';
    }

    const needsBaseUrl = baseKey === 'openai-compatible' || baseKey === 'gemini-compatible';
    if (needsBaseUrl && !(provider.baseUrl ?? '').trim()) {
      errors[`providers[${index}].baseUrl`] = 'Base URL is required for compatible providers.';
    }
  }

  for (const [index, model] of config.models.entries()) {
    const modelId = model.modelId.trim();
    const provider = model.provider.trim();
    const parsed = parseModelKeyStrict(model.modelKey);
    const providerBase = providerBaseKey(provider);

    if (!modelId) {
      errors[`models[${index}].modelId`] = 'Model ID is required.';
    }

    if (!parsed) {
      errors[`models[${index}].modelKey`] = 'Model key must be in provider::modelId format.';
    } else if (parsed[0] !== provider || parsed[1] !== modelId) {
      errors[`models[${index}].modelKey`] = 'Model key must match provider and model ID.';
    }

    if (!isAllowedProviderBaseKey(providerBase)) {
      errors[`models[${index}].provider`] = 'Model provider key is not allowed.';
    }

    if (!Number.isFinite(model.price) || model.price < 0) {
      errors[`models[${index}].price`] = 'Price must be greater than or equal to 0.';
    }
  }

  const enabledModelByKey = new Map(
    config.models
      .filter((model) => model.enabled)
      .map((model) => [model.modelKey.trim(), model] as const),
  );

  for (const [field, expectedType] of Object.entries(DEFAULT_MODEL_FIELD_TYPES) as Array<
    [keyof AdminDefaultModels, ModelType]
  >) {
    const selectedModelKey = config.defaultModels?.[field]?.trim() ?? '';
    if (!selectedModelKey) {
      continue;
    }

    const matchedModel = enabledModelByKey.get(selectedModelKey);
    if (!matchedModel) {
      errors[`defaultModels.${field}`] = 'Default model must reference an enabled model.';
      continue;
    }

    if (matchedModel.type !== expectedType) {
      errors[`defaultModels.${field}`] = `Default model type must be ${expectedType}.`;
    }
  }

  return errors;
}
