import type { CapabilitySelections, CapabilityValue } from '../../../lib/model-config-contract';
import type {
  AdminAiConfig,
  AdminDefaultModels,
  AdminModel,
  AdminProvider,
  ModelType,
} from '../../../api/admin';

import { validateAdminAiConfig } from './validation';

export const MODEL_TYPES: ModelType[] = ['llm', 'image', 'video', 'audio', 'lipsync'];
export const PROVIDER_BASE_KEYS = [
  'openai-compatible',
  'gemini-compatible',
  'anthropic',
  'jimeng',
  'fal',
  'qwen',
] as const;
export const DEFAULT_MODEL_FIELDS = [
  'analysisModel',
  'characterModel',
  'locationModel',
  'storyboardModel',
  'editModel',
  'videoModel',
  'lipSyncModel',
] as const;

export type ProviderBaseKey = (typeof PROVIDER_BASE_KEYS)[number];
export type DefaultModelField = (typeof DEFAULT_MODEL_FIELDS)[number];

export interface ProviderPreset {
  label: string;
  needsBaseUrl: boolean;
}

export const PROVIDER_PRESETS: Record<ProviderBaseKey, ProviderPreset> = {
  'openai-compatible': {
    label: 'OpenAI Compatible',
    needsBaseUrl: true,
  },
  'gemini-compatible': {
    label: 'Gemini Compatible',
    needsBaseUrl: true,
  },
  anthropic: {
    label: 'Anthropic',
    needsBaseUrl: true,
  },
  jimeng: {
    label: 'Jimeng Video',
    needsBaseUrl: true,
  },
  fal: {
    label: 'fal.ai',
    needsBaseUrl: false,
  },
  qwen: {
    label: 'Qwen',
    needsBaseUrl: false,
  },
};

export interface AdminAiConfigState {
  draft: AdminAiConfig | null;
  initial: AdminAiConfig | null;
  activeModelType: ModelType;
  errors: Record<string, string>;
}

export type ProviderFieldUpdate =
  | { field: 'name'; value: string }
  | { field: 'baseUrl'; value: string }
  | { field: 'apiKey'; value: string };

export type ModelFieldUpdate =
  | { field: 'name'; value: string }
  | { field: 'modelId'; value: string }
  | { field: 'provider'; value: string }
  | { field: 'price'; value: number }
  | { field: 'enabled'; value: boolean };

export type AdminAiConfigAction =
  | { type: 'LOAD_FROM_SERVER'; payload: AdminAiConfig }
  | { type: 'PROVIDER_ADD'; payload: { baseKey: ProviderBaseKey } }
  | { type: 'PROVIDER_UPDATE_FIELD'; payload: { index: number } & ProviderFieldUpdate }
  | { type: 'PROVIDER_DELETE'; payload: { index: number } }
  | { type: 'MODEL_ADD' }
  | { type: 'MODEL_UPDATE_FIELD'; payload: { index: number } & ModelFieldUpdate }
  | { type: 'MODEL_DELETE'; payload: { index: number } }
  | { type: 'DEFAULT_MODEL_UPDATE'; payload: { field: DefaultModelField; value: string } }
  | {
      type: 'CAPABILITY_DEFAULT_UPDATE';
      payload: { modelKey: string; field: string; value: CapabilityValue | null };
    }
  | { type: 'SET_ACTIVE_MODEL_TYPE'; payload: { modelType: ModelType } };

export const adminAiConfigInitialState: AdminAiConfigState = {
  draft: null,
  initial: null,
  activeModelType: 'llm',
  errors: {},
};

export function providerBaseKey(id: string): string {
  return id.split(':')[0] ?? id;
}

export function isProviderBaseKey(value: string): value is ProviderBaseKey {
  return PROVIDER_BASE_KEYS.includes(value as ProviderBaseKey);
}

export function modelKey(providerId: string, modelId: string): string {
  return `${providerId.trim()}::${modelId.trim()}`;
}

