import { useTranslation } from 'react-i18next';

import { AppIcon } from '../ui/icons';
import type { TaskPresentationState } from '../../lib/task/presentation';

interface TaskStatusOverlayProps {
  state: TaskPresentationState | null;
  className?: string;
}

export function TaskStatusOverlay({ state, className }: TaskStatusOverlayProps) {
  const { t } = useTranslation('common');
  if (!state) {
    return null;
  }
  if (state.mode !== 'overlay' && state.mode !== 'placeholder') {
    return null;
  }

  const label = state.labelKey ? t(state.labelKey) : t('loading');
  const wrapperClass = [
    'absolute inset-0 flex flex-col items-center justify-center bg-[var(--glass-overlay)]',
    className ?? '',
  ]
    .join(' ')
    .trim();

  return (
    <div className={wrapperClass}>
      {state.isError ? (
        <AppIcon name="alertSolid" className="h-7 w-7 text-[var(--glass-tone-danger-fg)]" />
      ) : (
        <AppIcon name="loader" className="h-7 w-7 animate-spin text-white" />
      )}
      <span className="mt-2 text-xs text-white">{label}</span>
    </div>
  );
}
