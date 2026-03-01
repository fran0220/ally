import { forwardRef, type TextareaHTMLAttributes } from 'react';

import { cx } from './cx';

export interface GlassTextareaProps extends TextareaHTMLAttributes<HTMLTextAreaElement> {
  density?: 'compact' | 'default';
}

export const GlassTextarea = forwardRef<HTMLTextAreaElement, GlassTextareaProps>(function GlassTextarea(
  { density = 'default', className, ...props },
  ref,
) {
  return (
    <textarea
      ref={ref}
      className={cx(
        'glass-textarea-base resize-none',
        density === 'compact' ? 'px-3 py-2 text-sm leading-6' : 'px-3 py-2.5 text-sm leading-6',
        className,
      )}
      {...props}
    />
  );
});
