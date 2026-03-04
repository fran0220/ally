export type TaskErrorSummary = {
  code: string | null;
  message: string;
  cancelled: boolean;
};

function asObject(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return null;
  }
  return value as Record<string, unknown>;
}

function asString(value: unknown): string | null {
  if (typeof value !== 'string') {
    return null;
  }
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function asBoolean(value: unknown): boolean {
  return value === true;
}

function looksCancelledMessage(value: string | null): boolean {
  if (!value) {
    return false;
  }
  const lower = value.toLowerCase();
  return (
    lower.includes('task cancelled') ||
    lower.includes('task canceled') ||
    lower.includes('cancelled by user') ||
    lower.includes('canceled by user') ||
    lower.includes('任务已取消')
  );
}

export function resolveTaskErrorSummary(payload: unknown, fallbackMessage = 'Task failed'): TaskErrorSummary {
  const source = asObject(payload) || {};
  const sourceError = asObject(source.error) || {};
  const sourceErrorDetails = asObject(sourceError.details) || {};
  const sourceDetails = asObject(source.details) || {};

  const code =
    asString(sourceError.code) ||
    asString(source.errorCode) ||
    asString(source.code) ||
    null;

  const message =
    asString(sourceError.message) ||
    asString(sourceErrorDetails.message) ||
    asString(source.errorMessage) ||
    asString(source.message) ||
    asString(sourceDetails.message) ||
    asString(sourceError.details) ||
    null;

  const cancelled =
    asBoolean(source.cancelled) ||
    asBoolean(source.canceled) ||
    asBoolean(sourceError.cancelled) ||
    asBoolean(sourceError.canceled) ||
    asBoolean(sourceErrorDetails.cancelled) ||
    asBoolean(sourceErrorDetails.canceled) ||
    code === 'TASK_CANCELLED' ||
    looksCancelledMessage(message);

  if (cancelled) {
    return {
      code: code || 'TASK_CANCELLED',
      message: 'Task cancelled by user',
      cancelled: true,
    };
  }

  return {
    code,
    message: message || fallbackMessage,
    cancelled: false,
  };
}

export function resolveTaskErrorMessage(payload: unknown, fallbackMessage = 'Task failed'): string {
  return resolveTaskErrorSummary(payload, fallbackMessage).message;
}
