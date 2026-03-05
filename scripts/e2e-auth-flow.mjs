#!/usr/bin/env node

import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'

const SCRIPT_DIR = path.dirname(new URL(import.meta.url).pathname)
const PROJECT_ROOT = path.resolve(SCRIPT_DIR, '..')
const DEFAULT_OUT_DIR = path.join(PROJECT_ROOT, 'reports')
const DEFAULT_TIMEOUT_MS = 15000

const PROTECTED_ENDPOINTS = [
  {
    id: 'protected-user-preference',
    method: 'GET',
    path: '/api/user-preference',
    expectedShape: 'preference object',
    verify(payload, failures) {
      assertObject(payload, failures, 'response body must be a JSON object')
      assertObject(payload?.preference, failures, 'response.preference must be an object')
    },
  },
  {
    id: 'protected-user-models',
    method: 'GET',
    path: '/api/user/models',
    expectedShape: 'llm/image/video/audio/lipsync arrays',
    verify(payload, failures) {
      assertObject(payload, failures, 'response body must be a JSON object')
      assertArray(payload?.llm, failures, 'response.llm must be an array')
      assertArray(payload?.image, failures, 'response.image must be an array')
      assertArray(payload?.video, failures, 'response.video must be an array')
      assertArray(payload?.audio, failures, 'response.audio must be an array')
      assertArray(payload?.lipsync, failures, 'response.lipsync must be an array')
    },
  },
  {
    id: 'protected-user-api-config',
    method: 'GET',
    path: '/api/user/api-config',
    expectedShape: 'models/providers/defaultModels',
    verify(payload, failures) {
      assertObject(payload, failures, 'response body must be a JSON object')
      assertArray(payload?.models, failures, 'response.models must be an array')
      assertArray(payload?.providers, failures, 'response.providers must be an array')
      assertObject(payload?.defaultModels, failures, 'response.defaultModels must be an object')
    },
  },
  {
    id: 'protected-projects-list',
    method: 'GET',
    path: '/api/projects?page=1&pageSize=5',
    expectedShape: 'projects array',
    verify(payload, failures) {
      assertObject(payload, failures, 'response body must be a JSON object')
      assertArray(payload?.projects, failures, 'response.projects must be an array')
    },
  },
  {
    id: 'protected-asset-hub-folders',
    method: 'GET',
    path: '/api/asset-hub/folders',
    expectedShape: 'folders array',
    verify(payload, failures) {
      assertObject(payload, failures, 'response body must be a JSON object')
      assertArray(payload?.folders, failures, 'response.folders must be an array')
    },
  },
]

function assert(condition, message) {
  if (!condition) {
    throw new Error(message)
  }
}

function assertObject(value, failures, message) {
  if (!isObject(value)) {
    failures.push(message)
  }
}

function assertArray(value, failures, message) {
  if (!Array.isArray(value)) {
    failures.push(message)
  }
}

function isObject(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
}

function parseArgs(argv) {
  const args = {
    dryRun: false,
  }

  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i]
    assert(token.startsWith('--'), `Unsupported argument: ${token}`)

    if (token === '--dry-run') {
      args.dryRun = true
      continue
    }

    const key = token.slice(2)
    const value = argv[i + 1]
    assert(value && !value.startsWith('--'), `Missing value for --${key}`)
    args[key] = value
    i += 1
  }

  return args
}

