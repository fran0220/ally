import type { ReactNode } from 'react';

import { cx } from './cx';

export interface GlassSurfaceProps {
  children: ReactNode;
  className?: string;
  variant?: 'panel' | 'card' | 'elevated' | 'modal';
  density?: 'compact' | 'default';
  interactive?: boolean;
  padded?: boolean;
}

export function GlassSurface({
  children,
  className,
  variant = 'panel',
  density = 'default',
  interactive = false,
  padded = true,
}: GlassSurfaceProps) {
  const variantClass =
    variant === 'elevated'
      ? 'glass-surface-elevated'
      : variant === 'modal'
        ? 'glass-surface-modal'
        : 'glass-surface';

  return (
    <div
      className={cx(
        variantClass,
        density === 'compact' ? 'glass-density-compact' : 'glass-density-default',
        padded ? 'p-4 md:p-6' : '',
        interactive ? 'transition-all duration-200 hover:-translate-y-0.5 hover:shadow-[var(--glass-shadow-md)]' : '',
        className,
      )}
    >
      {children}
    </div>
  );
}
