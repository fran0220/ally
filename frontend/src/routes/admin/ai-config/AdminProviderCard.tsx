import type { AdminProvider } from '../../../api/admin';
import { GlassButton, GlassField, GlassInput } from '../../../components/ui/primitives';

import {
  type ProviderFieldUpdate,
  PROVIDER_PRESETS,
  isProviderBaseKey,
  providerBaseKey,
} from './AdminAiConfigReducer';

interface AdminProviderCardProps {
  provider: AdminProvider;
  index: number;
  errors: Record<string, string>;
  onDelete: (index: number) => void;
  onUpdateField: (index: number, update: ProviderFieldUpdate) => void;
}

export function AdminProviderCard({ provider, index, errors, onDelete, onUpdateField }: AdminProviderCardProps) {
  const baseKey = providerBaseKey(provider.id);
  const preset = isProviderBaseKey(baseKey) ? PROVIDER_PRESETS[baseKey] : null;
  const idError = errors[`providers[${index}].id`];
  const nameError = errors[`providers[${index}].name`];
  const baseUrlError = errors[`providers[${index}].baseUrl`];

  return (
    <article className="glass-list-row flex-col items-stretch gap-3 p-3">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm font-semibold text-[var(--glass-text-primary)]">{preset?.label ?? baseKey}</p>
          <p className="text-xs text-[var(--glass-text-tertiary)]">{provider.id}</p>
          {idError ? <p className="mt-1 text-xs text-[var(--glass-tone-danger-fg)]">{idError}</p> : null}
        </div>
        <GlassButton size="sm" variant="ghost" onClick={() => onDelete(index)}>
          x
        </GlassButton>
      </div>

      <GlassField label="Name" error={nameError}>
        <GlassInput
          value={provider.name}
          onChange={(event) => onUpdateField(index, { field: 'name', value: event.target.value })}
        />
      </GlassField>

      {preset?.needsBaseUrl ? (
        <GlassField label="Base URL" error={baseUrlError}>
          <GlassInput
            value={provider.baseUrl ?? ''}
            onChange={(event) => onUpdateField(index, { field: 'baseUrl', value: event.target.value })}
          />
        </GlassField>
      ) : null}

      <GlassField label="API Key">
        <GlassInput
          type="password"
          value={provider.apiKey ?? ''}
          onChange={(event) => onUpdateField(index, { field: 'apiKey', value: event.target.value })}
        />
      </GlassField>
    </article>
  );
}
