import { resolveTaskPresentationState } from '../lib/task/presentation';
import { TaskStatusInline } from './task/TaskStatusInline';

interface ProgressToastProps {
  show: boolean;
  message: string;
  step?: string;
}

export function ProgressToast({ show, message, step }: ProgressToastProps) {
  if (!show) {
    return null;
  }

  const runningState = resolveTaskPresentationState({
    phase: 'processing',
    intent: 'generate',
    resource: 'text',
    hasOutput: true,
  });

  return (
    <div className="animate-slide-up fixed bottom-8 right-8 z-50">
      <div className="glass-surface-modal min-w-[320px] p-4">
        <div className="flex items-start space-x-3">
          <div className="mt-0.5 shrink-0">
            <TaskStatusInline state={runningState} className="[&>span]:sr-only" />
          </div>

          <div className="flex-1">
            <div className="mb-1 font-semibold text-[var(--glass-text-primary)]">{message}</div>
            {step ? <div className="text-sm text-[var(--glass-text-secondary)]">{step}</div> : null}
          </div>
        </div>

        <div className="mt-3 h-1.5 w-full overflow-hidden rounded-full bg-[var(--glass-bg-muted)]">
          <div className="animate-progress h-1.5 rounded-full bg-[var(--glass-tone-info-fg)]" />
        </div>
      </div>
    </div>
  );
}
