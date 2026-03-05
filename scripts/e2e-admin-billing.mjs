#!/usr/bin/env node

import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'

const SCRIPT_DIR = path.dirname(new URL(import.meta.url).pathname)
const PROJECT_ROOT = path.resolve(SCRIPT_DIR, '..')
const DEFAULT_OUT_DIR = path.join(PROJECT_ROOT, 'reports')
const DEFAULT_TIMEOUT_MS = 15000
const VALID_BILLING_MODES = new Set(['OFF', 'SHADOW', 'ENFORCE'])

function assert(condition, message) {
  if (!condition) {
    throw new Error(message)
  }
}

function parseArgs(argv) {
  const args = {
    dryRun: false,
  }

  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index]
    assert(token.startsWith('--'), `Unsupported argument: ${token}`)

    if (token === '--dry-run') {
      args.dryRun = true
      continue
    }

    const key = token.slice(2)
    const value = argv[index + 1]
    assert(value && !value.startsWith('--'), `Missing value for --${key}`)
    args[key] = value
    index += 1
  }

  return args
}

function normalizeBaseUrl(raw, name) {
  assert(typeof raw === 'string' && raw.trim().length > 0, `${name} is required`)
  const normalized = raw.trim()
  assert(/^https?:\/\//.test(normalized), `${name} must start with http:// or https://`)
  return normalized.endsWith('/') ? normalized : `${normalized}/`
}

function safeJsonParse(raw) {
  try {
    return { ok: true, value: JSON.parse(raw) }
  } catch {
    return { ok: false, value: null }
  }
}

function isPlainObject(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
}

function collectTopLevelKeys(value) {
  if (!isPlainObject(value)) {
    return null
  }
  return Object.keys(value).sort()
}

function ensureJsonObject(value, label, failures) {
  if (!isPlainObject(value)) {
    failures.push(`${label} is not a JSON object`)
    return null
  }
  return value
}

function ensureKeys(value, keys, label, failures) {
  const missing = keys.filter((key) => !(key in value))
  if (missing.length > 0) {
    failures.push(`${label} missing keys: ${missing.join(', ')}`)
  }
}

function truncate(text, maxLength = 240) {
  if (typeof text !== 'string') {
    return ''
  }
  if (text.length <= maxLength) {
    return text
  }
  return `${text.slice(0, maxLength)}...`
}

function createRow({
  id,
  method,
  path: requestPath,
  status,
  expected,
  actual,
  durationMs = 0,
  url = null,
  responseKeys = null,
  failures = [],
  note = null,
}) {
  return {
    id,
    method,
    path: requestPath,
    status,
    expected,
    actual,
    durationMs,
    url,
    responseKeys,
    failures,
    note,
  }
}

function createSkippedRow({ id, method, path: requestPath, expected, reason }) {
  return createRow({
    id,
    method,
    path: requestPath,
    status: 'SKIP',
    expected,
    actual: '-',
    note: reason,
  })
}

function evaluateBillingMode(rawValue) {
  const trimmed = typeof rawValue === 'string' ? rawValue.trim() : ''
  const expected = 'OFF|SHADOW|ENFORCE'

  if (trimmed.length === 0) {
    return createSkippedRow({
      id: 'env-billing-mode',
      method: 'ENV',
      path: 'BILLING_MODE',
      expected,
      reason: 'BILLING_MODE is not set in this shell environment',
    })
  }

  const normalized = trimmed.toUpperCase()
  if (VALID_BILLING_MODES.has(normalized)) {
    return createRow({
      id: 'env-billing-mode',
      method: 'ENV',
      path: 'BILLING_MODE',
      status: 'PASS',
      expected,
      actual: normalized,
      note: 'local env value is valid',
    })
  }

  return createRow({
    id: 'env-billing-mode',
    method: 'ENV',
    path: 'BILLING_MODE',
    status: 'FAIL',
    expected,
    actual: trimmed,
    failures: [`invalid BILLING_MODE value: ${trimmed}`],
  })
}

async function runRequest({ baseUrl, method, path: requestPath, token, body, timeoutMs = DEFAULT_TIMEOUT_MS }) {
  const targetUrl = new URL(requestPath, baseUrl).toString()
  const headers = {
    Accept: 'application/json',
  }

  if (typeof token === 'string' && token.trim().length > 0) {
    headers.Authorization = `Bearer ${token.trim()}`
  }

  if (body !== undefined) {
    headers['Content-Type'] = 'application/json'
  }

  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), timeoutMs)

  try {
    const response = await fetch(targetUrl, {
      method,
      headers,
      body: body !== undefined ? JSON.stringify(body) : undefined,
      signal: controller.signal,
    })

    const contentType = response.headers.get('content-type') || ''
    const rawBody = await response.text()

    let json = null
    let parseError = null
    if (rawBody.length > 0) {
      const parsed = safeJsonParse(rawBody)
      if (parsed.ok) {
        json = parsed.value
      } else if (contentType.includes('application/json')) {
        parseError = 'response declared JSON but body is not valid JSON'
      }
    }

    return {
      url: targetUrl,
      status: response.status,
      contentType,
      rawBody,
      json,
      responseKeys: collectTopLevelKeys(json),
      parseError,
      requestError: null,
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    return {
      url: targetUrl,
      status: null,
      contentType: '',
      rawBody: '',
      json: null,
      responseKeys: null,
      parseError: null,
      requestError: message,
    }
  } finally {
    clearTimeout(timer)
  }
}

