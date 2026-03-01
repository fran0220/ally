import { ApiClientError } from '../api/client';

/**
 * 判断错误是否应展示给用户。
 * 对于 AbortError（请求被取消）等场景静默处理，不弹 alert。
 */
export function shouldShowError(error: unknown): boolean {
  if (error instanceof DOMException && error.name === 'AbortError') {
    return false;
  }
  if (error instanceof ApiClientError && error.status === 0) {
    return false;
  }
  return true;
}
