#!/usr/bin/env node

import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { performance } from 'node:perf_hooks'

const VALID_METHODS = new Set(['GET', 'POST', 'PUT', 'PATCH', 'DELETE'])
const TEMPLATE_RE = /\{\{([A-Z0-9_]+)\}\}/g

const ALLOWED_CASE_KEYS = new Set([
  'id',
  'method',
  'path',
  'headers',
  'body',
  'requireAuth',
  'requiresEnv',
  'skipIfEnvMissing',
])

const SCRIPT_DIR = path.dirname(new URL(import.meta.url).pathname)
const PROJECT_ROOT = path.resolve(SCRIPT_DIR, '..')
const DEFAULT_CASES_PATH = path.join(SCRIPT_DIR, 'api-performance-cases.sample.json')
const DEFAULT_OUT_DIR = path.join(PROJECT_ROOT, 'reports')

const DEFAULT_DURATION_MS = 15000
const DEFAULT_CONCURRENCY = 12
const DEFAULT_REQUEST_TIMEOUT_MS = 10000
const DEFAULT_WARMUP_REQUESTS = 3

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

function parseOptionalInt(raw, name, fallback) {
  if (raw === undefined) {
    return fallback
  }
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

function validateStringArray(values, caseId, keyName) {
  assert(Array.isArray(values), `${caseId}: ${keyName} must be an array`)
  for (const value of values) {
    assert(typeof value === 'string' && value.trim().length > 0, `${caseId}: ${keyName} values must be non-empty strings`)
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

  const method = (entry.method || 'GET').trim().toUpperCase()
  assert(VALID_METHODS.has(method), `${entry.id}: unsupported method ${method}`)

  if (entry.headers !== undefined) {
    validateHeaders(entry.headers, entry.id)
  }

  if (entry.requireAuth !== undefined) {
    assert(typeof entry.requireAuth === 'boolean', `${entry.id}: requireAuth must be boolean`)
  }

  if (entry.requiresEnv !== undefined) {
    validateRequiresEnv(entry.requiresEnv, entry.id)
  }

  if (entry.skipIfEnvMissing !== undefined) {
    assert(typeof entry.skipIfEnvMissing === 'boolean', `${entry.id}: skipIfEnvMissing must be boolean`)
  }

  if (entry.body !== undefined) {
    assert(method !== 'GET' && method !== 'DELETE', `${entry.id}: GET/DELETE cannot define request body`)
  }

  return {
    id: entry.id,
    method,
    path: entry.path,
    headers: entry.headers ?? {},
    body: entry.body,
    requireAuth: entry.requireAuth ?? false,
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

function prepareCases(cases, env, token) {
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
          reason: `missing env: ${missing.join(', ')}`,
        })
        continue
      }
      throw new Error(`${testCase.id}: missing required env vars: ${missing.join(', ')}`)
    }

    if (testCase.requireAuth && (!token || token.trim().length === 0)) {
      skippedCases.push({
        id: testCase.id,
        method: testCase.method,
        path: testCase.path,
        reason: 'missing token for auth-required case',
      })
      continue
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

function buildRequestInit({ testCase, token, requestTimeoutMs }) {
  const headers = {
    Accept: 'application/json',
    ...testCase.headers,
  }
  if (testCase.requireAuth) {
    headers.Authorization = `Bearer ${token}`
  }
  if (testCase.body !== undefined) {
    headers['Content-Type'] = 'application/json'
  }

  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), requestTimeoutMs)

  return {
    init: {
      method: testCase.method,
      headers,
      body: testCase.body !== undefined ? JSON.stringify(testCase.body) : undefined,
      signal: controller.signal,
    },
    done: () => clearTimeout(timer),
  }
}

