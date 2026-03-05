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
        padded ? 'p-4 md:p-6' : '',
        interactive
          ? 'transition-all duration-[250ms] [transition-timing-function:cubic-bezier(0.68,-0.55,0.265,1.55)] hover:-translate-y-0.5 hover:shadow-[var(--glass-shadow-md)]'
          : '',
        className,
      )}
      {...props}
    >
      {children}
    </div>
  );
}