function normalizeBaseUrl(raw, name) {
  assert(typeof raw === 'string' && raw.trim().length > 0, `${name} is required`)
  const normalized = raw.trim()
  assert(/^https?:\/\//.test(normalized), `${name} must start with http:// or https://`)
  return normalized.endsWith('/') ? normalized : `${normalized}/`
}

function parseOptionalInt(raw, name) {
  if (raw === undefined) {
    return undefined
  }

  const parsed = Number(raw)
  assert(Number.isInteger(parsed) && parsed > 0, `${name} must be a positive integer`)
  return parsed
}

function safeJsonParse(raw) {
  try {
    return { ok: true, value: JSON.parse(raw) }
  } catch {
    return { ok: false, value: null }
  }
}

function truncateText(input, maxLength = 300) {
  const value = String(input ?? '').replace(/\s+/g, ' ').trim()
  if (value.length <= maxLength) {
    return value
  }
  return `${value.slice(0, maxLength - 3)}...`
}

function summarizePayload(rawBody, json) {
  if (json !== null && json !== undefined) {
    return truncateText(JSON.stringify(json), 320)
  }

  if (rawBody.length === 0) {
    return '<empty>'
  }

  return truncateText(rawBody, 320)
}

function base64UrlToUtf8(segment) {
  const normalized = segment.replace(/-/g, '+').replace(/_/g, '/')
  const missingPadding = normalized.length % 4
  const padded = missingPadding === 0 ? normalized : `${normalized}${'='.repeat(4 - missingPadding)}`
  return Buffer.from(padded, 'base64').toString('utf8')
}

function decodeJwtSegment(segment) {
  const decoded = base64UrlToUtf8(segment)
  return JSON.parse(decoded)
}

function validateJwtToken(token) {
  const failures = []

  if (typeof token !== 'string' || token.trim().length === 0) {
    failures.push('token must be a non-empty string')
    return { ok: false, failures, header: null, payload: null }
  }

  const segments = token.split('.')
  if (segments.length !== 3) {
    failures.push('token is not in JWT format (header.payload.signature)')
    return { ok: false, failures, header: null, payload: null }
  }

  let header = null
  let payload = null

  try {
    header = decodeJwtSegment(segments[0])
  } catch {
    failures.push('token header is not valid base64url JSON')
  }

  try {
    payload = decodeJwtSegment(segments[1])
  } catch {
    failures.push('token payload is not valid base64url JSON')
  }

  if (!isObject(header)) {
    failures.push('token header must be a JSON object')
  }
  if (!isObject(payload)) {
    failures.push('token payload must be a JSON object')
  }

  if (isObject(payload) && typeof payload.sub !== 'string') {
    failures.push('token payload.sub must be a string')
  }

  if (isObject(payload) && payload.exp !== undefined && typeof payload.exp !== 'number') {
    failures.push('token payload.exp must be a number when present')
  }

  return {
    ok: failures.length === 0,
    failures,
    header,
    payload,
  }
}

function extractToken(payload) {
  if (!isObject(payload)) {
    return null
  }

  if (typeof payload.token === 'string' && payload.token.trim().length > 0) {
    return payload.token.trim()
  }

  return null
}

function extractRefreshToken(payload) {
  if (!isObject(payload)) {
    return null
  }

  if (typeof payload.refreshToken === 'string' && payload.refreshToken.trim().length > 0) {
    return payload.refreshToken.trim()
  }

  if (typeof payload.refresh_token === 'string' && payload.refresh_token.trim().length > 0) {
    return payload.refresh_token.trim()
  }

  return null
}

function getSetCookieValues(headers) {
  if (typeof headers.getSetCookie === 'function') {
    const values = headers.getSetCookie()
    if (Array.isArray(values)) {
      return values
    }
  }

  const single = headers.get('set-cookie')
  return single ? [single] : []
}

function extractTokenFromSetCookie(setCookies) {
  for (const cookie of setCookies) {
    const match = cookie.match(/(?:^|\s|,)token=([^;\s,]+)/i)
    if (!match) {
      continue
    }

    try {
      return decodeURIComponent(match[1])
    } catch {
      return match[1]
    }
  }

  return null
}

async function runRequest({
  baseUrl,
  method,
  path: apiPath,
  timeoutMs,
  token,
  headers = {},
  body,
}) {
  const targetUrl = new URL(apiPath, baseUrl).toString()
  const requestHeaders = {
    Accept: 'application/json',
    ...headers,
  }

  if (token && !requestHeaders.Authorization) {
    requestHeaders.Authorization = `Bearer ${token}`
  }

  if (body !== undefined && !requestHeaders['Content-Type']) {
    requestHeaders['Content-Type'] = 'application/json'
  }

  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), timeoutMs)

  try {
    const response = await fetch(targetUrl, {
      method,
      headers: requestHeaders,
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: controller.signal,
    })

    const rawBody = await response.text()
    const parsed = safeJsonParse(rawBody)
    const json = parsed.ok ? parsed.value : null
    const setCookies = getSetCookieValues(response.headers)

    return {
      url: targetUrl,
      status: response.status,
      rawBody,
      json,
      parseError: parsed.ok || rawBody.length === 0 ? null : 'response is not valid JSON',
      summary: summarizePayload(rawBody, json),
      setCookies,
      cookieToken: extractTokenFromSetCookie(setCookies),
      requestError: null,
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    return {
      url: targetUrl,
      status: null,
      rawBody: '',
      json: null,
      parseError: null,
      summary: '<request failed>',
      setCookies: [],
      cookieToken: null,
      requestError: message,
    }
  } finally {
    clearTimeout(timer)
  }
}

