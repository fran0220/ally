import type { ReactNode } from 'react';

export function AnimatedBackground() {
  return (
    <div className="fixed inset-0 -z-10 overflow-hidden bg-[var(--glass-bg-canvas)]">
      <div className="absolute top-[-50%] left-[-50%] h-[200%] w-[200%] animate-aurora opacity-40 blur-[100px]">
        <div className="absolute left-0 top-0 h-1/2 w-1/2 animate-blob rounded-full bg-[var(--glass-bg-surface)] mix-blend-multiply" />
        <div className="animation-delay-2000 absolute right-0 top-0 h-1/2 w-1/2 animate-blob rounded-full bg-[var(--glass-bg-muted)] mix-blend-multiply" />
        <div className="animation-delay-4000 absolute bottom-0 left-0 h-1/2 w-1/2 animate-blob rounded-full bg-[var(--glass-bg-surface-strong)] mix-blend-multiply" />
      </div>
      <div className="absolute inset-0 bg-white/60 backdrop-blur-3xl" />
    </div>
  );
}

export function GlassPanel({ children, className = '' }: { children: ReactNode; className?: string }) {
  return <div className={`glass-surface-elevated ${className}`}>{children}</div>;
}

export function Button({
  children,
  primary = false,
  onClick,
  disabled = false,
  icon,
  className = '',
}: {
  children: ReactNode;
  primary?: boolean;
  onClick?: () => void;
  disabled?: boolean;
  icon?: ReactNode;
  className?: string;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={`glass-btn-base px-6 py-2.5 ${primary ? 'glass-btn-primary text-white' : 'glass-btn-secondary'} disabled:cursor-not-allowed disabled:opacity-50 ${className}`}
    >
      {icon && <span>{icon}</span>}
      {children}
    </button>
  );
}
