import { useEffect, useMemo, useReducer } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';

import type { AdminModel, ModelType } from '../../api/admin';
import { getAdminAiConfig, updateAdminAiConfig } from '../../api/admin';
import { ModelCapabilityDropdown } from '../../components/ui/config-modals/ModelCapabilityDropdown';
import { AppIcon } from '../../components/ui/icons';
import { GlassButton, GlassSurface } from '../../components/ui/primitives';
import type { CapabilityValue } from '../../lib/model-config-contract';
import { queryKeys } from '../../lib/query-keys';
import { AdminModelPanel } from './ai-config/AdminModelPanel';
import {
  adminAiConfigInitialState,
  adminAiConfigReducer,
  type DefaultModelField,
  isAdminAiConfigDirty,
} from './ai-config/AdminAiConfigReducer';
import { AdminProviderPool } from './ai-config/AdminProviderPool';

type DefaultModelCardType = Extract<ModelType, 'llm' | 'image' | 'video' | 'lipsync'>;
type DefaultModelCardIcon = 'llm' | 'image' | 'video' | 'lipsync';

interface DefaultModelCardConfig {
  field: DefaultModelField;
  modelType: DefaultModelCardType;
  titleKey: string;
  icon: DefaultModelCardIcon;
}

const MONO_ICON_BADGE =
  'inline-flex items-center justify-center rounded-[var(--glass-radius-md)] bg-[var(--glass-bg-surface)] p-1 text-[var(--glass-text-secondary)]';

const DEFAULT_MODEL_CARDS: DefaultModelCardConfig[] = [
  { field: 'analysisModel', modelType: 'llm', titleKey: 'textDefault', icon: 'llm' },
  { field: 'characterModel', modelType: 'image', titleKey: 'characterDefault', icon: 'image' },
  { field: 'locationModel', modelType: 'image', titleKey: 'locationDefault', icon: 'image' },
  { field: 'storyboardModel', modelType: 'image', titleKey: 'storyboardDefault', icon: 'image' },
  { field: 'editModel', modelType: 'image', titleKey: 'editDefault', icon: 'image' },
  { field: 'videoModel', modelType: 'video', titleKey: 'videoDefault', icon: 'video' },
  { field: 'lipSyncModel', modelType: 'lipsync', titleKey: 'lipsyncDefault', icon: 'lipsync' },
];