function createStepResult({
  id,
  title,
  method,
  path: apiPath,
  expectedStatus,
  actualStatus,
  responseSummary,
  requestError,
  failures,
  warnings,
}) {
  return {
    id,
    title,
    method,
    path: apiPath,
    expectedStatus,
    actualStatus,
    responseSummary,
    requestError,
    failures,
    warnings,
    status: failures.length === 0 ? 'PASS' : 'FAIL',
  }
}

function printStepResult(step) {
  const actualStatus = step.actualStatus === null ? 'N/A' : String(step.actualStatus)
  console.log(`[${step.status}] ${step.id} ${step.method} ${step.path}`)
  console.log(`  expected: ${step.expectedStatus} | actual: ${actualStatus}`)
  console.log(`  response: ${step.responseSummary}`)

  if (step.requestError) {
    console.log(`  request error: ${step.requestError}`)
  }

  for (const failure of step.failures) {
    console.log(`  failure: ${failure}`)
  }

  for (const warning of step.warnings) {
    console.log(`  warning: ${warning}`)
  }
}

function createDependencyFailureStep({ id, title, method, path: apiPath, message }) {
  return createStepResult({
    id,
    title,
    method,
    path: apiPath,
    expectedStatus: 'not executed',
    actualStatus: null,
    responseSummary: '<not executed>',
    requestError: null,
    failures: [message],
    warnings: [],
  })
}

async function runProtectedChecks({
  baseUrl,
  timeoutMs,
  token,
  mode,
}) {
  const steps = []

  for (const endpoint of PROTECTED_ENDPOINTS) {
    const requestToken = mode === 'with-token' ? token : mode === 'invalid-token' ? 'invalid-garbage-token' : null
    const expectedStatus = mode === 'with-token' ? '200' : '401'

    if (mode === 'with-token' && !requestToken) {
      const dependencyFailure = createDependencyFailureStep({
        id: `${endpoint.id}-with-token`,
        title: `Access protected endpoint with valid token (${endpoint.expectedShape})`,
        method: endpoint.method,
        path: endpoint.path,
        message: 'missing token from register step',
      })
      printStepResult(dependencyFailure)
      steps.push(dependencyFailure)
      continue
    }

    const result = await runRequest({
      baseUrl,
      timeoutMs,
      method: endpoint.method,
      path: endpoint.path,
      token: requestToken,
    })

    const failures = []
    const warnings = []

    if (result.requestError) {
      failures.push(`request failed: ${result.requestError}`)
    }

    if (result.parseError && result.status !== 401) {
      warnings.push(result.parseError)
    }

    if (mode === 'with-token') {
      if (result.status !== 200) {
        failures.push(`status ${result.status} != expected 200`)
      }
      endpoint.verify(result.json, failures)
    } else if (result.status !== 401) {
      failures.push(`status ${result.status} != expected 401`)
    }

    const step = createStepResult({
      id: `${endpoint.id}-${mode}`,
      title: mode === 'with-token'
        ? `Access protected endpoint with valid token (${endpoint.expectedShape})`
        : mode === 'without-token'
          ? 'Access protected endpoint without token'
          : 'Access protected endpoint with invalid token',
      method: endpoint.method,
      path: endpoint.path,
      expectedStatus,
      actualStatus: result.status,
      responseSummary: result.summary,
      requestError: result.requestError,
      failures,
      warnings,
    })

    printStepResult(step)
    steps.push(step)
  }

  return steps
}