async function executeHttpCase({
  id,
  method,
  path: requestPath,
  baseUrl,
  token,
  body,
  expectedStatuses,
  timeoutMs,
  validateResponse,
}) {
  const startedAt = Date.now()
  const result = await runRequest({
    baseUrl,
    method,
    path: requestPath,
    token,
    body,
    timeoutMs,
  })
  const durationMs = Date.now() - startedAt

  const failures = []
  if (result.requestError) {
    failures.push(`request failed: ${result.requestError}`)
  } else if (!expectedStatuses.includes(result.status)) {
    failures.push(`status ${result.status} not in expected [${expectedStatuses.join(', ')}]`)
    if (result.rawBody.length > 0) {
      failures.push(`response body: ${truncate(result.rawBody)}`)
    }
  }

  if (result.parseError) {
    failures.push(result.parseError)
  }

  if (failures.length === 0 && typeof validateResponse === 'function') {
    validateResponse({
      json: result.json,
      responseKeys: result.responseKeys,
      contentType: result.contentType,
      rawBody: result.rawBody,
      failures,
    })
  }

  return createRow({
    id,
    method,
    path: requestPath,
    status: failures.length === 0 ? 'PASS' : 'FAIL',
    expected: expectedStatuses.join('|'),
    actual: result.status === null ? 'request-error' : String(result.status),
    durationMs,
    url: result.url,
    responseKeys: result.responseKeys,
    failures,
  })
}

function buildRegisterPayload() {
  const suffix = `${Date.now()}${Math.random().toString(36).slice(2, 8)}`
  return {
    username: `e2e_admin_billing_${suffix}`,
    password: `pass_${suffix}_A1`,
  }
}

async function registerUser(baseUrl) {
  const credentials = buildRegisterPayload()
  let issuedToken = null

  const row = await executeHttpCase({
    id: 'setup-register-user',
    method: 'POST',
    path: '/api/auth/register',
    baseUrl,
    body: {
      name: credentials.username,
      password: credentials.password,
    },
    expectedStatuses: [201],
    validateResponse: ({ json, failures }) => {
      const payload = ensureJsonObject(json, 'register response', failures)
      if (!payload) {
        return
      }
      if (typeof payload.token !== 'string' || payload.token.trim().length === 0) {
        failures.push('register response missing token')
        return
      }
      issuedToken = payload.token.trim()
    },
  })

  return {
    row,
    token: issuedToken,
    username: credentials.username,
  }
}

function mdCell(value) {
  if (value === null || value === undefined) {
    return ''
  }
  return String(value).replace(/\|/g, '\\|')
}