const Icons = {
  settings: () => <AppIcon name="settingsHex" className="h-3.5 w-3.5" />,
  llm: () => <AppIcon name="menu" className="h-3.5 w-3.5" />,
  image: () => <AppIcon name="image" className="h-3.5 w-3.5" />,
  video: () => <AppIcon name="video" className="h-3.5 w-3.5" />,
  lipsync: () => <AppIcon name="audioWave" className="h-3.5 w-3.5" />,
  chevronDown: () => <AppIcon name="chevronDown" className="h-3 w-3" />,
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

function isCapabilityValue(value: unknown): value is CapabilityValue {
  return typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean';
}

function extractCapabilityFieldsFromModel(
  capabilities: unknown,
  modelType: string,
): Array<{ field: string; options: CapabilityValue[] }> {
  if (!isRecord(capabilities)) {
    return [];
  }

  const namespace = capabilities[modelType];
  if (!isRecord(namespace)) {
    return [];
  }

  return Object.entries(namespace)
    .filter(
      ([key, value]) =>
        key.endsWith('Options') && Array.isArray(value) && value.every(isCapabilityValue) && value.length > 0,
    )
    .map(([key, value]) => ({
      field: key.slice(0, -'Options'.length),
      options: value as CapabilityValue[],
    }));
}

function parseBySample(input: string, sample: CapabilityValue): CapabilityValue {
  if (typeof sample === 'number') {
    return Number(input);
  }
  if (typeof sample === 'boolean') {
    return input === 'true';
  }
  return input;
}

function toCapabilityFieldLabel(field: string): string {
  return field.replace(/([A-Z])/g, ' $1').replace(/^./, (char) => char.toUpperCase());
}

function normalizeModelKey(rawValue: string | undefined): string {
  const trimmed = rawValue?.trim() ?? '';
  if (!trimmed) {
    return '';
  }

  const parts = trimmed.split('::');
  if (parts.length !== 2) {
    return '';
  }

  const provider = parts[0]?.trim() ?? '';
  const modelId = parts[1]?.trim() ?? '';
  if (!provider || !modelId) {
    return '';
  }

  return `${provider}::${modelId}`;
}

function findEnabledModelsByType(models: AdminModel[], modelType: DefaultModelCardType): AdminModel[] {
  return models.filter((model) => model.enabled && model.type === modelType);
}

export function AiConfig() {
  const { t } = useTranslation(['apiConfig', 'common']);
  const queryClient = useQueryClient();
  const [state, dispatch] = useReducer(adminAiConfigReducer, adminAiConfigInitialState);

  const configQuery = useQuery({
    queryKey: queryKeys.admin.aiConfig(),
    queryFn: getAdminAiConfig,
  });

  useEffect(() => {
    if (configQuery.data) {
      dispatch({ type: 'LOAD_FROM_SERVER', payload: configQuery.data });
    }
  }, [configQuery.data]);

  const saveMutation = useMutation({
    mutationFn: updateAdminAiConfig,
    onSuccess: (next) => {
      queryClient.setQueryData(queryKeys.admin.aiConfig(), next);
      dispatch({ type: 'LOAD_FROM_SERVER', payload: next });
    },
  });

  const isDirty = isAdminAiConfigDirty(state);
  const validationCount = Object.keys(state.errors).length;
  const canSave = Boolean(state.draft) && isDirty && validationCount === 0 && !saveMutation.isPending;
  const canReset = Boolean(state.initial) && isDirty && !saveMutation.isPending;

  const saveDisabledReason = useMemo(() => {
    if (saveMutation.isPending) {
      return 'Saving...';
    }
    if (!state.draft) {
      return 'No config loaded.';
    }
    if (!isDirty) {
      return 'No changes to save.';
    }
    if (validationCount > 0) {
      return `Resolve ${validationCount} validation issue${validationCount === 1 ? '' : 's'} before saving.`;
    }
    return '';
  }, [isDirty, saveMutation.isPending, state.draft, validationCount]);

  const validationMessages = useMemo(() => {
    return [...new Set(Object.values(state.errors))];
  }, [state.errors]);

  const providerNameById = useMemo(() => {
    return new Map(
      (state.draft?.providers ?? []).map((provider) => [provider.id, provider.name?.trim() || provider.id]),
    );
  }, [state.draft?.providers]);

  if (!state.draft) {
    return (
      <main className="page-shell py-10">
        <GlassSurface>{configQuery.isLoading ? t('common:loading') : 'No config loaded'}</GlassSurface>
      </main>
    );
  }

  const draft = state.draft;

  return (
    <main className="page-shell py-8 md:py-10">
      <header className="mb-6 flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="glass-page-title">{t('apiConfig:title')}</h1>
          <p className="glass-page-subtitle">Rust Admin Config · openai-compatible / gemini-compatible / fal / qwen</p>
        </div>

        <div className="flex items-center gap-2">
          <GlassButton
            size="sm"
            variant="soft"
            disabled={!canReset}
            onClick={() => {
              if (state.initial) {
                dispatch({ type: 'LOAD_FROM_SERVER', payload: state.initial });
              }
            }}
          >
            Reset
          </GlassButton>

          <span className="inline-flex" title={canSave ? '' : saveDisabledReason}>
            <GlassButton
              variant="primary"
              loading={saveMutation.isPending}
              disabled={!canSave}
              onClick={() => {
                if (!canSave) {
                  return;
                }
                void saveMutation.mutateAsync(draft);
              }}
            >
              {saveMutation.isPending ? t('apiConfig:saving') : t('apiConfig:save')}
            </GlassButton>
          </span>
        </div>
      </header>

      {configQuery.error instanceof Error ? (
        <p className="mb-4 text-sm text-[var(--glass-tone-danger-fg)]">{configQuery.error.message}</p>
      ) : null}
      {saveMutation.error instanceof Error ? (
        <p className="mb-4 text-sm text-[var(--glass-tone-danger-fg)]">{saveMutation.error.message}</p>
      ) : null}

      {validationCount > 0 ? (
        <GlassSurface className="mb-4" density="compact">
          <p className="text-sm font-semibold text-[var(--glass-tone-danger-fg)]">
            Resolve {validationCount} validation issue{validationCount === 1 ? '' : 's'} before saving.
          </p>
          <p className="mt-1 text-xs text-[var(--glass-text-secondary)]">{validationMessages.slice(0, 3).join(' ')}</p>
          {validationMessages.length > 3 ? (
            <p className="mt-1 text-xs text-[var(--glass-text-tertiary)]">+{validationMessages.length - 3} more issues.</p>
          ) : null}
        </GlassSurface>
      ) : null}

      <GlassSurface className="mb-4" density="compact">
        <div className="mb-1 flex items-center gap-2 px-1">
          <span className="glass-surface-soft inline-flex h-6 w-6 items-center justify-center rounded-[var(--glass-radius-md)] text-[var(--glass-text-secondary)]">
            <Icons.settings />
          </span>
          <h2 className="text-[15px] font-semibold text-[var(--glass-text-primary)]">{t('apiConfig:defaultModels')}</h2>
        </div>
        <p className="mb-2.5 px-1 text-[12px] text-[var(--glass-text-secondary)]">{t('apiConfig:defaultModel.hint')}</p>

        <div className="grid grid-cols-1 gap-2.5 md:grid-cols-2 xl:grid-cols-3">
          {DEFAULT_MODEL_CARDS.map((card) => {
            const options = findEnabledModelsByType(draft.models, card.modelType);
            const currentKey = normalizeModelKey(draft.defaultModels?.[card.field]);
            const currentModel = currentKey ? options.find((option) => option.modelKey === currentKey) ?? null : null;
            const currentCapabilitySelections = currentKey ? draft.capabilityDefaults?.[currentKey] : undefined;
            const capabilityFields = extractCapabilityFieldsFromModel(currentModel?.capabilities, card.modelType);
            const capabilityOverrides =
              currentModel && currentCapabilitySelections
                ? capabilityFields.reduce<Record<string, CapabilityValue>>((accumulator, definition) => {
                    const selectedValue = currentCapabilitySelections[definition.field];
                    if (selectedValue !== undefined) {
                      accumulator[definition.field] = selectedValue;
                    }
                    return accumulator;
                  }, {})
                : {};
            const ModelIcon = Icons[card.icon];
            const errorMessage = state.errors[`defaultModels.${card.field}`];

            return (
              <div key={card.field} className="glass-surface-soft rounded-[var(--glass-radius-lg)] p-2.5">
                <div className="mb-2 flex items-center gap-2">
                  <span className={MONO_ICON_BADGE}>
                    <ModelIcon />
                  </span>
                  <span className="text-[12px] font-semibold text-[var(--glass-text-primary)]">
                    {t(`apiConfig:${card.titleKey}`)}
                  </span>
                </div>

                {card.modelType === 'llm' || card.modelType === 'image' || card.modelType === 'video' ? (
                  <ModelCapabilityDropdown
                    compact
                    models={options.map((option) => ({
                      value: option.modelKey,
                      label: option.name,
                      provider: option.provider,
                      providerName: providerNameById.get(option.provider) ?? option.provider,
                    }))}
                    value={currentKey || undefined}
                    onModelChange={(nextModelKey) => {
                      dispatch({
                        type: 'DEFAULT_MODEL_UPDATE',
                        payload: {
                          field: card.field,
                          value: nextModelKey,
                        },
                      });

                      const nextModel = options.find((option) => option.modelKey === nextModelKey);
                      const nextCapabilityFields = extractCapabilityFieldsFromModel(
                        nextModel?.capabilities,
                        card.modelType,
                      );
                      const existingSelections = draft.capabilityDefaults?.[nextModelKey] ?? {};

                      for (const definition of nextCapabilityFields) {
                        const firstOption = definition.options[0];
                        if (firstOption === undefined || existingSelections[definition.field] !== undefined) {
                          continue;
                        }

                        dispatch({
                          type: 'CAPABILITY_DEFAULT_UPDATE',
                          payload: {
                            modelKey: nextModelKey,
                            field: definition.field,
                            value: firstOption,
                          },
                        });
                      }
                    }}
                    capabilityFields={capabilityFields.map((definition) => ({
                      ...definition,
                      label: toCapabilityFieldLabel(definition.field),
                    }))}
                    capabilityOverrides={capabilityOverrides}
                    onCapabilityChange={(field, rawValue, sample) => {
                      if (!currentKey) {
                        return;
                      }

                      dispatch({
                        type: 'CAPABILITY_DEFAULT_UPDATE',
                        payload: {
                          modelKey: currentKey,
                          field,
                          value: rawValue ? parseBySample(rawValue, sample) : null,
                        },
                      });
                    }}
                    placeholder={t('apiConfig:selectDefault')}
                  />
                ) : (
                  <div className="relative">
                    <select
                      value={currentKey}
                      onChange={(event) =>
                        dispatch({
                          type: 'DEFAULT_MODEL_UPDATE',
                          payload: {
                            field: card.field,
                            value: event.target.value,
                          },
                        })
                      }
                      className="glass-select-base w-full cursor-pointer appearance-none py-1.5 pl-2.5 pr-7 text-[12px]"
                    >
                      <option value="">{t('apiConfig:selectDefault')}</option>
                      {options.map((option, index) => (
                        <option key={`${option.modelKey}-${index}`} value={option.modelKey}>
                          {option.name} ({providerNameById.get(option.provider) ?? option.provider})
                        </option>
                      ))}
                    </select>
                    <div className="pointer-events-none absolute right-2.5 top-2 text-[var(--glass-text-tertiary)]">
                      <Icons.chevronDown />
                    </div>
                  </div>
                )}

                {errorMessage ? <p className="mt-1 text-[11px] text-[var(--glass-tone-danger-fg)]">{errorMessage}</p> : null}
              </div>
            );
          })}
        </div>
      </GlassSurface>

      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
        <AdminProviderPool
          providers={draft.providers}
          errors={state.errors}
          onAddProvider={(baseKey) => dispatch({ type: 'PROVIDER_ADD', payload: { baseKey } })}
          onDeleteProvider={(index) => dispatch({ type: 'PROVIDER_DELETE', payload: { index } })}
          onUpdateProviderField={(index, update) =>
            dispatch({
              type: 'PROVIDER_UPDATE_FIELD',
              payload: {
                index,
                ...update,
              },
            })
          }
        />

        <AdminModelPanel
          models={draft.models}
          providers={draft.providers}
          activeModelType={state.activeModelType}
          errors={state.errors}
          addModelLabel={t('apiConfig:addModel')}
          onAddModel={() => dispatch({ type: 'MODEL_ADD' })}
          onSetActiveModelType={(modelType) => dispatch({ type: 'SET_ACTIVE_MODEL_TYPE', payload: { modelType } })}
          onDeleteModel={(index) => dispatch({ type: 'MODEL_DELETE', payload: { index } })}
          onUpdateModelField={(index, update) =>
            dispatch({
              type: 'MODEL_UPDATE_FIELD',
              payload: {
                index,
                ...update,
              },
            })
          }
        />
      </div>
    </main>
  );
}