function escapeMarkdownCell(value) {
  return String(value ?? '')
    .replace(/\|/g, '\\|')
    .replace(/\n/g, ' ')
}

function renderMarkdownReport({
  generatedAt,
  baseUrl,
  testUser,
  summary,
  steps,
}) {
  const lines = []
  lines.push('# E2E Auth Flow Report')
  lines.push('')
  lines.push(`- Generated at: ${generatedAt}`)
  lines.push(`- Base URL: ${baseUrl}`)
  lines.push(`- Test user: ${testUser}`)
  lines.push(`- Total steps: ${summary.total}`)
  lines.push(`- Pass: ${summary.pass}`)
  lines.push(`- Fail: ${summary.fail}`)
  lines.push('')
  lines.push('| Step | Method | Path | Result | Expected | Actual | Detail |')
  lines.push('|---|---|---|---|---|---|---|')

  for (const step of steps) {
    const detail = step.failures[0] || step.warnings[0] || step.responseSummary
    lines.push(
      `| ${escapeMarkdownCell(step.id)} | ${escapeMarkdownCell(step.method)} | ${escapeMarkdownCell(step.path)} | ${escapeMarkdownCell(step.status)} | ${escapeMarkdownCell(step.expectedStatus)} | ${escapeMarkdownCell(step.actualStatus ?? 'N/A')} | ${escapeMarkdownCell(detail)} |`,
    )
  }

  lines.push('')
  lines.push('## Failures')
  lines.push('')

  const failed = steps.filter((step) => step.status === 'FAIL')
  if (failed.length === 0) {
    lines.push('- none')
  } else {
    for (const step of failed) {
      lines.push(`- ${step.id} (${step.method} ${step.path})`)
      for (const failure of step.failures) {
        lines.push(`  - ${failure}`)
      }
      if (step.requestError) {
        lines.push(`  - request error: ${step.requestError}`)
      }
      lines.push(`  - response: ${step.responseSummary}`)
    }
  }

  lines.push('')
  lines.push('## Warnings')
  lines.push('')

  const warned = steps.filter((step) => step.warnings.length > 0)
  if (warned.length === 0) {
    lines.push('- none')
  } else {
    for (const step of warned) {
      lines.push(`- ${step.id} (${step.method} ${step.path})`)
      for (const warning of step.warnings) {
        lines.push(`  - ${warning}`)
      }
    }
  }

  lines.push('')
  return `${lines.join('\n')}\n`
}

function writeReports({ outDir, baseUrl, testUser, steps }) {
  fs.mkdirSync(outDir, { recursive: true })

  const summary = {
    total: steps.length,
    pass: steps.filter((step) => step.status === 'PASS').length,
    fail: steps.filter((step) => step.status === 'FAIL').length,
  }

  const generatedAt = new Date().toISOString()
  const report = {
    generatedAt,
    baseUrl,
    testUser,
    summary,
    steps,
  }

  const jsonPath = path.join(outDir, 'e2e-auth-flow.json')
  const mdPath = path.join(outDir, 'e2e-auth-flow.md')

  fs.writeFileSync(jsonPath, JSON.stringify(report, null, 2), 'utf8')
  fs.writeFileSync(mdPath, renderMarkdownReport(report), 'utf8')

  return {
    summary,
    jsonPath,
    mdPath,
  }
}

