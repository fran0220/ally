import type { ReactNode } from 'react';

import { cx } from './cx';

export type UiTone = 'neutral' | 'info' | 'success' | 'warning' | 'danger';

export interface GlassChipProps {
  tone?: UiTone;
  icon?: ReactNode;
  onRemove?: () => void;
  children: ReactNode;
  className?: string;
}

export function GlassChip({ tone = 'neutral', icon, onRemove, children, className }: GlassChipProps) {
  const toneClass =
    tone === 'info'
      ? 'glass-chip-info'
      : tone === 'success'
        ? 'glass-chip-success'
        : tone === 'warning'
          ? 'glass-chip-warning'
          : tone === 'danger'
            ? 'glass-chip-danger'
            : 'glass-chip-neutral';

  return (
    <span className={cx('glass-chip', toneClass, className)}>
      {icon}
      <span>{children}</span>
      {onRemove ? (
        <button
          type="button"
          onClick={onRemove}
          className="rounded-full p-0.5 transition-colors hover:bg-black/10"
          aria-label="remove"
        >
          <span className="block h-3 w-3 leading-[12px]">x</span>
        </button>
      ) : null}
    </span>
  );
}
