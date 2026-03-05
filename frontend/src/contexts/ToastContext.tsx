import { createContext, useCallback, useContext, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

import { AppIcon } from '@/components/ui/icons';

export interface Toast {
  id: string;
  message: string;
  type: 'success' | 'error' | 'warning' | 'info';
  duration: number;
}

interface ToastContextValue {
  toasts: Toast[];
  showToast: (message: string, type?: Toast['type'], duration?: number) => void;
  showError: (code: string, details?: Record<string, unknown>) => void;
  dismissToast: (id: string) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const { t } = useTranslation('errors');

  const showToast = useCallback((message: string, type: Toast['type'] = 'info', duration = 5000) => {
    const id = Math.random().toString(36).slice(2, 9);
    setToasts((previous) => [...previous, { id, message, type, duration }]);

    if (duration > 0) {
      window.setTimeout(() => {
        setToasts((previous) => previous.filter((toast) => toast.id !== id));
      }, duration);
    }
  }, []);

  const showError = useCallback(
    (code: string, details?: Record<string, unknown>) => {
      const translationValues = Object.fromEntries(
        Object.entries(details || {}).map(([key, value]) => {
          if (typeof value === 'string' || typeof value === 'number' || value instanceof Date) {
            return [key, value];
          }
          return [key, String(value)];
        }),
      );
      const translated = t(code, translationValues);
      showToast(typeof translated === 'string' ? translated : code, 'error', 8000);
    },
    [showToast, t],
  );

  const dismissToast = useCallback((id: string) => {
    setToasts((previous) => previous.filter((toast) => toast.id !== id));
  }, []);

  return (
    <ToastContext.Provider value={{ toasts, showToast, showError, dismissToast }}>
      {children}
      <ToastContainer toasts={toasts} onDismiss={dismissToast} />
    </ToastContext.Provider>
  );
}

export function useToast(): ToastContextValue {
  const context = useContext(ToastContext);
  if (!context) {
    throw new Error('useToast must be used within ToastProvider');
  }
  return context;
}

function ToastContainer({ toasts, onDismiss }: { toasts: Toast[]; onDismiss: (id: string) => void }) {
  if (toasts.length === 0) {
    return null;
  }

  return (
    <div className="fixed top-4 right-4 z-[9999] flex flex-col gap-2 pointer-events-none">
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className={`
            pointer-events-auto
            flex items-center gap-3
            px-4 py-3
            rounded-[var(--glass-radius-lg)]
            animate-in slide-in-from-right-full duration-300
            max-w-md
            border
            ${getToastStyle(toast.type)}
          `}
        >
          <span className="w-5 h-5 flex items-center justify-center">{getToastIcon(toast.type)}</span>
          <span className="text-sm font-medium flex-1">{toast.message}</span>
          <button
            type="button"
            onClick={() => onDismiss(toast.id)}
            className="glass-btn-base glass-btn-ghost w-6 h-6 rounded-[var(--glass-radius-sm)] p-0 opacity-70 hover:opacity-100 transition-opacity"
          >
            <AppIcon name="close" className="w-4 h-4" />
          </button>
        </div>
      ))}
    </div>
  );
}

function getToastStyle(type: Toast['type']): string {
  switch (type) {
    case 'success':
      return 'bg-[var(--glass-tone-success-bg)] text-[var(--glass-tone-success-fg)] border-[color:color-mix(in_srgb,var(--glass-tone-success-fg)_22%,transparent)]';
    case 'error':
      return 'bg-[var(--glass-tone-danger-bg)] text-[var(--glass-tone-danger-fg)] border-[color:color-mix(in_srgb,var(--glass-tone-danger-fg)_22%,transparent)]';
    case 'warning':
      return 'bg-[var(--glass-tone-warning-bg)] text-[var(--glass-tone-warning-fg)] border-[color:color-mix(in_srgb,var(--glass-tone-warning-fg)_22%,transparent)]';
    case 'info':
    default:
      return 'bg-[var(--glass-tone-info-bg)] text-[var(--glass-tone-info-fg)] border-[color:color-mix(in_srgb,var(--glass-tone-info-fg)_22%,transparent)]';
  }
}

function getToastIcon(type: Toast['type']) {
  switch (type) {
    case 'success':
      return <AppIcon name="check" className="w-4 h-4" />;
    case 'error':
      return <AppIcon name="close" className="w-4 h-4" />;
    case 'warning':
      return <AppIcon name="alert" className="w-4 h-4" />;
    case 'info':
    default:
      return <AppIcon name="info" className="w-4 h-4" />;
  }
}
