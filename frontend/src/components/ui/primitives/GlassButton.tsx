import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from 'react';

import { cx } from './cx';

export interface GlassButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger' | 'soft';
  size?: 'sm' | 'md' | 'lg';
  loading?: boolean;
  iconLeft?: ReactNode;
  iconRight?: ReactNode;
}

export const GlassButton = forwardRef<HTMLButtonElement, GlassButtonProps>(function GlassButton(
  {
    variant = 'secondary',
    size = 'md',
    loading = false,
    iconLeft,
    iconRight,
    className,
    children,
    disabled,
    ...props
  },
  ref,
) {
  const variantClass =
    variant === 'primary'
      ? 'glass-btn-primary'
      : variant === 'ghost'
        ? 'glass-btn-ghost'
        : variant === 'danger'
          ? 'glass-btn-danger'
          : variant === 'soft'
            ? 'glass-btn-soft'
            : 'glass-btn-secondary';

  const sizeClass =
    size === 'sm' ? 'h-7 px-2.5 text-xs' : size === 'lg' ? 'h-10 px-5 text-sm' : 'h-8 px-3 text-sm';

  return (
    <button
      ref={ref}
      className={cx('glass-btn-base', variantClass, sizeClass, className)}
      disabled={disabled || loading}
      {...props}
    >
      {loading ? <span className="inline-block h-3.5 w-3.5 animate-spin rounded-full border-2 border-current border-r-transparent" /> : iconLeft}
      {children}
      {!loading ? iconRight : null}
    </button>
  );
});
