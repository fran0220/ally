import { useQuery } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';

import { getPreference, listUserModels } from '../../api/user';
import { GlassChip, GlassSurface } from '../../components/ui/primitives';
import { queryKeys } from '../../lib/query-keys';

export function Profile() {
  const { t } = useTranslation('profile');
  const preferenceQuery = useQuery({
    queryKey: queryKeys.user.preference(),
    queryFn: getPreference,
  });

  const modelsQuery = useQuery({
    queryKey: queryKeys.user.models(),
    queryFn: listUserModels,
  });

  const preference = preferenceQuery.data?.preference;

  return (
    <main className="page-shell py-10">
      <header className="mb-6">
        <h1 className="glass-page-title">{t('personalAccount')}</h1>
        <p className="glass-page-subtitle">{t('apiConfig')}</p>
      </header>

      {preferenceQuery.error instanceof Error ? (
        <p className="mb-4 text-sm text-[var(--glass-tone-danger-fg)]">{preferenceQuery.error.message}</p>
      ) : null}

      <div className="grid gap-4 lg:grid-cols-2">
        <GlassSurface>
          <h2 className="text-sm font-semibold text-[var(--glass-text-secondary)]">Preference</h2>
          {preferenceQuery.isLoading ? <p className="mt-3 text-sm text-[var(--glass-text-tertiary)]">Loading...</p> : null}
          {preference ? (
            <dl className="mt-3 grid grid-cols-2 gap-3 text-sm">
              <div>
                <dt className="text-[var(--glass-text-tertiary)]">Video Ratio</dt>
                <dd>{preference.videoRatio}</dd>
              </div>
              <div>
                <dt className="text-[var(--glass-text-tertiary)]">Art Style</dt>
                <dd>{preference.artStyle}</dd>
              </div>
              <div>
                <dt className="text-[var(--glass-text-tertiary)]">TTS Rate</dt>
                <dd>{preference.ttsRate}</dd>
              </div>
              <div>
                <dt className="text-[var(--glass-text-tertiary)]">Updated</dt>
                <dd>{new Date(preference.updatedAt).toLocaleString()}</dd>
              </div>
            </dl>
          ) : null}
        </GlassSurface>

        <GlassSurface>
          <h2 className="text-sm font-semibold text-[var(--glass-text-secondary)]">Available Models</h2>
          {modelsQuery.isLoading ? <p className="mt-3 text-sm text-[var(--glass-text-tertiary)]">Loading...</p> : null}
          {modelsQuery.error instanceof Error ? (
            <p className="mt-3 text-sm text-[var(--glass-tone-danger-fg)]">{modelsQuery.error.message}</p>
          ) : null}
          {modelsQuery.data ? (
            <div className="mt-3 space-y-3">
              {(['llm', 'image', 'video', 'audio', 'lipsync'] as const).map((type) => (
                <div key={type}>
                  <p className="mb-2 text-xs uppercase tracking-wide text-[var(--glass-text-tertiary)]">{type}</p>
                  <div className="flex flex-wrap gap-2">
                    {modelsQuery.data[type].slice(0, 6).map((model) => (
                      <GlassChip key={model.value} tone="info">
                        {model.label}
                      </GlassChip>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          ) : null}
        </GlassSurface>
      </div>
    </main>
  );
}
