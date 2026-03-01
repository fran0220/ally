import { forwardRef, type InputHTMLAttributes } from 'react';

import { cx } from './cx';

export interface GlassInputProps extends InputHTMLAttributes<HTMLInputElement> {
  density?: 'compact' | 'default';
}

export const GlassInput = forwardRef<HTMLInputElement, GlassInputProps>(function GlassInput(
  { density = 'default', className, ...props },
  ref,
) {
  return (
    <input
      ref={ref}
      className={cx(
        'glass-input-base',
        density === 'compact' ? 'h-9 px-3 text-sm leading-5' : 'h-10 px-3 text-sm leading-5',
        className,
      )}
      {...props}
    />
  );
});
