import type { AdminProvider } from '../../../api/admin';
import { GlassButton, GlassSurface } from '../../../components/ui/primitives';

import { type ProviderBaseKey, type ProviderFieldUpdate, PROVIDER_BASE_KEYS } from './AdminAiConfigReducer';
import { AdminProviderCard } from './AdminProviderCard';

interface AdminProviderPoolProps {
  providers: AdminProvider[];
  errors: Record<string, string>;
  onAddProvider: (baseKey: ProviderBaseKey) => void;
  onDeleteProvider: (index: number) => void;
  onUpdateProviderField: (index: number, update: ProviderFieldUpdate) => void;
}

export function AdminProviderPool({
  providers,
  errors,
  onAddProvider,
  onDeleteProvider,
  onUpdateProviderField,
}: AdminProviderPoolProps) {
  return (
    <GlassSurface>
      <div className="mb-3 flex items-center justify-between gap-3">
        <h2 className="text-sm font-semibold text-[var(--glass-text-secondary)]">Provider Pool</h2>
        <div className="flex flex-wrap gap-1">
          {PROVIDER_BASE_KEYS.map((baseKey) => (
            <GlassButton key={baseKey} size="sm" variant="soft" onClick={() => onAddProvider(baseKey)}>
              + {baseKey}
            </GlassButton>
          ))}
        </div>
      </div>

      <div className="space-y-3">
        {providers.map((provider, index) => (
          <AdminProviderCard
            key={provider.id}
            provider={provider}
            index={index}
            errors={errors}
            onDelete={onDeleteProvider}
            onUpdateField={onUpdateProviderField}
          />
        ))}
        {providers.length === 0 ? (
          <p className="text-sm text-[var(--glass-text-tertiary)]">No providers configured yet.</p>
        ) : null}
      </div>
    </GlassSurface>
  );
}