async function runRequest({ baseUrl, token, testCase, requestTimeoutMs }) {
  const targetUrl = new URL(testCase.path, baseUrl).toString()
  const started = performance.now()
  const { init, done } = buildRequestInit({ testCase, token, requestTimeoutMs })

  try {
    const response = await fetch(targetUrl, init)
    await response.arrayBuffer()
    return {
      ok: response.ok,
      status: response.status,
      durationMs: performance.now() - started,
      error: response.ok ? null : `HTTP ${response.status}`,
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    return {
      ok: false,
      status: null,
      durationMs: performance.now() - started,
      error: message,
    }
  } finally {
    done()
  }
}

function percentile(sorted, p) {
  if (sorted.length === 0) return null
  const rank = Math.ceil((p / 100) * sorted.length)
  const index = Math.min(sorted.length - 1, Math.max(0, rank - 1))
  return sorted[index]
}

function summarizeLatency(samples) {
  if (samples.length === 0) {
    return {
      min: null,
      avg: null,
      p50: null,
      p95: null,
      p99: null,
      max: null,
    }
  }

  const sorted = [...samples].sort((a, b) => a - b)
  const sum = sorted.reduce((acc, value) => acc + value, 0)

  return {
    min: Number(sorted[0].toFixed(2)),
    avg: Number((sum / sorted.length).toFixed(2)),
    p50: Number(percentile(sorted, 50).toFixed(2)),
    p95: Number(percentile(sorted, 95).toFixed(2)),
    p99: Number(percentile(sorted, 99).toFixed(2)),
    max: Number(sorted[sorted.length - 1].toFixed(2)),
  }
}

function toPercent(part, total) {
  if (!total) return 0
  return Number(((part / total) * 100).toFixed(2))
}

function deltaPercent(candidate, baseline) {
  if (baseline === null || baseline === undefined || baseline === 0) {
    return null
  }
  return Number((((candidate - baseline) / baseline) * 100).toFixed(2))
}

async function runSideBenchmark({ sideName, baseUrl, token, testCase, durationMs, concurrency, requestTimeoutMs }) {
  const stopAt = Date.now() + durationMs
  const durations = []
  const statusCounts = new Map()
  const errorCounts = new Map()

  let totalRequests = 0
  let successRequests = 0
  let failedRequests = 0

  async function workerLoop() {
    while (Date.now() < stopAt) {
      const result = await runRequest({ baseUrl, token, testCase, requestTimeoutMs })
      totalRequests += 1
      durations.push(result.durationMs)

      if (result.status !== null) {
        const key = String(result.status)
        statusCounts.set(key, (statusCounts.get(key) || 0) + 1)
      }

      if (result.ok) {
        successRequests += 1
      } else {
        failedRequests += 1
        const errorKey = result.error || 'unknown error'
        errorCounts.set(errorKey, (errorCounts.get(errorKey) || 0) + 1)
      }
    }
  }

  await Promise.all(
    Array.from({ length: concurrency }, () => workerLoop()),
  )

  const elapsedMs = Math.max(1, durationMs)
  const rps = Number((totalRequests / (elapsedMs / 1000)).toFixed(2))

  return {
    side: sideName,
    totalRequests,
    successRequests,
    failedRequests,
    successRate: toPercent(successRequests, totalRequests),
    rps,
    latency: summarizeLatency(durations),
    statusCounts: Object.fromEntries(statusCounts.entries()),
    errorCounts: Object.fromEntries(errorCounts.entries()),
  }
}

async function warmupSide({ baseUrl, token, testCase, requestTimeoutMs, warmupRequests }) {
  for (let i = 0; i < warmupRequests; i += 1) {
    await runRequest({ baseUrl, token, testCase, requestTimeoutMs })
  }
}

function evaluateCaseResult(testCase, legacy, rust) {
  const failures = []
  if (legacy.totalRequests === 0) {
    failures.push('legacy side produced zero requests')
  }
  if (rust.totalRequests === 0) {
    failures.push('rust side produced zero requests')
  }
  if (legacy.successRequests === 0) {
    failures.push('legacy side has zero successful responses')
  }
  if (rust.successRequests === 0) {
    failures.push('rust side has zero successful responses')
  }

  return {
    id: testCase.id,
    method: testCase.method,
    path: testCase.path,
    status: failures.length === 0 ? 'PASS' : 'FAIL',
    failures,
    legacy,
    rust,
    deltas: {
      rpsPercent: deltaPercent(rust.rps, legacy.rps),
      p95Percent: deltaPercent(rust.latency.p95, legacy.latency.p95),
      p99Percent: deltaPercent(rust.latency.p99, legacy.latency.p99),
      successRatePercent: deltaPercent(rust.successRate, legacy.successRate),
    },
  }
}

function renderMarkdown(summary, rows, skippedCases, options) {
  const lines = []
  lines.push('# API Performance Benchmark Report')
  lines.push('')
  lines.push(`- Generated at: ${new Date().toISOString()}`)
  lines.push(`- Duration per case: ${options.durationMs}ms`)
  lines.push(`- Concurrency: ${options.concurrency}`)
  lines.push(`- Request timeout: ${options.requestTimeoutMs}ms`)
  lines.push(`- Warmup requests per side: ${options.warmupRequests}`)
  lines.push(`- Total: ${summary.total} | Executed: ${summary.executed} | Skipped: ${summary.skipped} | Pass: ${summary.pass} | Fail: ${summary.fail}`)
  lines.push('')
  lines.push('| Case | Result | Legacy RPS | Rust RPS | RPS Δ% | Legacy P95(ms) | Rust P95(ms) | P95 Δ% | Legacy Success% | Rust Success% |')
  lines.push('|---|---|---|---|---|---|---|---|---|---|')

  for (const row of rows) {
    lines.push(`| ${row.id} | ${row.status} | ${row.legacy.rps} | ${row.rust.rps} | ${row.deltas.rpsPercent ?? ''} | ${row.legacy.latency.p95 ?? ''} | ${row.rust.latency.p95 ?? ''} | ${row.deltas.p95Percent ?? ''} | ${row.legacy.successRate} | ${row.rust.successRate} |`)
  }

  lines.push('')
  lines.push('## Skipped Cases')
  lines.push('')
  if (skippedCases.length === 0) {
    lines.push('- none')
  } else {
    for (const skipped of skippedCases) {
      lines.push(`- ${skipped.id} (${skipped.method} ${skipped.path}) reason: ${skipped.reason}`)
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

function writeReports(outDir, rows, skippedCases, options) {
  fs.mkdirSync(outDir, { recursive: true })

  const summary = {
    total: rows.length + skippedCases.length,
    executed: rows.length,
    skipped: skippedCases.length,
    pass: rows.filter((row) => row.status === 'PASS').length,
    fail: rows.filter((row) => row.status === 'FAIL').length,
  }

  const jsonPath = path.join(outDir, 'api-performance-benchmark.json')
  const mdPath = path.join(outDir, 'api-performance-benchmark.md')

  fs.writeFileSync(
    jsonPath,
    JSON.stringify(
      {
        generatedAt: new Date().toISOString(),
        summary,
        options,
        rows,
        skippedCases,
      },
      null,
      2,
    ),
    'utf8',
  )

  fs.writeFileSync(mdPath, renderMarkdown(summary, rows, skippedCases, options), 'utf8')
  return { summary, jsonPath, mdPath }
}

async function main() {
  const args = parseArgs(process.argv.slice(2))
  const legacyBase = normalizeBaseUrl(args['legacy-base'], 'legacy-base')
  const rustBase = normalizeBaseUrl(args['rust-base'], 'rust-base')
  const token = args.token || process.env.WW_TOKEN || ''
  const casesPath = path.resolve(args.cases || DEFAULT_CASES_PATH)
  const outDir = path.resolve(args['out-dir'] || DEFAULT_OUT_DIR)

  const options = {
    durationMs: parseOptionalInt(args['duration-ms'], 'duration-ms', DEFAULT_DURATION_MS),
    concurrency: parseOptionalInt(args.concurrency, 'concurrency', DEFAULT_CONCURRENCY),
    requestTimeoutMs: parseOptionalInt(args['request-timeout-ms'], 'request-timeout-ms', DEFAULT_REQUEST_TIMEOUT_MS),
    warmupRequests: parseOptionalInt(args['warmup-requests'], 'warmup-requests', DEFAULT_WARMUP_REQUESTS),
  }

  const cases = loadCases(casesPath)
  const { activeCases, skippedCases } = prepareCases(cases, process.env, token)

  if (args.dryRun) {
    console.log(`[dry-run] cases file: ${casesPath}`)
    console.log(`[dry-run] loaded ${cases.length} cases`)
    console.log(`[dry-run] active ${activeCases.length} skipped ${skippedCases.length}`)
    for (const testCase of activeCases) {
      console.log(`- ${testCase.id}: ${testCase.method} ${testCase.path}`)
    }
    for (const skipped of skippedCases) {
      console.log(`- [skip] ${skipped.id}: ${skipped.reason}`)
    }
    return
  }

  if (activeCases.length === 0) {
    console.log('no runnable benchmark cases after env/token resolution (all skipped)')
    return
  }

  const rows = []
  for (const testCase of activeCases) {
    await Promise.all([
      warmupSide({ baseUrl: legacyBase, token, testCase, requestTimeoutMs: options.requestTimeoutMs, warmupRequests: options.warmupRequests }),
      warmupSide({ baseUrl: rustBase, token, testCase, requestTimeoutMs: options.requestTimeoutMs, warmupRequests: options.warmupRequests }),
    ])

    const [legacyResult, rustResult] = await Promise.all([
      runSideBenchmark({ sideName: 'legacy', baseUrl: legacyBase, token, testCase, durationMs: options.durationMs, concurrency: options.concurrency, requestTimeoutMs: options.requestTimeoutMs }),
      runSideBenchmark({ sideName: 'rust', baseUrl: rustBase, token, testCase, durationMs: options.durationMs, concurrency: options.concurrency, requestTimeoutMs: options.requestTimeoutMs }),
    ])

    rows.push(evaluateCaseResult(testCase, legacyResult, rustResult))
  }

  const { summary, jsonPath, mdPath } = writeReports(outDir, rows, skippedCases, options)
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
