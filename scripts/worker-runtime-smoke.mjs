#!/usr/bin/env node

import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'

const VALID_METHODS = new Set(['POST', 'PUT', 'PATCH'])
const TERMINAL_STATUSES = new Set(['completed', 'failed', 'dismissed'])
const TEMPLATE_RE = /\{\{([A-Z0-9_]+)\}\}/g

const ALLOWED_CASE_KEYS = new Set([
  'id',
  'method',
  'path',
  'headers',
  'body',
  'requireAuth',
  'expectedSubmitStatus',
  'expectTerminalStatus',
  'expectResultKeys',
  'expectErrorCode',
  'timeoutMs',
  'pollIntervalMs',
  'requiresEnv',
  'skipIfEnvMissing',
])

const SCRIPT_DIR = path.dirname(new URL(import.meta.url).pathname)
const PROJECT_ROOT = path.resolve(SCRIPT_DIR, '..')
const DEFAULT_CASES_PATH = path.join(SCRIPT_DIR, 'worker-runtime-cases.sample.json')
const DEFAULT_OUT_DIR = path.join(PROJECT_ROOT, 'reports')
const DEFAULT_TIMEOUT_MS = 240000
const DEFAULT_POLL_INTERVAL_MS = 2000

function assert(condition, message) {
  if (!condition) {
    throw new Error(message)
  }
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
  if (raw === undefined) return undefined
  const parsed = Number(raw)
  assert(Number.isInteger(parsed) && parsed > 0, `${name} must be a positive integer`)
  return parsed
}

function validateHeaders(headers, caseId) {
  assert(headers !== null && typeof headers === 'object' && !Array.isArray(headers), `${caseId}: headers must be an object`)
  for (const [key, value] of Object.entries(headers)) {
    assert(typeof key === 'string' && key.trim().length > 0, `${caseId}: header key must be a non-empty string`)
    assert(typeof value === 'string', `${caseId}: header ${key} must be string`)
  }
}

function validateStringArray(input, caseId, keyName) {
  assert(Array.isArray(input), `${caseId}: ${keyName} must be an array`)
  for (const value of input) {
    assert(typeof value === 'string' && value.trim().length > 0, `${caseId}: ${keyName} values must be non-empty strings`)
  }
}

function validateStatusArray(statuses, caseId) {
  validateStringArray(statuses, caseId, 'expectTerminalStatus')
  for (const status of statuses) {
    assert(TERMINAL_STATUSES.has(status), `${caseId}: unsupported terminal status ${status}`)
  }
}

function validateRequiresEnv(requiresEnv, caseId) {
  validateStringArray(requiresEnv, caseId, 'requiresEnv')
  for (const envName of requiresEnv) {
    assert(/^[A-Z0-9_]+$/.test(envName), `${caseId}: invalid env var name ${envName}`)
  }
}

function collectTemplateVars(value, out) {
  if (typeof value === 'string') {
    let match = TEMPLATE_RE.exec(value)
    while (match) {
      out.add(match[1])
      match = TEMPLATE_RE.exec(value)
    }
    TEMPLATE_RE.lastIndex = 0
    return
  }

  if (Array.isArray(value)) {
    for (const item of value) {
      collectTemplateVars(item, out)
    }
    return
  }

  if (value !== null && typeof value === 'object') {
    for (const item of Object.values(value)) {
      collectTemplateVars(item, out)
    }
  }
}

function resolveTemplateString(raw, env) {
  return raw.replace(TEMPLATE_RE, (_, envName) => String(env[envName]))
}

function resolveTemplates(value, env) {
  if (typeof value === 'string') {
    return resolveTemplateString(value, env)
  }
  if (Array.isArray(value)) {
    return value.map((item) => resolveTemplates(item, env))
  }
  if (value !== null && typeof value === 'object') {
    const out = {}
    for (const [key, item] of Object.entries(value)) {
      out[key] = resolveTemplates(item, env)
    }
    return out
  }
  return value
}