async function runFlow({ baseUrl, timeoutMs, testUser, testPassword }) {
  const steps = []

  let accessToken = null
  let refreshToken = null

  const registerResult = await runRequest({
    baseUrl,
    timeoutMs,
    method: 'POST',
    path: '/api/auth/register',
    body: {
      name: testUser,
      email: testUser,
      password: testPassword,
    },
  })

  {
    const failures = []
    const warnings = []

    if (registerResult.requestError) {
      failures.push(`request failed: ${registerResult.requestError}`)
    }

    if (registerResult.status !== 200 && registerResult.status !== 201) {
      failures.push(`status ${registerResult.status} != expected 200/201`)
    }

    if (registerResult.parseError) {
      failures.push(registerResult.parseError)
    }

    const bodyToken = extractToken(registerResult.json)
    const cookieToken = registerResult.cookieToken

    if (!bodyToken) {
      failures.push('response.token is required')
      if (cookieToken) {
        warnings.push('falling back to token from Set-Cookie for subsequent steps')
      }
    }

    const selectedToken = bodyToken || cookieToken
    if (selectedToken) {
      const tokenValidation = validateJwtToken(selectedToken)
      if (!tokenValidation.ok) {
        failures.push(...tokenValidation.failures.map((item) => `token validation failed: ${item}`))
      }
      accessToken = selectedToken
    }

    refreshToken = extractRefreshToken(registerResult.json)
    if (!refreshToken) {
      warnings.push('response.refreshToken missing; refresh step will reuse access token as request body value')
    }

    const step = createStepResult({
      id: 'auth-register',
      title: 'Register a new user',
      method: 'POST',
      path: '/api/auth/register',
      expectedStatus: '200 or 201',
      actualStatus: registerResult.status,
      responseSummary: registerResult.summary,
      requestError: registerResult.requestError,
      failures,
      warnings,
    })
    printStepResult(step)
    steps.push(step)
  }

  const withTokenSteps = await runProtectedChecks({
    baseUrl,
    timeoutMs,
    token: accessToken,
    mode: 'with-token',
  })
  steps.push(...withTokenSteps)

  const withoutTokenSteps = await runProtectedChecks({
    baseUrl,
    timeoutMs,
    token: null,
    mode: 'without-token',
  })
  steps.push(...withoutTokenSteps)

  const invalidTokenSteps = await runProtectedChecks({
    baseUrl,
    timeoutMs,
    token: null,
    mode: 'invalid-token',
  })
  steps.push(...invalidTokenSteps)

  if (!accessToken) {
    const dependencyFailure = createDependencyFailureStep({
      id: 'auth-refresh',
      title: 'Refresh token',
      method: 'POST',
      path: '/api/auth/refresh',
      message: 'cannot refresh because register step did not produce a usable token',
    })
    printStepResult(dependencyFailure)
    steps.push(dependencyFailure)
  } else {
    const refreshBody = {
      refreshToken: refreshToken || accessToken,
    }

    const refreshResult = await runRequest({
      baseUrl,
      timeoutMs,
      method: 'POST',
      path: '/api/auth/refresh',
      token: accessToken,
      body: refreshBody,
    })

    const failures = []
    const warnings = []

    if (refreshResult.requestError) {
      failures.push(`request failed: ${refreshResult.requestError}`)
    }

    if (refreshResult.status !== 200) {
      failures.push(`status ${refreshResult.status} != expected 200`)
    }

    if (refreshResult.parseError) {
      failures.push(refreshResult.parseError)
    }

    const refreshedToken = extractToken(refreshResult.json) || refreshResult.cookieToken
    if (!extractToken(refreshResult.json) && refreshResult.cookieToken) {
      warnings.push('response.token missing; using token from Set-Cookie fallback')
    }

    if (!refreshedToken) {
      failures.push('response did not contain a usable token')
    } else {
      const tokenValidation = validateJwtToken(refreshedToken)
      if (!tokenValidation.ok) {
        failures.push(...tokenValidation.failures.map((item) => `token validation failed: ${item}`))
      }
      accessToken = refreshedToken
    }

    const refreshedRefreshToken = extractRefreshToken(refreshResult.json)
    if (refreshedRefreshToken) {
      refreshToken = refreshedRefreshToken
    }

    const step = createStepResult({
      id: 'auth-refresh',
      title: 'Refresh token',
      method: 'POST',
      path: '/api/auth/refresh',
      expectedStatus: '200',
      actualStatus: refreshResult.status,
      responseSummary: refreshResult.summary,
      requestError: refreshResult.requestError,
      failures,
      warnings,
    })
    printStepResult(step)
    steps.push(step)
  }

  const loginResult = await runRequest({
    baseUrl,
    timeoutMs,
    method: 'POST',
    path: '/api/auth/login',
    body: {
      username: testUser,
      password: testPassword,
    },
  })

  {
    const failures = []
    const warnings = []

    if (loginResult.requestError) {
      failures.push(`request failed: ${loginResult.requestError}`)
    }

    if (loginResult.status !== 200) {
      failures.push(`status ${loginResult.status} != expected 200`)
    }

    if (loginResult.parseError) {
      failures.push(loginResult.parseError)
    }

    const loginToken = extractToken(loginResult.json)
    if (!loginToken) {
      failures.push('response.token is required')
    } else {
      const tokenValidation = validateJwtToken(loginToken)
      if (!tokenValidation.ok) {
        failures.push(...tokenValidation.failures.map((item) => `token validation failed: ${item}`))
      }
      accessToken = loginToken
    }

    const loginRefreshToken = extractRefreshToken(loginResult.json)
    if (loginRefreshToken) {
      refreshToken = loginRefreshToken
    }

    const user = isObject(loginResult.json) ? loginResult.json.user : null
    if (!isObject(user)) {
      failures.push('response.user must be an object')
    } else {
      if (typeof user.id !== 'string' || user.id.trim().length === 0) {
        failures.push('response.user.id must be a non-empty string')
      }
      const hasUsername = typeof user.username === 'string' && user.username.trim().length > 0
      const hasName = typeof user.name === 'string' && user.name.trim().length > 0
      if (!hasUsername && !hasName) {
        failures.push('response.user must include username or name')
      }
      if (typeof user.role !== 'string' || user.role.trim().length === 0) {
        failures.push('response.user.role must be a non-empty string')
      }
    }

    if (refreshToken) {
      warnings.push('refreshToken captured from auth flow')
    }

    const step = createStepResult({
      id: 'auth-login',
      title: 'Login with registered credentials',
      method: 'POST',
      path: '/api/auth/login',
      expectedStatus: '200',
      actualStatus: loginResult.status,
      responseSummary: loginResult.summary,
      requestError: loginResult.requestError,
      failures,
      warnings,
    })
    printStepResult(step)
    steps.push(step)
  }

  return steps
}

