import { ApiClientError } from '../api/client';

export function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === 'AbortError';
}

/**
 * 判断错误是否应展示给用户。
 * 对于 AbortError（请求被取消）等场景静默处理，不弹 alert。
 */
export function shouldShowError(error: unknown): boolean {
  if (isAbortError(error)) {
    return false;
  }
  if (error instanceof ApiClientError && error.status === 0) {
    return false;
  }
  return true;
}
