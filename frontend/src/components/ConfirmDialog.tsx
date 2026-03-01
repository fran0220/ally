import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

import { AppIcon } from './ui/icons';

interface ConfirmDialogProps {
  show: boolean;
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  onConfirm: () => void;
  onCancel: () => void;
  type?: 'danger' | 'warning' | 'info';
}

type TypeStyle = {
  icon: ReactNode;
  confirmBg: string;
  iconBg: string;
};

export function ConfirmDialog({
  show,
  title,
  message,
  confirmText,
  cancelText,
  onConfirm,
  onCancel,
  type = 'danger',
}: ConfirmDialogProps) {
  const { t } = useTranslation('common');
  if (!show) {
    return null;
  }

  const finalConfirmText = confirmText ?? t('confirm');
  const finalCancelText = cancelText ?? t('cancel');

  const typeStyles: Record<'danger' | 'warning' | 'info', TypeStyle> = {
    danger: {
      icon: <AppIcon name="alert" className="h-6 w-6 text-[var(--glass-tone-danger-fg)]" />,
      confirmBg: 'glass-btn-tone-danger',
      iconBg: 'bg-[var(--glass-tone-danger-bg)]',
    },
    warning: {
      icon: <AppIcon name="alert" className="h-6 w-6 text-[var(--glass-tone-warning-fg)]" />,
      confirmBg: 'glass-btn-tone-warning',
      iconBg: 'bg-[var(--glass-tone-warning-bg)]',
    },
    info: {
      icon: <AppIcon name="info" className="h-6 w-6 text-[var(--glass-tone-info-fg)]" />,
      confirmBg: 'glass-btn-tone-info',
      iconBg: 'bg-[var(--glass-tone-info-bg)]',
    },
  };

  const currentStyle = typeStyles[type];

  return (
    <>
      <div className="animate-fade-in fixed inset-0 z-50 glass-overlay" onClick={onCancel} />

      <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
        <div
          className="animate-scale-in glass-surface-modal w-full max-w-md p-6"
          onClick={(event) => event.stopPropagation()}
        >
          <div className={`mb-4 flex h-12 w-12 items-center justify-center rounded-full ${currentStyle.iconBg}`}>
            {currentStyle.icon}
          </div>

          <h3 className="mb-2 text-xl font-semibold text-[var(--glass-text-primary)]">{title}</h3>
          <p className="mb-6 text-[var(--glass-text-secondary)]">{message}</p>

          <div className="flex gap-3">
            <button
              type="button"
              onClick={onCancel}
              className="glass-btn-base glass-btn-secondary flex-1 rounded-xl px-4 py-2.5 font-medium"
            >
              {finalCancelText}
            </button>
            <button
              type="button"
              onClick={onConfirm}
              className={`glass-btn-base flex-1 rounded-xl px-4 py-2.5 font-medium ${currentStyle.confirmBg}`}
            >
              {finalConfirmText}
            </button>
          </div>
        </div>
      </div>
    </>
  );
}