function renderMarkdownReport(metadata, summary, rows) {
  const lines = []
  lines.push('# E2E Admin + Billing Report')
  lines.push('')
  lines.push(`- Generated at: ${new Date().toISOString()}`)
  lines.push(`- Base URL: ${metadata.baseUrl}`)
  lines.push(`- BILLING_MODE env: ${metadata.billingModeEnv || 'UNSET'}`)
  lines.push(`- User token source: ${metadata.userTokenSource}`)
  lines.push(`- Admin token provided: ${metadata.adminTokenProvided ? 'yes' : 'no'}`)
  if (metadata.autoRegisteredUser) {
    lines.push(`- Auto registered user: ${metadata.autoRegisteredUser}`)
  }
  lines.push(`- Total: ${summary.total}`)
  lines.push(`- Pass: ${summary.pass}`)
  lines.push(`- Fail: ${summary.fail}`)
  lines.push(`- Skip: ${summary.skip}`)
  lines.push('')

  lines.push('| Case | Method | Path | Result | Expected | Actual | Duration(ms) |')
  lines.push('|---|---|---|---|---|---|---|')
  for (const row of rows) {
    lines.push(
      `| ${mdCell(row.id)} | ${mdCell(row.method)} | ${mdCell(row.path)} | ${mdCell(row.status)} | ${mdCell(row.expected)} | ${mdCell(row.actual)} | ${mdCell(row.durationMs)} |`,
    )
  }

  lines.push('')
  lines.push('## Failures')
  lines.push('')
  const failedRows = rows.filter((row) => row.status === 'FAIL')
  if (failedRows.length === 0) {
    lines.push('- none')
  } else {
    for (const row of failedRows) {
      lines.push(`- ${row.id} (${row.method} ${row.path})`)
      for (const failure of row.failures) {
        lines.push(`  - ${failure}`)
      }
    }
  }

  lines.push('')
  lines.push('## Skipped')
  lines.push('')
  const skippedRows = rows.filter((row) => row.status === 'SKIP')
  if (skippedRows.length === 0) {
    lines.push('- none')
  } else {
    for (const row of skippedRows) {
      lines.push(`- ${row.id}: ${row.note || 'no reason provided'}`)
    }
  }

  lines.push('')
  return `${lines.join('\n')}\n`
}

function writeReports(outDir, metadata, rows) {
  fs.mkdirSync(outDir, { recursive: true })

  const summary = {
    total: rows.length,
    pass: rows.filter((row) => row.status === 'PASS').length,
    fail: rows.filter((row) => row.status === 'FAIL').length,
    skip: rows.filter((row) => row.status === 'SKIP').length,
  }

  const jsonPath = path.join(outDir, 'e2e-admin-billing.json')
  const mdPath = path.join(outDir, 'e2e-admin-billing.md')

  fs.writeFileSync(
    jsonPath,
    JSON.stringify(
      {
        generatedAt: new Date().toISOString(),
        metadata,
        summary,
        rows,
      },
      null,
      2,
    ),
    'utf8',
  )

  fs.writeFileSync(mdPath, renderMarkdownReport(metadata, summary, rows), 'utf8')
  return { summary, jsonPath, mdPath }
}

function printDryRun(args, baseUrl, outDir, billingModeEnv) {
  const userTokenProvided = typeof args.token === 'string' && args.token.trim().length > 0
  const adminTokenProvided = typeof args['admin-token'] === 'string' && args['admin-token'].trim().length > 0

  console.log(`[dry-run] base: ${baseUrl}`)
  console.log(`[dry-run] out dir: ${outDir}`)
  console.log(`[dry-run] BILLING_MODE env: ${billingModeEnv || '(unset)'}`)
  console.log(`[dry-run] user token: ${userTokenProvided ? 'provided by --token' : 'auto-register via /api/auth/register'}`)
  console.log(`[dry-run] admin token: ${adminTokenProvided ? 'provided by --admin-token' : 'not provided (admin positive tests will be skipped)'}`)
  console.log('[dry-run] planned cases:')
  for (const id of [
    'env-billing-mode',
    'setup-register-user',
    'admin-ai-config-no-token',
    'admin-ai-config-non-admin',
    'admin-ai-config-admin-get',
    'admin-ai-config-admin-put',
    'billing-balance-auth',
    'billing-costs-auth',
    'billing-cost-details-auth',
    'billing-transactions-auth',
    'billing-balance-no-token',
    'billing-costs-no-token',
    'billing-cost-details-no-token',
    'billing-transactions-no-token',
  ]) {
    console.log(`- ${id}`)
  }
}

