import { useTranslation } from 'react-i18next';

import { AppIcon } from '../ui/icons';
import type { TaskPresentationState } from '../../lib/task/presentation';

interface TaskStatusInlineProps {
  state: TaskPresentationState | null;
  className?: string;
}

export function TaskStatusInline({ state, className }: TaskStatusInlineProps) {
  const { t } = useTranslation('common');
  if (!state) {
    return null;
  }
  if (!state.isRunning && !state.isError) {
    return null;
  }

  const label = state.labelKey ? t(state.labelKey) : t('loading');
  const wrapperClass = ['inline-flex items-center gap-1 text-xs', className ?? ''].join(' ').trim();

  return (
    <div className={wrapperClass}>
      {state.isError ? (
        <span className="text-[var(--glass-tone-danger-fg)]">{label}</span>
      ) : (
        <>
          <AppIcon name="loader" className="h-3.5 w-3.5 animate-spin text-[var(--glass-tone-info-fg)]" />
          <span className="text-[var(--glass-text-secondary)]">{label}</span>
        </>
      )}
    </div>
  );
}

export default TaskStatusInline;