export function generateProviderId(baseKey: string, providers: AdminProvider[]): string {
  const hasBase = providers.some((provider) => providerBaseKey(provider.id) === baseKey);
  if (!hasBase) {
    return baseKey;
  }

  let suffix = 2;
  while (providers.some((provider) => provider.id === `${baseKey}:${suffix}`)) {
    suffix += 1;
  }

  return `${baseKey}:${suffix}`;
}

export function addProvider(draft: AdminAiConfig, baseKey: ProviderBaseKey): AdminAiConfig {
  const nextId = generateProviderId(baseKey, draft.providers);
  const preset = PROVIDER_PRESETS[baseKey];
  const provider: AdminProvider = {
    id: nextId,
    name: `${preset.label} ${draft.providers.length + 1}`,
    baseUrl: preset.needsBaseUrl ? '' : undefined,
    apiKey: '',
  };

  return {
    ...draft,
    providers: [...draft.providers, provider],
  };
}

export function updateProviderAtIndex(
  providers: AdminProvider[],
  index: number,
  updater: (provider: AdminProvider) => AdminProvider,
): AdminProvider[] {
  const current = providers[index];
  if (!current) {
    return providers;
  }

  const nextProviders = [...providers];
  nextProviders[index] = updater(current);
  return nextProviders;
}

export function updateModelAtIndex(
  models: AdminModel[],
  index: number,
  updater: (model: AdminModel) => AdminModel,
): AdminModel[] {
  const current = models[index];
  if (!current) {
    return models;
  }

  const nextModels = [...models];
  nextModels[index] = updater(current);
  return nextModels;
}

function normalizeAiConfig(payload: AdminAiConfig): AdminAiConfig {
  return {
    ...payload,
    defaultModels: { ...(payload.defaultModels ?? {}) },
    capabilityDefaults: { ...(payload.capabilityDefaults ?? {}) },
  };
}

function remapDefaultModelSelection(
  defaultModels: AdminDefaultModels | undefined,
  fromModelKey: string,
  toModelKey: string,
): AdminDefaultModels | undefined {
  if (!defaultModels || fromModelKey === toModelKey) {
    return defaultModels;
  }

  let changed = false;
  const nextDefaultModels: AdminDefaultModels = { ...defaultModels };
  for (const field of DEFAULT_MODEL_FIELDS) {
    if ((nextDefaultModels[field] ?? '') === fromModelKey) {
      nextDefaultModels[field] = toModelKey;
      changed = true;
    }
  }

  return changed ? nextDefaultModels : defaultModels;
}

function removeModelSelection(
  defaultModels: AdminDefaultModels | undefined,
  removedModelKey: string,
): AdminDefaultModels | undefined {
  if (!defaultModels) {
    return defaultModels;
  }

  let changed = false;
  const nextDefaultModels: AdminDefaultModels = { ...defaultModels };
  for (const field of DEFAULT_MODEL_FIELDS) {
    if ((nextDefaultModels[field] ?? '') === removedModelKey) {
      delete nextDefaultModels[field];
      changed = true;
    }
  }

  return changed ? nextDefaultModels : defaultModels;
}

function remapCapabilityDefaultsModelKey(
  capabilityDefaults: CapabilitySelections | undefined,
  fromModelKey: string,
  toModelKey: string,
): CapabilitySelections | undefined {
  if (!capabilityDefaults || fromModelKey === toModelKey) {
    return capabilityDefaults;
  }

  const currentValues = capabilityDefaults[fromModelKey];
  if (!currentValues) {
    return capabilityDefaults;
  }

  const nextCapabilityDefaults: CapabilitySelections = { ...capabilityDefaults };
  delete nextCapabilityDefaults[fromModelKey];

  nextCapabilityDefaults[toModelKey] = {
    ...(nextCapabilityDefaults[toModelKey] ?? {}),
    ...currentValues,
  };

  return nextCapabilityDefaults;
}

