import i18n from '../../i18n'
import type { UnifiedErrorCode } from './codes'

export const USER_ERROR_MESSAGES_ZH: Record<UnifiedErrorCode, string> = {
  UNAUTHORIZED: '请先登录后再试。',
  FORBIDDEN: '你没有权限执行此操作。',
  NOT_FOUND: '没有找到对应的数据。',
  INVALID_PARAMS: '请求参数不正确，请检查后重试。',
  MISSING_CONFIG: '系统配置不完整，请联系管理员。',
  CONFLICT: '当前状态冲突，请刷新后重试。',
  TASK_NOT_READY: '任务还在处理中，请稍后。',
  NO_RESULT: '任务已完成，但没有可用结果。',
  RATE_LIMIT: '请求过于频繁，请稍后重试。',
  QUOTA_EXCEEDED: '额度已用尽，请稍后再试。',
  EXTERNAL_ERROR: '外部服务暂时不可用，请稍后重试。',
  NETWORK_ERROR: '网络异常，请稍后重试。',
  INSUFFICIENT_BALANCE: '余额不足，请先充值。',
  SENSITIVE_CONTENT: '内容可能涉及敏感信息，请修改后重试。',
  GENERATION_TIMEOUT: '生成超时，请重试。',
  GENERATION_FAILED: '生成失败，请稍后重试。',
  WATCHDOG_TIMEOUT: '任务执行超时，系统已终止该任务。',
  WORKER_EXECUTION_ERROR: '任务执行失败，请稍后重试。',
  INTERNAL_ERROR: '系统内部错误，请稍后重试。',
}

export const USER_ERROR_MESSAGES_EN: Record<UnifiedErrorCode, string> = {
  UNAUTHORIZED: 'Please log in first.',
  FORBIDDEN: "You don't have permission for this action.",
  NOT_FOUND: 'The requested data was not found.',
  INVALID_PARAMS: 'Invalid parameters, please check and try again.',
  MISSING_CONFIG: 'System configuration is incomplete, please contact admin.',
  CONFLICT: 'State conflict, please refresh and try again.',
  TASK_NOT_READY: 'Task is still processing, please wait.',
  NO_RESULT: 'Task completed but no results available.',
  RATE_LIMIT: 'Too many requests, please try again later.',
  QUOTA_EXCEEDED: 'Quota exhausted, please try again later.',
  EXTERNAL_ERROR: 'External service temporarily unavailable.',
  NETWORK_ERROR: 'Network error, please try again later.',
  INSUFFICIENT_BALANCE: 'Insufficient balance, please recharge.',
  SENSITIVE_CONTENT:
    'Content may contain sensitive information, please modify and retry.',
  GENERATION_TIMEOUT: 'Generation timed out, please retry.',
  GENERATION_FAILED: 'Generation failed, please try again later.',
  WATCHDOG_TIMEOUT: 'Task execution timed out.',
  WORKER_EXECUTION_ERROR: 'Task execution failed, please try again later.',
  INTERNAL_ERROR: 'Internal system error, please try again later.',
}

type UserErrorMessageLocale = 'zh' | 'en'

function resolveUserErrorMessageLocale(locale?: string | null): UserErrorMessageLocale {
  const detectedLanguage = locale || i18n.language || i18n.resolvedLanguage
  return detectedLanguage?.toLowerCase().startsWith('zh') ? 'zh' : 'en'
}

export function getUserMessageByCode(code: UnifiedErrorCode, locale?: string | null) {
  const resolvedLocale = resolveUserErrorMessageLocale(locale)
  return resolvedLocale === 'zh' ? USER_ERROR_MESSAGES_ZH[code] : USER_ERROR_MESSAGES_EN[code]
}
