import type { AdminModel, AdminProvider, ModelType } from '../../../api/admin';
import { GlassButton, GlassSurface } from '../../../components/ui/primitives';

import { type ModelFieldUpdate, MODEL_TYPES } from './AdminAiConfigReducer';
import { AdminModelCard } from './AdminModelCard';

interface AdminModelPanelProps {
  models: AdminModel[];
  providers: AdminProvider[];
  activeModelType: ModelType;
  errors: Record<string, string>;
  onAddModel: () => void;
  onSetActiveModelType: (modelType: ModelType) => void;
  onDeleteModel: (index: number) => void;
  onUpdateModelField: (index: number, update: ModelFieldUpdate) => void;
  addModelLabel: string;
}

export function AdminModelPanel({
  models,
  providers,
  activeModelType,
  errors,
  onAddModel,
  onSetActiveModelType,
  onDeleteModel,
  onUpdateModelField,
  addModelLabel,
}: AdminModelPanelProps) {
  const filteredModels = models.flatMap((model, index) =>
    model.type === activeModelType ? [{ model, index }] : [],
  );

  return (
    <GlassSurface>
      <div className="mb-3 flex items-center justify-between gap-3">
        <h2 className="text-sm font-semibold text-[var(--glass-text-secondary)]">Models</h2>
        <GlassButton size="sm" variant="soft" onClick={onAddModel}>
          + {addModelLabel}
        </GlassButton>
      </div>

      <div className="mb-3 flex flex-wrap gap-2">
        {MODEL_TYPES.map((type) => (
          <GlassButton
            key={type}
            size="sm"
            variant={activeModelType === type ? 'primary' : 'soft'}
            onClick={() => onSetActiveModelType(type)}
          >
            {type}
          </GlassButton>
        ))}
      </div>

      <div className="space-y-2">
        {filteredModels.map(({ model, index }) => (
          <AdminModelCard
            key={`${model.modelKey}-${index}`}
            model={model}
            index={index}
            providers={providers}
            errors={errors}
            onDelete={onDeleteModel}
            onUpdateField={onUpdateModelField}
          />
        ))}
        {filteredModels.length === 0 ? (
          <p className="text-sm text-[var(--glass-text-tertiary)]">No models in this category.</p>
        ) : null}
      </div>
    </GlassSurface>
  );
}
