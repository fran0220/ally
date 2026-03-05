import type { HTMLAttributes, ReactNode } from 'react';

import { cx } from './cx';

export interface GlassSurfaceProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
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
  ...props
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
        padded ? 'p-3 md:p-4' : '',
        interactive
          ? 'hover-lift press-feedback cursor-pointer'
          : '',
        className,
      )}
      {...props}
    >
      {children}
    </div>
  );
}