function validateCaseEntry(entry, index) {
  const caseId = `case[${index}]`
  assert(entry !== null && typeof entry === 'object' && !Array.isArray(entry), `${caseId}: must be an object`)

  for (const key of Object.keys(entry)) {
    assert(ALLOWED_CASE_KEYS.has(key), `${caseId}: unsupported key ${key}`)
  }

  assert(typeof entry.id === 'string' && entry.id.trim().length > 0, `${caseId}: id is required`)
  assert(typeof entry.path === 'string' && entry.path.startsWith('/'), `${entry.id}: path must start with /`)

  const method = (entry.method || 'POST').trim().toUpperCase()
  assert(VALID_METHODS.has(method), `${entry.id}: unsupported method ${method}`)

  if (entry.headers !== undefined) {
    validateHeaders(entry.headers, entry.id)
  }

  if (entry.requireAuth !== undefined) {
    assert(typeof entry.requireAuth === 'boolean', `${entry.id}: requireAuth must be boolean`)
  }

  if (entry.expectedSubmitStatus !== undefined) {
    assert(Number.isInteger(entry.expectedSubmitStatus), `${entry.id}: expectedSubmitStatus must be an integer`)
    assert(entry.expectedSubmitStatus >= 100 && entry.expectedSubmitStatus <= 599, `${entry.id}: expectedSubmitStatus must be a valid HTTP status`)
  }

  if (entry.expectTerminalStatus !== undefined) {
    validateStatusArray(entry.expectTerminalStatus, entry.id)
  }

  if (entry.expectResultKeys !== undefined) {
    validateStringArray(entry.expectResultKeys, entry.id, 'expectResultKeys')
  }

  if (entry.expectErrorCode !== undefined) {
    assert(typeof entry.expectErrorCode === 'string' && entry.expectErrorCode.trim().length > 0, `${entry.id}: expectErrorCode must be a non-empty string`)
  }

  if (entry.timeoutMs !== undefined) {
    assert(Number.isInteger(entry.timeoutMs) && entry.timeoutMs > 0, `${entry.id}: timeoutMs must be a positive integer`)
  }

  if (entry.pollIntervalMs !== undefined) {
    assert(Number.isInteger(entry.pollIntervalMs) && entry.pollIntervalMs > 0, `${entry.id}: pollIntervalMs must be a positive integer`)
  }

  if (entry.requiresEnv !== undefined) {
    validateRequiresEnv(entry.requiresEnv, entry.id)
  }

  if (entry.skipIfEnvMissing !== undefined) {
    assert(typeof entry.skipIfEnvMissing === 'boolean', `${entry.id}: skipIfEnvMissing must be boolean`)
  }

  return {
    id: entry.id,
    method,
    path: entry.path,
    headers: entry.headers ?? {},
    body: entry.body ?? {},
    requireAuth: entry.requireAuth ?? true,
    expectedSubmitStatus: entry.expectedSubmitStatus ?? 200,
    expectTerminalStatus: entry.expectTerminalStatus ?? ['completed', 'failed'],
    expectResultKeys: entry.expectResultKeys ?? [],
    expectErrorCode: entry.expectErrorCode,
    timeoutMs: entry.timeoutMs ?? DEFAULT_TIMEOUT_MS,
    pollIntervalMs: entry.pollIntervalMs ?? DEFAULT_POLL_INTERVAL_MS,
    requiresEnv: entry.requiresEnv ?? [],
    skipIfEnvMissing: entry.skipIfEnvMissing ?? false,
  }
}

function loadCases(casesPath) {
  assert(fs.existsSync(casesPath), `cases file not found: ${casesPath}`)
  const raw = fs.readFileSync(casesPath, 'utf8')
  const parsed = JSON.parse(raw)
  assert(Array.isArray(parsed), 'cases file must be a JSON array')
  assert(parsed.length > 0, 'cases file must not be empty')
  return parsed.map((entry, index) => validateCaseEntry(entry, index))
}

function prepareCases(cases, env) {
  const activeCases = []
  const skippedCases = []

  for (const testCase of cases) {
    const requiredEnv = new Set(testCase.requiresEnv)
    collectTemplateVars(testCase.path, requiredEnv)
    collectTemplateVars(testCase.headers, requiredEnv)
    collectTemplateVars(testCase.body, requiredEnv)

    const missing = [...requiredEnv].filter((envName) => {
      const value = env[envName]
      if (value === undefined || value === null) return true
      return String(value).trim().length === 0
    })

    if (missing.length > 0) {
      if (testCase.skipIfEnvMissing) {
        skippedCases.push({
          id: testCase.id,
          method: testCase.method,
          path: testCase.path,
          missingEnv: missing,
        })
        continue
      }
      throw new Error(`${testCase.id}: missing required env vars: ${missing.join(', ')}`)
    }

    activeCases.push({
      ...testCase,
      path: resolveTemplates(testCase.path, env),
      headers: resolveTemplates(testCase.headers, env),
      body: resolveTemplates(testCase.body, env),
    })
  }

  return { activeCases, skippedCases }
}

function buildHeaders(testCase, token) {
  const headers = {
    Accept: 'application/json',
    ...testCase.headers,
  }

  if (testCase.requireAuth) {
    assert(typeof token === 'string' && token.trim().length > 0, `${testCase.id}: token is required for auth case`)
    headers.Authorization = `Bearer ${token}`
  }

  headers['Content-Type'] = 'application/json'
  return headers
}

