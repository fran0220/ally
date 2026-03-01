import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';

import {
  type AdminAiConfig,
  type AdminModel,
  type AdminProvider,
  type ModelType,
  getAdminAiConfig,
  updateAdminAiConfig,
} from '../../api/admin';
import { GlassButton, GlassChip, GlassField, GlassInput, GlassSurface } from '../../components/ui/primitives';
import { queryKeys } from '../../lib/query-keys';

const MODEL_TYPES: ModelType[] = ['llm', 'image', 'video', 'audio', 'lipsync'];
const PROVIDER_BASE_KEYS = ['openai-compatible', 'gemini-compatible', 'fal', 'qwen'] as const;

interface ProviderPreset {
  label: string;
  needsBaseUrl: boolean;
  supportsGeminiMode: boolean;
}

const PROVIDER_PRESETS: Record<(typeof PROVIDER_BASE_KEYS)[number], ProviderPreset> = {
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

function providerBaseKey(id: string): string {
  return id.split(':')[0] ?? id;
}

function modelKey(providerId: string, modelId: string): string {
  return `${providerId.trim()}::${modelId.trim()}`;
}

function generateProviderId(baseKey: string, providers: AdminProvider[]): string {
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

function addProvider(draft: AdminAiConfig, baseKey: (typeof PROVIDER_BASE_KEYS)[number]): AdminAiConfig {
  const nextId = generateProviderId(baseKey, draft.providers);
  const preset = PROVIDER_PRESETS[baseKey];
  const provider: AdminProvider = {
    id: nextId,
    name: `${preset.label} ${draft.providers.length + 1}`,
    baseUrl: preset.needsBaseUrl ? '' : undefined,
    apiKey: '',
    apiMode: preset.supportsGeminiMode ? 'gemini-sdk' : undefined,
  };
  return { ...draft, providers: [...draft.providers, provider] };
}

function updateProviderAtIndex(
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

function updateModelAtIndex(
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

export function AiConfig() {
  const { t } = useTranslation(['apiConfig', 'common']);
  const queryClient = useQueryClient();

  const configQuery = useQuery({
    queryKey: queryKeys.admin.aiConfig(),
    queryFn: getAdminAiConfig,
  });

  const [draft, setDraft] = useState<AdminAiConfig | null>(null);
  const [activeModelType, setActiveModelType] = useState<ModelType>('llm');

  useEffect(() => {
    if (configQuery.data) {
      setDraft(configQuery.data);
    }
  }, [configQuery.data]);

  const saveMutation = useMutation({
    mutationFn: updateAdminAiConfig,
    onSuccess: (next) => {
      queryClient.setQueryData(queryKeys.admin.aiConfig(), next);
      setDraft(next);
    },
  });

  const isDirty = useMemo(() => {
    if (!draft || !configQuery.data) {
      return false;
    }
    return JSON.stringify(draft) !== JSON.stringify(configQuery.data);
  }, [draft, configQuery.data]);

  const filteredModels = useMemo(() => {
    if (!draft) {
      return [];
    }
    return draft.models.filter((model) => model.type === activeModelType);
  }, [activeModelType, draft]);

  if (!draft) {
    return (
      <main className="page-shell py-10">
        <GlassSurface>{configQuery.isLoading ? t('common:loading') : 'No config loaded'}</GlassSurface>
      </main>
    );
  }

  return (
    <main className="page-shell py-8 md:py-10">
      <header className="mb-6 flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="glass-page-title">{t('apiConfig:title')}</h1>
          <p className="glass-page-subtitle">Rust Admin Config · openai-compatible / gemini-compatible / fal / qwen</p>
        </div>
        <GlassButton
          variant="primary"
          loading={saveMutation.isPending}
          disabled={!isDirty || saveMutation.isPending}
          onClick={() => {
            void saveMutation.mutateAsync(draft);
          }}
        >
          {saveMutation.isPending ? t('apiConfig:saving') : t('apiConfig:save')}
        </GlassButton>
      </header>

      {configQuery.error instanceof Error ? (
        <p className="mb-4 text-sm text-[var(--glass-tone-danger-fg)]">{configQuery.error.message}</p>
      ) : null}
      {saveMutation.error instanceof Error ? (
        <p className="mb-4 text-sm text-[var(--glass-tone-danger-fg)]">{saveMutation.error.message}</p>
      ) : null}

      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
        <GlassSurface>
          <div className="mb-3 flex items-center justify-between">
            <h2 className="text-sm font-semibold text-[var(--glass-text-secondary)]">Provider Pool</h2>
            <div className="flex flex-wrap gap-1">
              {PROVIDER_BASE_KEYS.map((baseKey) => (
                <GlassButton
                  key={baseKey}
                  size="sm"
                  variant="soft"
                  onClick={() => setDraft((previous) => (previous ? addProvider(previous, baseKey) : previous))}
                >
                  + {baseKey}
                </GlassButton>
              ))}
            </div>
          </div>

          <div className="space-y-3">
            {draft.providers.map((provider, index) => {
              const baseKey = providerBaseKey(provider.id) as keyof typeof PROVIDER_PRESETS;
              const preset = PROVIDER_PRESETS[baseKey];

              return (
                <article key={provider.id} className="glass-list-row flex-col items-stretch gap-3 p-3">
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-sm font-semibold text-[var(--glass-text-primary)]">{preset?.label ?? baseKey}</p>
                      <p className="text-xs text-[var(--glass-text-tertiary)]">{provider.id}</p>
                    </div>
                    <GlassButton
                      size="sm"
                      variant="ghost"
                      onClick={() =>
                        setDraft((previous) => {
                          if (!previous) {
                            return previous;
                          }
                          const nextProviders = previous.providers.filter((_, itemIndex) => itemIndex !== index);
                          const nextModels = previous.models.filter((model) => model.provider !== provider.id);
                          return { ...previous, providers: nextProviders, models: nextModels };
                        })
                      }
                    >
                      x
                    </GlassButton>
                  </div>

                  <GlassField label="Name">
                    <GlassInput
                      value={provider.name}
                      onChange={(event) =>
                        setDraft((previous) => {
                          if (!previous) {
                            return previous;
                          }
                          const nextProviders = updateProviderAtIndex(previous.providers, index, (item) => ({
                            ...item,
                            name: event.target.value,
                          }));
                          return { ...previous, providers: nextProviders };
                        })
                      }
                    />
                  </GlassField>

                  {preset?.needsBaseUrl ? (
                    <GlassField label="Base URL">
                      <GlassInput
                        value={provider.baseUrl ?? ''}
                        onChange={(event) =>
                          setDraft((previous) => {
                            if (!previous) {
                              return previous;
                            }
                            const nextProviders = updateProviderAtIndex(previous.providers, index, (item) => ({
                              ...item,
                              baseUrl: event.target.value,
                            }));
                            return { ...previous, providers: nextProviders };
                          })
                        }
                      />
                    </GlassField>
                  ) : null}

                  <GlassField label="API Key">
                    <GlassInput
                      type="password"
                      value={provider.apiKey ?? ''}
                      onChange={(event) =>
                        setDraft((previous) => {
                          if (!previous) {
                            return previous;
                          }
                          const nextProviders = updateProviderAtIndex(previous.providers, index, (item) => ({
                            ...item,
                            apiKey: event.target.value,
                          }));
                          return { ...previous, providers: nextProviders };
                        })
                      }
                    />
                  </GlassField>

                  {preset?.supportsGeminiMode ? (
                    <label className="flex items-center gap-2 text-xs text-[var(--glass-text-secondary)]">
                      <input
                        type="checkbox"
                        checked={provider.apiMode === 'gemini-sdk'}
                        onChange={(event) =>
                          setDraft((previous) => {
                            if (!previous) {
                              return previous;
                            }
                            const nextProviders = updateProviderAtIndex(previous.providers, index, (item) => ({
                              ...item,
                              apiMode: event.target.checked ? 'gemini-sdk' : undefined,
                            }));
                            return { ...previous, providers: nextProviders };
                          })
                        }
                      />
                      Gemini SDK mode
                    </label>
                  ) : null}
                </article>
              );
            })}
          </div>
        </GlassSurface>

        <GlassSurface>
          <div className="mb-3 flex items-center justify-between">
            <h2 className="text-sm font-semibold text-[var(--glass-text-secondary)]">Models</h2>
            <GlassButton
              size="sm"
              variant="soft"
              onClick={() => {
                const providerId = draft.providers[0]?.id ?? 'openai-compatible';
                const modelId = `new-${activeModelType}-${draft.models.length + 1}`;
                const model: AdminModel = {
                  modelId,
                  modelKey: modelKey(providerId, modelId),
                  name: modelId,
                  type: activeModelType,
                  provider: providerId,
                  enabled: true,
                  price: 0,
                };
                setDraft((previous) =>
                  previous
                    ? {
                        ...previous,
                        models: [...previous.models, model],
                      }
                    : previous,
                );
              }}
            >
              + {t('apiConfig:addModel')}
            </GlassButton>
          </div>

          <div className="mb-3 flex flex-wrap gap-2">
            {MODEL_TYPES.map((type) => (
              <GlassButton
                key={type}
                size="sm"
                variant={activeModelType === type ? 'primary' : 'soft'}
                onClick={() => setActiveModelType(type)}
              >
                {type}
              </GlassButton>
            ))}
          </div>

          <div className="space-y-2">
            {filteredModels.map((model) => {
              const modelIndex = draft.models.findIndex((item) => item.modelKey === model.modelKey);
              if (modelIndex === -1) {
                return null;
              }

              return (
                <article key={model.modelKey} className="glass-list-row flex-col items-stretch gap-3 p-3">
                  <div className="flex items-center justify-between">
                    <GlassChip tone={model.enabled ? 'success' : 'neutral'}>{model.type}</GlassChip>
                    <GlassButton
                      size="sm"
                      variant="ghost"
                      onClick={() =>
                        setDraft((previous) => {
                          if (!previous) {
                            return previous;
                          }
                          const nextModels = previous.models.filter((_, index) => index !== modelIndex);
                          return { ...previous, models: nextModels };
                        })
                      }
                    >
                      x
                    </GlassButton>
                  </div>

                  <GlassField label="Display Name">
                    <GlassInput
                      value={model.name}
                      onChange={(event) =>
                        setDraft((previous) => {
                          if (!previous) {
                            return previous;
                          }
                          const nextModels = updateModelAtIndex(previous.models, modelIndex, (item) => ({
                            ...item,
                            name: event.target.value,
                          }));
                          return { ...previous, models: nextModels };
                        })
                      }
                    />
                  </GlassField>

                  <div className="grid gap-2 md:grid-cols-2">
                    <GlassField label="Model ID">
                      <GlassInput
                        value={model.modelId}
                        onChange={(event) =>
                          setDraft((previous) => {
                            if (!previous) {
                              return previous;
                            }
                            const nextModelId = event.target.value;
                            const nextModels = updateModelAtIndex(previous.models, modelIndex, (item) => ({
                              ...item,
                              modelId: nextModelId,
                              modelKey: modelKey(item.provider, nextModelId),
                            }));
                            return { ...previous, models: nextModels };
                          })
                        }
                      />
                    </GlassField>

                    <GlassField label="Provider">
                      <select
                        className="glass-select-base h-10 px-3 text-sm"
                        value={model.provider}
                        onChange={(event) =>
                          setDraft((previous) => {
                            if (!previous) {
                              return previous;
                            }
                            const nextProvider = event.target.value;
                            const nextModels = updateModelAtIndex(previous.models, modelIndex, (item) => ({
                              ...item,
                              provider: nextProvider,
                              modelKey: modelKey(nextProvider, item.modelId),
                            }));
                            return { ...previous, models: nextModels };
                          })
                        }
                      >
                        {draft.providers.map((provider) => (
                          <option key={provider.id} value={provider.id}>
                            {provider.name}
                          </option>
                        ))}
                      </select>
                    </GlassField>

                    <GlassField label="Price">
                      <GlassInput
                        type="number"
                        min={0}
                        step={0.01}
                        value={String(model.price)}
                        onChange={(event) =>
                          setDraft((previous) => {
                            if (!previous) {
                              return previous;
                            }
                            const nextModels = updateModelAtIndex(previous.models, modelIndex, (item) => ({
                              ...item,
                              price: Number(event.target.value),
                            }));
                            return { ...previous, models: nextModels };
                          })
                        }
                      />
                    </GlassField>

                    <label className="flex items-center gap-2 self-end text-sm text-[var(--glass-text-secondary)]">
                      <input
                        type="checkbox"
                        checked={model.enabled}
                        onChange={(event) =>
                          setDraft((previous) => {
                            if (!previous) {
                              return previous;
                            }
                            const nextModels = updateModelAtIndex(previous.models, modelIndex, (item) => ({
                              ...item,
                              enabled: event.target.checked,
                            }));
                            return { ...previous, models: nextModels };
                          })
                        }
                      />
                      Enabled
                    </label>
                  </div>
                </article>
              );
            })}
            {filteredModels.length === 0 ? (
              <p className="text-sm text-[var(--glass-text-tertiary)]">No models in this category.</p>
            ) : null}
          </div>
        </GlassSurface>
      </div>
    </main>
  );
}
