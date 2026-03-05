import type { AdminModel, AdminProvider } from '../../../api/admin';
import { GlassButton, GlassChip, GlassField, GlassInput } from '../../../components/ui/primitives';

import type { ModelFieldUpdate } from './AdminAiConfigReducer';

interface AdminModelCardProps {
  model: AdminModel;
  index: number;
  providers: AdminProvider[];
  errors: Record<string, string>;
  onDelete: (index: number) => void;
  onUpdateField: (index: number, update: ModelFieldUpdate) => void;
}

export function AdminModelCard({ model, index, providers, errors, onDelete, onUpdateField }: AdminModelCardProps) {
  const modelIdError = errors[`models[${index}].modelId`];
  const modelKeyError = errors[`models[${index}].modelKey`];
  const providerError = errors[`models[${index}].provider`];
  const priceError = errors[`models[${index}].price`];
  const hasSelectedProvider = providers.some((provider) => provider.id === model.provider);

  return (
    <article className="glass-list-row flex-col items-stretch gap-3 p-3">
      <div className="flex items-center justify-between">
        <GlassChip tone={model.enabled ? 'success' : 'neutral'}>{model.type}</GlassChip>
        <GlassButton size="sm" variant="ghost" onClick={() => onDelete(index)}>
          x
        </GlassButton>
      </div>

      <GlassField label="Display Name">
        <GlassInput
          value={model.name}
          onChange={(event) => onUpdateField(index, { field: 'name', value: event.target.value })}
        />
      </GlassField>

      <div className="grid gap-2 md:grid-cols-2">
        <GlassField label="Model ID" error={modelIdError ?? modelKeyError} hint={`Key: ${model.modelKey}`}>
          <GlassInput
            value={model.modelId}
            onChange={(event) => onUpdateField(index, { field: 'modelId', value: event.target.value })}
          />
        </GlassField>

        <GlassField label="Provider" error={providerError}>
          <select
            className="glass-select-base h-10 px-3 text-sm"
            value={model.provider}
            onChange={(event) => onUpdateField(index, { field: 'provider', value: event.target.value })}
          >
            {!hasSelectedProvider ? <option value={model.provider}>{model.provider || 'Unknown provider'}</option> : null}
            {providers.map((provider) => (
              <option key={provider.id} value={provider.id}>
                {provider.name}
              </option>
            ))}
          </select>
        </GlassField>

        <GlassField label="Price" error={priceError}>
          <GlassInput
            type="number"
            min={0}
            step={0.01}
            value={String(model.price)}
            onChange={(event) => onUpdateField(index, { field: 'price', value: Number(event.target.value) })}
          />
        </GlassField>

        <label className="flex items-center gap-2 self-end text-sm text-[var(--glass-text-secondary)]">
          <input
            type="checkbox"
            checked={model.enabled}
            onChange={(event) => onUpdateField(index, { field: 'enabled', value: event.target.checked })}
          />
          Enabled
        </label>
      </div>
    </article>
  );
}