async function main() {
  const args = parseArgs(process.argv.slice(2))
  const baseUrl = normalizeBaseUrl(args.base, 'base')
  const outDir = path.resolve(args['out-dir'] || DEFAULT_OUT_DIR)
  const timeoutMs = parseOptionalInt(args['timeout-ms'], 'timeout-ms') || DEFAULT_TIMEOUT_MS

  const timestamp = Date.now()
  const testUser = `e2e-auth-${timestamp}@test.com`
  const testPassword = `E2E-Auth-${timestamp}!`

  if (args.dryRun) {
    console.log(`[dry-run] base: ${baseUrl}`)
    console.log(`[dry-run] out-dir: ${outDir}`)
    console.log(`[dry-run] timeout-ms: ${timeoutMs}`)
    console.log(`[dry-run] test user: ${testUser}`)
    console.log('[dry-run] planned steps:')
    console.log('- auth-register: POST /api/auth/register')
    for (const endpoint of PROTECTED_ENDPOINTS) {
      console.log(`- ${endpoint.id}-with-token: ${endpoint.method} ${endpoint.path}`)
    }
    for (const endpoint of PROTECTED_ENDPOINTS) {
      console.log(`- ${endpoint.id}-without-token: ${endpoint.method} ${endpoint.path}`)
    }
    for (const endpoint of PROTECTED_ENDPOINTS) {
      console.log(`- ${endpoint.id}-invalid-token: ${endpoint.method} ${endpoint.path}`)
    }
    console.log('- auth-refresh: POST /api/auth/refresh')
    console.log('- auth-login: POST /api/auth/login')
    return
  }

  const steps = await runFlow({
    baseUrl,
    timeoutMs,
    testUser,
    testPassword,
  })

  const { summary, jsonPath, mdPath } = writeReports({
    outDir,
    baseUrl,
    testUser,
    steps,
  })

  console.log(`wrote ${jsonPath}`)
  console.log(`wrote ${mdPath}`)
  console.log(`PASS=${summary.pass} FAIL=${summary.fail} TOTAL=${summary.total}`)

  if (summary.fail > 0) {
    process.exitCode = 2
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error))
  process.exitCode = 1
})