function removeCapabilityDefaultsForModels(
  capabilityDefaults: CapabilitySelections | undefined,
  removedModelKeys: readonly string[],
): CapabilitySelections | undefined {
  if (!capabilityDefaults || removedModelKeys.length === 0) {
    return capabilityDefaults;
  }

  const removedSet = new Set(removedModelKeys);
  let changed = false;
  const nextCapabilityDefaults: CapabilitySelections = {};
  for (const [modelKeyValue, values] of Object.entries(capabilityDefaults)) {
    if (removedSet.has(modelKeyValue)) {
      changed = true;
      continue;
    }
    nextCapabilityDefaults[modelKeyValue] = values;
  }

  return changed ? nextCapabilityDefaults : capabilityDefaults;
}

function withValidation(state: AdminAiConfigState, nextDraft: AdminAiConfig | null): AdminAiConfigState {
  return {
    ...state,
    draft: nextDraft,
    errors: validateAdminAiConfig(nextDraft),
  };
}

export function adminAiConfigReducer(state: AdminAiConfigState, action: AdminAiConfigAction): AdminAiConfigState {
  switch (action.type) {
    case 'LOAD_FROM_SERVER': {
      const nextDraft = normalizeAiConfig(action.payload);
      return {
        ...state,
        draft: nextDraft,
        initial: nextDraft,
        errors: validateAdminAiConfig(nextDraft),
      };
    }

    case 'PROVIDER_ADD': {
      if (!state.draft) {
        return state;
      }

      return withValidation(state, addProvider(state.draft, action.payload.baseKey));
    }

    case 'PROVIDER_UPDATE_FIELD': {
      if (!state.draft) {
        return state;
      }

      const { index } = action.payload;
      const nextProviders = updateProviderAtIndex(state.draft.providers, index, (provider) => {
        switch (action.payload.field) {
          case 'name':
            return { ...provider, name: action.payload.value };
          case 'baseUrl':
            return { ...provider, baseUrl: action.payload.value };
          case 'apiKey':
            return { ...provider, apiKey: action.payload.value };
        }
      });

      if (nextProviders === state.draft.providers) {
        return state;
      }

      return withValidation(state, {
        ...state.draft,
        providers: nextProviders,
      });
    }

    case 'PROVIDER_DELETE': {
      if (!state.draft) {
        return state;
      }

      const provider = state.draft.providers[action.payload.index];
      if (!provider) {
        return state;
      }

      const nextProviders = state.draft.providers.filter((_, index) => index !== action.payload.index);
      const removedModelKeys = state.draft.models
        .filter((model) => model.provider === provider.id)
        .map((model) => model.modelKey);
      const nextModels = state.draft.models.filter((model) => model.provider !== provider.id);
      const nextDefaultModels = removedModelKeys.reduce<AdminDefaultModels | undefined>(
        (current, modelKeyValue) => removeModelSelection(current, modelKeyValue),
        state.draft.defaultModels,
      );
      const nextCapabilityDefaults = removeCapabilityDefaultsForModels(
        state.draft.capabilityDefaults,
        removedModelKeys,
      );

      return withValidation(state, {
        ...state.draft,
        providers: nextProviders,
        models: nextModels,
        defaultModels: nextDefaultModels,
        capabilityDefaults: nextCapabilityDefaults,
      });
    }

    case 'MODEL_ADD': {
      if (!state.draft) {
        return state;
      }

      const providerId = state.draft.providers[0]?.id ?? 'openai-compatible';
      const modelId = `new-${state.activeModelType}-${state.draft.models.length + 1}`;
      const model: AdminModel = {
        modelId,
        modelKey: modelKey(providerId, modelId),
        name: modelId,
        type: state.activeModelType,
        provider: providerId,
        enabled: true,
        price: 0,
      };

      return withValidation(state, {
        ...state.draft,
        models: [...state.draft.models, model],
      });
    }

    case 'MODEL_UPDATE_FIELD': {
      if (!state.draft) {
        return state;
      }

      const { index } = action.payload;
      const nextModels = updateModelAtIndex(state.draft.models, index, (model) => {
        switch (action.payload.field) {
          case 'name':
            return { ...model, name: action.payload.value };
          case 'modelId': {
            const nextModelId = action.payload.value;
            return {
              ...model,
              modelId: nextModelId,
              modelKey: modelKey(model.provider, nextModelId),
            };
          }
          case 'provider': {
            const nextProvider = action.payload.value;
            return {
              ...model,
              provider: nextProvider,
              modelKey: modelKey(nextProvider, model.modelId),
            };
          }
          case 'price':
            return { ...model, price: action.payload.value };
          case 'enabled':
            return { ...model, enabled: action.payload.value };
        }
      });

      if (nextModels === state.draft.models) {
        return state;
      }

      const previousModel = state.draft.models[index];
      const updatedModel = nextModels[index];

      let nextDefaultModels = state.draft.defaultModels;
      let nextCapabilityDefaults = state.draft.capabilityDefaults;

      if (previousModel && updatedModel && previousModel.modelKey !== updatedModel.modelKey) {
        nextDefaultModels = remapDefaultModelSelection(
          nextDefaultModels,
          previousModel.modelKey,
          updatedModel.modelKey,
        );
        nextCapabilityDefaults = remapCapabilityDefaultsModelKey(
          nextCapabilityDefaults,
          previousModel.modelKey,
          updatedModel.modelKey,
        );
      }

      if (updatedModel && action.payload.field === 'enabled' && !action.payload.value) {
        nextDefaultModels = removeModelSelection(nextDefaultModels, updatedModel.modelKey);
      }

      return withValidation(state, {
        ...state.draft,
        models: nextModels,
        defaultModels: nextDefaultModels,
        capabilityDefaults: nextCapabilityDefaults,
      });
    }

    case 'MODEL_DELETE': {
      if (!state.draft) {
        return state;
      }

      const model = state.draft.models[action.payload.index];
      if (!model) {
        return state;
      }

      const nextDefaultModels = removeModelSelection(state.draft.defaultModels, model.modelKey);
      const nextCapabilityDefaults = removeCapabilityDefaultsForModels(state.draft.capabilityDefaults, [model.modelKey]);

      return withValidation(state, {
        ...state.draft,
        models: state.draft.models.filter((_, index) => index !== action.payload.index),
        defaultModels: nextDefaultModels,
        capabilityDefaults: nextCapabilityDefaults,
      });
    }

    case 'DEFAULT_MODEL_UPDATE': {
      if (!state.draft) {
        return state;
      }

      const { field, value } = action.payload;
      const currentValue = state.draft.defaultModels?.[field] ?? '';
      if (currentValue === value) {
        return state;
      }

      const nextDefaultModels: AdminDefaultModels = {
        ...(state.draft.defaultModels ?? {}),
      };

      if (value.trim()) {
        nextDefaultModels[field] = value;
      } else {
        delete nextDefaultModels[field];
      }

      return withValidation(state, {
        ...state.draft,
        defaultModels: nextDefaultModels,
      });
    }

    case 'CAPABILITY_DEFAULT_UPDATE': {
      if (!state.draft) {
        return state;
      }

      const { modelKey: rawModelKey, field, value } = action.payload;
      const modelKeyValue = rawModelKey.trim();
      if (!modelKeyValue) {
        return state;
      }

      const currentValue = state.draft.capabilityDefaults?.[modelKeyValue]?.[field];
      if ((value === null && currentValue === undefined) || currentValue === value) {
        return state;
      }

      const nextCapabilityDefaults: CapabilitySelections = {
        ...(state.draft.capabilityDefaults ?? {}),
      };
      const nextValues = {
        ...(nextCapabilityDefaults[modelKeyValue] ?? {}),
      };

      if (value === null) {
        delete nextValues[field];
      } else {
        nextValues[field] = value;
      }

      if (Object.keys(nextValues).length === 0) {
        delete nextCapabilityDefaults[modelKeyValue];
      } else {
        nextCapabilityDefaults[modelKeyValue] = nextValues;
      }

      return withValidation(state, {
        ...state.draft,
        capabilityDefaults: nextCapabilityDefaults,
      });
    }

    case 'SET_ACTIVE_MODEL_TYPE': {
      return {
        ...state,
        activeModelType: action.payload.modelType,
        errors: validateAdminAiConfig(state.draft),
      };
    }

    default:
      return state;
  }
}

export function isAdminAiConfigDirty(state: AdminAiConfigState): boolean {
  return state.draft !== state.initial;
}
