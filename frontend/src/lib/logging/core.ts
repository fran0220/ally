type LogLevel = 'DEBUG' | 'INFO' | 'WARN' | 'ERROR';

type SemanticContext = {
  module?: string;
  action?: string;
  requestId?: string;
  taskId?: string;
  projectId?: string;
  userId?: string;
  provider?: string;
  errorCode?: string;
  retryable?: boolean;
  durationMs?: number;
};

type ScopedLogFn = (...args: unknown[]) => void;

type ScopedLogEvent = {
  level: LogLevel;
  message: string;
  details?: Record<string, unknown> | unknown[] | null;
};

export type ScopedLogger = {
  debug: ScopedLogFn;
  info: ScopedLogFn;
  warn: ScopedLogFn;
  error: ScopedLogFn;
  event: (event: ScopedLogEvent) => void;
  child: (context: Partial<SemanticContext>) => ScopedLogger;
};

function logWithLevel(level: LogLevel, context: Partial<SemanticContext>, args: unknown[]) {
  const prefix = context.module ? `[${context.module}]` : '[app]';
  const output = [prefix, ...args];

  if (level === 'ERROR') {
    console.error(...output);
    return;
  }

  if (level === 'WARN') {
    console.warn(...output);
    return;
  }

  if (level === 'DEBUG') {
    console.debug(...output);
    return;
  }

  console.info(...output);
}

export function logDebug(...args: unknown[]) {
  logWithLevel('DEBUG', {}, args);
}

export function logInfo(...args: unknown[]) {
  logWithLevel('INFO', {}, args);
}

export function logWarn(...args: unknown[]) {
  logWithLevel('WARN', {}, args);
}

export function logError(...args: unknown[]) {
  logWithLevel('ERROR', {}, args);
}

export function createScopedLogger(baseContext: Partial<SemanticContext>): ScopedLogger {
  const withLevel = (level: LogLevel): ScopedLogFn => (...args: unknown[]) => {
    logWithLevel(level, baseContext, args);
  };

  return {
    debug: withLevel('DEBUG'),
    info: withLevel('INFO'),
    warn: withLevel('WARN'),
    error: withLevel('ERROR'),
    event: (event: ScopedLogEvent) => {
      logWithLevel(event.level, baseContext, [event.message, event.details ?? null]);
    },
    child: (context: Partial<SemanticContext>) => createScopedLogger({ ...baseContext, ...context }),
  };
}