async function main() {
  const args = parseArgs(process.argv.slice(2))
  const baseUrl = normalizeBaseUrl(args.base, 'base')
  const outDir = path.resolve(args['out-dir'] || DEFAULT_OUT_DIR)
  const billingModeEnv = typeof process.env.BILLING_MODE === 'string' ? process.env.BILLING_MODE.trim() : ''

  if (args.token !== undefined) {
    assert(String(args.token).trim().length > 0, '--token cannot be empty')
  }
  if (args['admin-token'] !== undefined) {
    assert(String(args['admin-token']).trim().length > 0, '--admin-token cannot be empty')
  }

  if (args.dryRun) {
    printDryRun(args, baseUrl, outDir, billingModeEnv)
    return
  }

  const rows = []
  rows.push(evaluateBillingMode(billingModeEnv))

  const adminToken = typeof args['admin-token'] === 'string' ? args['admin-token'].trim() : ''
  let userToken = typeof args.token === 'string' ? args.token.trim() : ''
  let userTokenSource = userToken ? '--token' : 'auto-register'
  let autoRegisteredUser = null

  if (userToken) {
    rows.push(
      createSkippedRow({
        id: 'setup-register-user',
        method: 'POST',
        path: '/api/auth/register',
        expected: '201',
        reason: 'skipped because --token was provided',
      }),
    )
  } else {
    const registration = await registerUser(baseUrl)
    rows.push(registration.row)
    if (registration.row.status === 'PASS' && registration.token) {
      userToken = registration.token
      autoRegisteredUser = registration.username
    } else {
      userTokenSource = 'unavailable'
    }
  }

  rows.push(
    await executeHttpCase({
      id: 'admin-ai-config-no-token',
      method: 'GET',
      path: '/api/admin/ai-config',
      baseUrl,
      expectedStatuses: [401],
    }),
  )

  if (userToken) {
    rows.push(
      await executeHttpCase({
        id: 'admin-ai-config-non-admin',
        method: 'GET',
        path: '/api/admin/ai-config',
        baseUrl,
        token: userToken,
        expectedStatuses: [401, 403],
      }),
    )
  } else {
    rows.push(
      createSkippedRow({
        id: 'admin-ai-config-non-admin',
        method: 'GET',
        path: '/api/admin/ai-config',
        expected: '401|403',
        reason: 'skipped because user token is unavailable',
      }),
    )
  }

  let adminPayload = null
  if (adminToken) {
    const adminGetRow = await executeHttpCase({
      id: 'admin-ai-config-admin-get',
      method: 'GET',
      path: '/api/admin/ai-config',
      baseUrl,
      token: adminToken,
      expectedStatuses: [200],
      validateResponse: ({ json, failures }) => {
        const payload = ensureJsonObject(json, 'admin GET response', failures)
        if (!payload) {
          return
        }
        if (!Array.isArray(payload.providers)) {
          failures.push('admin GET response.providers must be an array')
        }
        if (!Array.isArray(payload.models)) {
          failures.push('admin GET response.models must be an array')
        }
        if (failures.length === 0) {
          adminPayload = {
            providers: payload.providers,
            models: payload.models,
          }
        }
      },
    })
    rows.push(adminGetRow)

    if (adminGetRow.status === 'PASS' && adminPayload) {
      rows.push(
        await executeHttpCase({
          id: 'admin-ai-config-admin-put',
          method: 'PUT',
          path: '/api/admin/ai-config',
          baseUrl,
          token: adminToken,
          body: adminPayload,
          expectedStatuses: [200],
          validateResponse: ({ json, failures }) => {
            const payload = ensureJsonObject(json, 'admin PUT response', failures)
            if (!payload) {
              return
            }
            if (!Array.isArray(payload.providers)) {
              failures.push('admin PUT response.providers must be an array')
            }
            if (!Array.isArray(payload.models)) {
              failures.push('admin PUT response.models must be an array')
            }
            if (Array.isArray(payload.providers) && payload.providers.length !== adminPayload.providers.length) {
              failures.push('admin PUT response.providers length mismatch after update')
            }
            if (Array.isArray(payload.models) && payload.models.length !== adminPayload.models.length) {
              failures.push('admin PUT response.models length mismatch after update')
            }
          },
        }),
      )
    } else {
      rows.push(
        createSkippedRow({
          id: 'admin-ai-config-admin-put',
          method: 'PUT',
          path: '/api/admin/ai-config',
          expected: '200',
          reason: 'skipped because admin GET case failed',
        }),
      )
    }
  } else {
    rows.push(
      createSkippedRow({
        id: 'admin-ai-config-admin-get',
        method: 'GET',
        path: '/api/admin/ai-config',
        expected: '200',
        reason: 'skipped because --admin-token was not provided',
      }),
    )
    rows.push(
      createSkippedRow({
        id: 'admin-ai-config-admin-put',
        method: 'PUT',
        path: '/api/admin/ai-config',
        expected: '200',
        reason: 'skipped because --admin-token was not provided',
      }),
    )
  }

  const billingAuthCases = [
    {
      id: 'billing-balance-auth',
      method: 'GET',
      path: '/api/user/balance',
      expectedStatuses: [200],
      validateResponse: ({ json, failures }) => {
        const payload = ensureJsonObject(json, 'billing balance response', failures)
        if (!payload) {
          return
        }
        ensureKeys(payload, ['success', 'currency', 'balance', 'frozenAmount', 'totalSpent'], 'billing balance response', failures)
      },
    },
    {
      id: 'billing-costs-auth',
      method: 'GET',
      path: '/api/user/costs?startDate=2026-01-01&endDate=2026-03-31',
      expectedStatuses: [200],
      validateResponse: ({ json, failures }) => {
        const payload = ensureJsonObject(json, 'billing costs response', failures)
        if (!payload) {
          return
        }
        ensureKeys(payload, ['userId', 'currency', 'total', 'byProject'], 'billing costs response', failures)
        if ('byProject' in payload && !Array.isArray(payload.byProject)) {
          failures.push('billing costs response.byProject must be an array')
        }
      },
    },
    {
      id: 'billing-cost-details-auth',
      method: 'GET',
      path: '/api/user/costs/details?page=1&pageSize=10',
      expectedStatuses: [200],
      validateResponse: ({ json, failures }) => {
        const payload = ensureJsonObject(json, 'billing cost details response', failures)
        if (!payload) {
          return
        }
        ensureKeys(payload, ['success', 'currency', 'records', 'total', 'page', 'pageSize', 'totalPages'], 'billing cost details response', failures)
        if ('records' in payload && !Array.isArray(payload.records)) {
          failures.push('billing cost details response.records must be an array')
        }
      },
    },
    {
      id: 'billing-transactions-auth',
      method: 'GET',
      path: '/api/user/transactions?page=1&pageSize=10',
      expectedStatuses: [200],
      validateResponse: ({ json, failures }) => {
        const payload = ensureJsonObject(json, 'billing transactions response', failures)
        if (!payload) {
          return
        }
        ensureKeys(payload, ['currency', 'transactions', 'pagination'], 'billing transactions response', failures)
        if ('transactions' in payload && !Array.isArray(payload.transactions)) {
          failures.push('billing transactions response.transactions must be an array')
        }
        if ('pagination' in payload) {
          const pagination = ensureJsonObject(payload.pagination, 'billing transactions response.pagination', failures)
          if (pagination) {
            ensureKeys(pagination, ['page', 'pageSize', 'total', 'totalPages'], 'billing transactions response.pagination', failures)
          }
        }
      },
    },
  ]

  if (userToken) {
    for (const testCase of billingAuthCases) {
      rows.push(
        await executeHttpCase({
          ...testCase,
          baseUrl,
          token: userToken,
        }),
      )
    }
  } else {
    for (const testCase of billingAuthCases) {
      rows.push(
        createSkippedRow({
          id: testCase.id,
          method: testCase.method,
          path: testCase.path,
          expected: testCase.expectedStatuses.join('|'),
          reason: 'skipped because user token is unavailable',
        }),
      )
    }
  }

  for (const testCase of [
    {
      id: 'billing-balance-no-token',
      method: 'GET',
      path: '/api/user/balance',
    },
    {
      id: 'billing-costs-no-token',
      method: 'GET',
      path: '/api/user/costs?startDate=2026-01-01&endDate=2026-03-31',
    },
    {
      id: 'billing-cost-details-no-token',
      method: 'GET',
      path: '/api/user/costs/details?page=1&pageSize=10',
    },
    {
      id: 'billing-transactions-no-token',
      method: 'GET',
      path: '/api/user/transactions?page=1&pageSize=10',
    },
  ]) {
    rows.push(
      await executeHttpCase({
        ...testCase,
        baseUrl,
        expectedStatuses: [401],
      }),
    )
  }

  const metadata = {
    baseUrl,
    billingModeEnv: billingModeEnv || null,
    userTokenSource,
    adminTokenProvided: adminToken.length > 0,
    autoRegisteredUser,
  }

  const { summary, jsonPath, mdPath } = writeReports(outDir, metadata, rows)
  console.log(`wrote ${jsonPath}`)
  console.log(`wrote ${mdPath}`)
  console.log(`PASS=${summary.pass} FAIL=${summary.fail} SKIP=${summary.skip} TOTAL=${summary.total}`)

  if (summary.fail > 0) {
    process.exitCode = 2
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error))
  process.exitCode = 1
})