function safeJsonParse(raw) {
  try {
    return { ok: true, value: JSON.parse(raw) }
  } catch {
    return { ok: false, value: null }
  }
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

function toTaskUrl(baseUrl, taskId) {
  return new URL(`/api/tasks/${encodeURIComponent(taskId)}?includeEvents=1&eventsLimit=200`, baseUrl).toString()
}

async function submitCase({ baseUrl, token, testCase }) {
  const targetUrl = new URL(testCase.path, baseUrl).toString()
  const headers = buildHeaders(testCase, token)

  const response = await fetch(targetUrl, {
    method: testCase.method,
    headers,
    body: JSON.stringify(testCase.body),
  })

  const rawBody = await response.text()
  const parsed = safeJsonParse(rawBody)
  const json = parsed.ok ? parsed.value : null

  return {
    url: targetUrl,
    status: response.status,
    contentType: response.headers.get('content-type') || '',
    rawBody,
    json,
    parseError: parsed.ok ? null : 'submit response is not valid JSON',
  }
}

async function fetchTask({ baseUrl, token, taskId }) {
  const url = toTaskUrl(baseUrl, taskId)
  const response = await fetch(url, {
    method: 'GET',
    headers: {
      Accept: 'application/json',
      Authorization: `Bearer ${token}`,
    },
  })

  const rawBody = await response.text()
  const parsed = safeJsonParse(rawBody)
  const json = parsed.ok ? parsed.value : null

  return {
    url,
    status: response.status,
    rawBody,
    json,
    parseError: parsed.ok ? null : 'task polling response is not valid JSON',
  }
}

function normalizeEvents(events) {
  if (!Array.isArray(events)) return []
  return events
    .filter((item) => item && typeof item === 'object')
    .map((item) => ({
      id: item.id,
      // Rust task API returns snake_case while legacy returns camelCase.
      eventType: item.eventType ?? item.event_type,
      payload: item.payload,
      createdAt: item.createdAt ?? item.created_at,
    }))
}

function ensureResultKeys(task, expectResultKeys) {
  if (expectResultKeys.length === 0) {
    return null
  }
  const result = task?.result
  if (result === null || typeof result !== 'object' || Array.isArray(result)) {
    return `task.result is not a JSON object`
  }
  const missing = expectResultKeys.filter((key) => !(key in result))
  if (missing.length > 0) {
    return `task.result missing keys: ${missing.join(', ')}`
  }
  return null
}

function evaluateTerminal({ testCase, submitStatus, submitUrl, submitBody, taskPayload, elapsedMs, polls }) {
  const failures = []

  if (submitStatus !== testCase.expectedSubmitStatus) {
    failures.push(`submit status ${submitStatus} != expected ${testCase.expectedSubmitStatus}`)
  }

  if (!taskPayload || typeof taskPayload !== 'object') {
    failures.push('task payload is missing or invalid')
  }

  const status = typeof taskPayload?.status === 'string' ? taskPayload.status : 'unknown'
  if (!testCase.expectTerminalStatus.includes(status)) {
    failures.push(`terminal status ${status} not in expected [${testCase.expectTerminalStatus.join(', ')}]`)
  }

  const events = normalizeEvents(taskPayload?.events)
  const eventTypes = events
    .map((item) => item.eventType)
    .filter((item) => typeof item === 'string')

  if (!eventTypes.includes('task.processing')) {
    failures.push('task.processing event not found')
  }

  if (status === 'completed' && !eventTypes.includes('task.completed')) {
    failures.push('task.completed event not found')
  }
  if (status === 'failed' && !eventTypes.includes('task.failed')) {
    failures.push('task.failed event not found')
  }

  if (status === 'completed') {
    const resultError = ensureResultKeys(taskPayload, testCase.expectResultKeys)
    if (resultError) failures.push(resultError)
  }

  if (status === 'failed' && testCase.expectErrorCode) {
    const errorCode = typeof taskPayload?.errorCode === 'string' ? taskPayload.errorCode : ''
    if (errorCode !== testCase.expectErrorCode) {
      failures.push(`task.errorCode ${errorCode || 'null'} != expected ${testCase.expectErrorCode}`)
    }
  }

  return {
    id: testCase.id,
    method: testCase.method,
    path: testCase.path,
    submitStatus,
    submitUrl,
    submitBody,
    taskId: taskPayload?.id ?? null,
    finalStatus: status,
    errorCode: taskPayload?.errorCode ?? null,
    elapsedMs,
    polls,
    eventTypes,
    status: failures.length === 0 ? 'PASS' : 'FAIL',
    failures,
  }
}

async function runCase({ baseUrl, token, testCase }) {
  const startedAt = Date.now()

  let submit
  try {
    submit = await submitCase({ baseUrl, token, testCase })
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    return {
      id: testCase.id,
      method: testCase.method,
      path: testCase.path,
      submitStatus: null,
      submitUrl: new URL(testCase.path, baseUrl).toString(),
      submitBody: null,
      taskId: null,
      finalStatus: 'unknown',
      errorCode: null,
      elapsedMs: Date.now() - startedAt,
      polls: 0,
      eventTypes: [],
      status: 'FAIL',
      failures: [`submit request failed: ${message}`],
    }
  }

  const failures = []
  if (submit.parseError) {
    failures.push(submit.parseError)
  }

  const taskId = submit.json?.taskId
  if (typeof taskId !== 'string' || taskId.trim().length === 0) {
    failures.push('submit response missing taskId')
  }

  if (submit.status !== testCase.expectedSubmitStatus) {
    failures.push(`submit status ${submit.status} != expected ${testCase.expectedSubmitStatus}`)
  }

  if (failures.length > 0) {
    return {
      id: testCase.id,
      method: testCase.method,
      path: testCase.path,
      submitStatus: submit.status,
      submitUrl: submit.url,
      submitBody: submit.json,
      taskId: typeof taskId === 'string' ? taskId : null,
      finalStatus: 'unknown',
      errorCode: null,
      elapsedMs: Date.now() - startedAt,
      polls: 0,
      eventTypes: [],
      status: 'FAIL',
      failures,
    }
  }

  const deadline = Date.now() + testCase.timeoutMs
  let polls = 0

  while (Date.now() < deadline) {
    polls += 1

    let taskResponse
    try {
      taskResponse = await fetchTask({ baseUrl, token, taskId })
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      return {
        id: testCase.id,
        method: testCase.method,
        path: testCase.path,
        submitStatus: submit.status,
        submitUrl: submit.url,
        submitBody: submit.json,
        taskId,
        finalStatus: 'unknown',
        errorCode: null,
        elapsedMs: Date.now() - startedAt,
        polls,
        eventTypes: [],
        status: 'FAIL',
        failures: [`task polling request failed: ${message}`],
      }
    }

    if (taskResponse.status !== 200) {
      return {
        id: testCase.id,
        method: testCase.method,
        path: testCase.path,
        submitStatus: submit.status,
        submitUrl: submit.url,
        submitBody: submit.json,
        taskId,
        finalStatus: 'unknown',
        errorCode: null,
        elapsedMs: Date.now() - startedAt,
        polls,
        eventTypes: [],
        status: 'FAIL',
        failures: [`task polling returned non-200 status: ${taskResponse.status}`],
      }
    }

    if (taskResponse.parseError) {
      return {
        id: testCase.id,
        method: testCase.method,
        path: testCase.path,
        submitStatus: submit.status,
        submitUrl: submit.url,
        submitBody: submit.json,
        taskId,
        finalStatus: 'unknown',
        errorCode: null,
        elapsedMs: Date.now() - startedAt,
        polls,
        eventTypes: [],
        status: 'FAIL',
        failures: [taskResponse.parseError],
      }
    }

    const taskPayload = taskResponse.json?.task
    const status = typeof taskPayload?.status === 'string' ? taskPayload.status : ''

    if (TERMINAL_STATUSES.has(status)) {
      const elapsedMs = Date.now() - startedAt
      return evaluateTerminal({
        testCase,
        submitStatus: submit.status,
        submitUrl: submit.url,
        submitBody: submit.json,
        taskPayload: {
          ...taskPayload,
          events: taskResponse.json?.events,
        },
        elapsedMs,
        polls,
      })
    }

    await sleep(testCase.pollIntervalMs)
  }

  return {
    id: testCase.id,
    method: testCase.method,
    path: testCase.path,
    submitStatus: submit.status,
    submitUrl: submit.url,
    submitBody: submit.json,
    taskId,
    finalStatus: 'timeout',
    errorCode: null,
    elapsedMs: Date.now() - startedAt,
    polls,
    eventTypes: [],
    status: 'FAIL',
    failures: [`task did not reach terminal state within ${testCase.timeoutMs}ms`],
  }
}

function renderMarkdownReport(summary, rows, skippedCases) {
  const lines = []
  lines.push('# Worker Runtime Smoke Report')
  lines.push('')
  lines.push(`- Generated at: ${new Date().toISOString()}`)
  lines.push(`- Total: ${summary.total}`)
  lines.push(`- Executed: ${summary.executed}`)
  lines.push(`- Skipped: ${summary.skipped}`)
  lines.push(`- Pass: ${summary.pass}`)
  lines.push(`- Fail: ${summary.fail}`)
  lines.push('')
  lines.push('| Case | Result | Submit Status | Final Status | Task ID | Elapsed(ms) | Polls | Error Code |')
  lines.push('|---|---|---|---|---|---|---|---|')

  for (const row of rows) {
    lines.push(`| ${row.id} | ${row.status} | ${row.submitStatus} | ${row.finalStatus} | ${row.taskId || ''} | ${row.elapsedMs} | ${row.polls} | ${row.errorCode || ''} |`)
  }

  lines.push('')
  lines.push('## Skipped Cases')
  lines.push('')
  if (skippedCases.length === 0) {
    lines.push('- none')
  } else {
    for (const skipped of skippedCases) {
      lines.push(`- ${skipped.id} (${skipped.method} ${skipped.path}) missing env: ${skipped.missingEnv.join(', ')}`)
    }
  }

  lines.push('')
  lines.push('## Failures')
  lines.push('')
  const failed = rows.filter((row) => row.status === 'FAIL')
  if (failed.length === 0) {
    lines.push('- none')
  } else {
    for (const row of failed) {
      lines.push(`- ${row.id} (${row.method} ${row.path})`)
      for (const failure of row.failures) {
        lines.push(`  - ${failure}`)
      }
    }
  }

  lines.push('')
  return `${lines.join('\n')}\n`
}

function writeReports(outDir, rows, skippedCases) {
  fs.mkdirSync(outDir, { recursive: true })

  const summary = {
    total: rows.length + skippedCases.length,
    executed: rows.length,
    skipped: skippedCases.length,
    pass: rows.filter((row) => row.status === 'PASS').length,
    fail: rows.filter((row) => row.status === 'FAIL').length,
  }

  const jsonPath = path.join(outDir, 'worker-runtime-smoke.json')
  const mdPath = path.join(outDir, 'worker-runtime-smoke.md')

  fs.writeFileSync(
    jsonPath,
    JSON.stringify(
      {
        generatedAt: new Date().toISOString(),
        summary,
        rows,
        skippedCases,
      },
      null,
      2,
    ),
    'utf8',
  )

  fs.writeFileSync(mdPath, renderMarkdownReport(summary, rows, skippedCases), 'utf8')

  return { summary, jsonPath, mdPath }
}

async function main() {
  const args = parseArgs(process.argv.slice(2))
  const baseUrl = normalizeBaseUrl(args.base || args['rust-base'], 'base')
  const token = args.token || process.env.WW_TOKEN
  const casesPath = path.resolve(args.cases || DEFAULT_CASES_PATH)
  const outDir = path.resolve(args['out-dir'] || DEFAULT_OUT_DIR)

  const allCases = loadCases(casesPath)
  const { activeCases, skippedCases } = prepareCases(allCases, process.env)

  if (args.dryRun) {
    console.log(`[dry-run] cases file: ${casesPath}`)
    console.log(`[dry-run] loaded ${allCases.length} cases`)
    console.log(`[dry-run] active ${activeCases.length} skipped ${skippedCases.length}`)
    for (const testCase of activeCases) {
      console.log(`- ${testCase.id}: ${testCase.method} ${testCase.path}`)
    }
    for (const skipped of skippedCases) {
      console.log(`- [skip] ${skipped.id}: missing env ${skipped.missingEnv.join(', ')}`)
    }
    return
  }

  assert(typeof token === 'string' && token.trim().length > 0, 'token is required (use --token or WW_TOKEN)')

  if (activeCases.length === 0) {
    console.log('no runnable worker cases after env resolution (all skipped)')
    return
  }

  const concurrency = parseOptionalInt(args.concurrency, 'concurrency') ?? 1
  const rows = []

  for (let index = 0; index < activeCases.length; index += concurrency) {
    const batch = activeCases.slice(index, index + concurrency)
    const batchRows = await Promise.all(
      batch.map(async (testCase) => runCase({ baseUrl, token, testCase })),
    )
    rows.push(...batchRows)
  }

  const { summary, jsonPath, mdPath } = writeReports(outDir, rows, skippedCases)
  console.log(`wrote ${jsonPath}`)
  console.log(`wrote ${mdPath}`)
  console.log(`PASS=${summary.pass} FAIL=${summary.fail} SKIPPED=${summary.skipped} TOTAL=${summary.total}`)

  if (summary.fail > 0) {
    process.exitCode = 2
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error))
  process.exitCode = 1
})
