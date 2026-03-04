import { resolveTaskErrorMessage } from './task/error-message';

export const ERROR_CODES = {
  INSUFFICIENT_BALANCE: 'INSUFFICIENT_BALANCE',
  OPERATION_FAILED: 'INTERNAL_ERROR',
  NETWORK_ERROR: 'NETWORK_ERROR',
} as const;

export type ErrorCode = (typeof ERROR_CODES)[keyof typeof ERROR_CODES];

type ApiErrorPayload = {
  code?: string;
  message?: string;
  error?: string | { code?: string; message?: string };
};

async function readApiErrorPayload(res: Response): Promise<ApiErrorPayload | null> {
  try {
    return (await res.json()) as ApiErrorPayload;
  } catch {
    return null;
  }
}

export async function handleApiError(
  res: Response,
  fallbackCode: ErrorCode = ERROR_CODES.OPERATION_FAILED,
): Promise<never> {
  const payload = await readApiErrorPayload(res);
  const resolvedCode = payload?.code || (typeof payload?.error === 'object' ? payload.error.code : null) || fallbackCode;
  const fallbackMessage = resolvedCode || fallbackCode;
  throw new Error(resolveTaskErrorMessage(payload, fallbackMessage));
}

export async function checkApiResponse(
  res: Response,
  fallbackCode: ErrorCode = ERROR_CODES.OPERATION_FAILED,
): Promise<void> {
  if (res.ok) {
    return;
  }
  await handleApiError(res, fallbackCode);
}

export function isInsufficientBalanceError(error: Error): boolean {
  return error.message === ERROR_CODES.INSUFFICIENT_BALANCE;
}
