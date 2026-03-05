import type { AdminAiConfig, AdminModel, AdminProvider, ModelType } from '../../../api/admin';

import { validateAdminAiConfig } from './validation';

export const MODEL_TYPES: ModelType[] = ['llm', 'image', 'video', 'audio', 'lipsync'];
export const PROVIDER_BASE_KEYS = ['openai-compatible', 'gemini-compatible', 'fal', 'qwen'] as const;

export type ProviderBaseKey = (typeof PROVIDER_BASE_KEYS)[number];

export interface ProviderPreset {
  label: string;
  needsBaseUrl: boolean;
  supportsGeminiMode: boolean;
}

export const PROVIDER_PRESETS: Record<ProviderBaseKey, ProviderPreset> = {
  'openai-compatible': {
    label: 'OpenAI Compatible',
    needsBaseUrl: true,
    supportsGeminiMode: false,
  },
  'gemini-compatible': {
    label: 'Gemini Compatible',
    needsBaseUrl: true,
    supportsGeminiMode: true,
  },
  fal: {
    label: 'fal.ai',
    needsBaseUrl: false,
    supportsGeminiMode: false,
  },
  qwen: {
    label: 'Qwen',
    needsBaseUrl: false,
    supportsGeminiMode: false,
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
  | { field: 'apiKey'; value: string }
  | { field: 'apiMode'; value: 'gemini-sdk' | undefined };

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
    apiMode: preset.supportsGeminiMode ? 'gemini-sdk' : undefined,
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
      const nextDraft = action.payload;
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
          case 'apiMode':
            return { ...provider, apiMode: action.payload.value };
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
      const nextModels = state.draft.models.filter((model) => model.provider !== provider.id);

      return withValidation(state, {
        ...state.draft,
        providers: nextProviders,
        models: nextModels,
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

      return withValidation(state, {
        ...state.draft,
        models: nextModels,
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

      return withValidation(state, {
        ...state.draft,
        models: state.draft.models.filter((_, index) => index !== action.payload.index),
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
