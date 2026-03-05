import { useEffect, useMemo, useReducer } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';

import { getAdminAiConfig, updateAdminAiConfig } from '../../api/admin';
import { GlassButton, GlassSurface } from '../../components/ui/primitives';
import { queryKeys } from '../../lib/query-keys';
import { AdminModelPanel } from './ai-config/AdminModelPanel';
import {
  adminAiConfigInitialState,
  adminAiConfigReducer,
  isAdminAiConfigDirty,
} from './ai-config/AdminAiConfigReducer';
import { AdminProviderPool } from './ai-config/AdminProviderPool';

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

  if (!state.draft) {
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
                if (!state.draft || !canSave) {
                  return;
                }
                void saveMutation.mutateAsync(state.draft);
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

      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
        <AdminProviderPool
          providers={state.draft.providers}
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
          models={state.draft.models}
          providers={state.draft.providers}
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
