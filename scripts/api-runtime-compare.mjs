#!/usr/bin/env node

import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'

const VALID_METHODS = new Set(['GET', 'POST', 'PUT', 'PATCH', 'DELETE'])
const ALLOWED_CASE_KEYS = new Set([
  'id',
  'method',
  'path',
  'expectedStatus',
  'headers',
  'body',
  'requireAuth',
  'compare',
  'timeoutMs',
  'requiresEnv',
  'skipIfEnvMissing',
])
const ALLOWED_COMPARE_KEYS = new Set(['strictStatus', 'topLevelKeys'])
const DEFAULT_TIMEOUT_MS = 15000
const TEMPLATE_RE = /\{\{([A-Z0-9_]+)\}\}/g

const SCRIPT_DIR = path.dirname(new URL(import.meta.url).pathname)
const PROJECT_ROOT = path.resolve(SCRIPT_DIR, '..')
const DEFAULT_CASES_PATH = path.join(SCRIPT_DIR, 'api-runtime-cases.sample.json')
const DEFAULT_OUT_DIR = path.join(PROJECT_ROOT, 'reports')

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
  const url = raw.trim()
  assert(/^https?:\/\//.test(url), `${name} must start with http:// or https://`)
  return url.endsWith('/') ? url : `${url}/`
}

function parseOptionalInt(raw, name) {
  if (raw === undefined) {
    return undefined
  }
  const parsed = Number(raw)
  assert(Number.isInteger(parsed) && parsed > 0, `${name} must be a positive integer`)
  return parsed
}

function validateRequiresEnv(requiresEnv, caseId) {
  assert(Array.isArray(requiresEnv), `${caseId}: requiresEnv must be an array`)
  for (const envName of requiresEnv) {
    assert(typeof envName === 'string' && envName.trim().length > 0, `${caseId}: requiresEnv values must be non-empty strings`)
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
    const next = {}
    for (const [key, item] of Object.entries(value)) {
      next[key] = resolveTemplates(item, env)
    }
    return next
  }
  return value
}

function validateHeaders(headers, caseId) {
  assert(headers !== null && typeof headers === 'object' && !Array.isArray(headers), `${caseId}: headers must be an object`)
  for (const [key, value] of Object.entries(headers)) {
    assert(typeof key === 'string' && key.trim().length > 0, `${caseId}: header key must be non-empty string`)
    assert(typeof value === 'string', `${caseId}: header ${key} must be string`) 
  }
}

function validateCompare(compare, caseId) {
  assert(compare !== null && typeof compare === 'object' && !Array.isArray(compare), `${caseId}: compare must be an object`)
  for (const key of Object.keys(compare)) {
    assert(ALLOWED_COMPARE_KEYS.has(key), `${caseId}: unsupported compare key ${key}`)
  }
  if (compare.strictStatus !== undefined) {
    assert(typeof compare.strictStatus === 'boolean', `${caseId}: compare.strictStatus must be boolean`)
  }
  if (compare.topLevelKeys !== undefined) {
    assert(typeof compare.topLevelKeys === 'boolean', `${caseId}: compare.topLevelKeys must be boolean`)
  }
}

function validateCaseEntry(entry, index) {
  const caseId = `case[${index}]`
  assert(entry !== null && typeof entry === 'object' && !Array.isArray(entry), `${caseId}: must be an object`)

  for (const key of Object.keys(entry)) {
    assert(ALLOWED_CASE_KEYS.has(key), `${caseId}: unsupported key ${key}`)
  }

  assert(typeof entry.id === 'string' && entry.id.trim().length > 0, `${caseId}: id is required`) 
  assert(typeof entry.method === 'string', `${entry.id}: method is required`)
  const method = entry.method.trim().toUpperCase()
  assert(VALID_METHODS.has(method), `${entry.id}: unsupported method ${entry.method}`)
  assert(typeof entry.path === 'string' && entry.path.startsWith('/'), `${entry.id}: path must start with /`)

  if (entry.expectedStatus !== undefined) {
    assert(Number.isInteger(entry.expectedStatus), `${entry.id}: expectedStatus must be integer`)
    assert(entry.expectedStatus >= 100 && entry.expectedStatus <= 599, `${entry.id}: expectedStatus must be valid HTTP status`)
  }

  if (entry.headers !== undefined) {
    validateHeaders(entry.headers, entry.id)
  }

  if (entry.requireAuth !== undefined) {
    assert(typeof entry.requireAuth === 'boolean', `${entry.id}: requireAuth must be boolean`)
  }

  if (entry.compare !== undefined) {
    validateCompare(entry.compare, entry.id)
  }

  if (entry.timeoutMs !== undefined) {
    assert(Number.isInteger(entry.timeoutMs) && entry.timeoutMs > 0, `${entry.id}: timeoutMs must be a positive integer`)
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
    expectedStatus: entry.expectedStatus,
    headers: entry.headers ?? {},
    body: entry.body,
    requireAuth: entry.requireAuth ?? false,
    compare: {
      strictStatus: entry.compare?.strictStatus ?? true,
      topLevelKeys: entry.compare?.topLevelKeys ?? true,
    },
    timeoutMs: entry.timeoutMs ?? DEFAULT_TIMEOUT_MS,
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
      if (value === undefined || value === null) {
        return true
      }
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

  if (testCase.body !== undefined) {
    headers['Content-Type'] = 'application/json'
  }

  return headers
}

function safeJsonParse(raw) {
  try {
    return { ok: true, value: JSON.parse(raw) }
  } catch {
    return { ok: false, value: null }
  }
}

function collectTopLevelKeys(value) {
  if (value === null || Array.isArray(value) || typeof value !== 'object') {
    return null
  }
  return Object.keys(value).sort()
}

async function runSideRequest({ sideName, baseUrl, testCase, token }) {
  const targetUrl = new URL(testCase.path, baseUrl).toString()
  const headers = buildHeaders(testCase, token)
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), testCase.timeoutMs)

  try {
    const response = await fetch(targetUrl, {
      method: testCase.method,
      headers,
      body: testCase.body !== undefined ? JSON.stringify(testCase.body) : undefined,
      signal: controller.signal,
    })

    const contentType = response.headers.get('content-type') || ''
    const rawBody = await response.text()

    let parseError = null
    let parsedJson = null
    if (rawBody.length > 0) {
      const parsed = safeJsonParse(rawBody)
      if (parsed.ok) {
        parsedJson = parsed.value
      } else if (contentType.includes('application/json')) {
        parseError = 'response declared JSON but body is not valid JSON'
      }
    }

    return {
      sideName,
      url: targetUrl,
      status: response.status,
      contentType,
      parseError,
      rawBody,
      json: parsedJson,
      topLevelKeys: collectTopLevelKeys(parsedJson),
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    return {
      sideName,
      url: targetUrl,
      status: null,
      contentType: '',
      parseError: null,
      rawBody: '',
      json: null,
      topLevelKeys: null,
      requestError: message,
    }
  } finally {
    clearTimeout(timer)
  }
}

function diffKeys(reference, candidate) {
  const referenceSet = new Set(reference)
  const candidateSet = new Set(candidate)
  return {
    missingInRust: reference.filter((key) => !candidateSet.has(key)),
    extraInRust: candidate.filter((key) => !referenceSet.has(key)),
  }
}

function evaluateCase(testCase, legacyResult, rustResult) {
  const failures = []

  if (legacyResult.requestError) {
    failures.push(`legacy request failed: ${legacyResult.requestError}`)
  }
  if (rustResult.requestError) {
    failures.push(`rust request failed: ${rustResult.requestError}`)
  }
  if (legacyResult.parseError) {
    failures.push(`legacy parse failed: ${legacyResult.parseError}`)
  }
  if (rustResult.parseError) {
    failures.push(`rust parse failed: ${rustResult.parseError}`)
  }

  if (testCase.expectedStatus !== undefined) {
    if (legacyResult.status !== testCase.expectedStatus) {
      failures.push(`legacy status ${legacyResult.status} != expected ${testCase.expectedStatus}`)
    }
    if (rustResult.status !== testCase.expectedStatus) {
      failures.push(`rust status ${rustResult.status} != expected ${testCase.expectedStatus}`)
    }
  }

  if (testCase.compare.strictStatus && legacyResult.status !== rustResult.status) {
    failures.push(`status mismatch legacy=${legacyResult.status} rust=${rustResult.status}`)
  }

  const keysDiff = {
    missingInRust: [],
    extraInRust: [],
  }

  if (testCase.compare.topLevelKeys) {
    if (!legacyResult.topLevelKeys) {
      failures.push('legacy response is not a JSON object')
    }
    if (!rustResult.topLevelKeys) {
      failures.push('rust response is not a JSON object')
    }

    if (legacyResult.topLevelKeys && rustResult.topLevelKeys) {
      const diff = diffKeys(legacyResult.topLevelKeys, rustResult.topLevelKeys)
      keysDiff.missingInRust = diff.missingInRust
      keysDiff.extraInRust = diff.extraInRust
      if (diff.missingInRust.length > 0 || diff.extraInRust.length > 0) {
        failures.push(
          `top-level key mismatch missing=[${diff.missingInRust.join(', ')}] extra=[${diff.extraInRust.join(', ')}]`,
        )
      }
    }
  }

  return {
    id: testCase.id,
    method: testCase.method,
    path: testCase.path,
    status: failures.length === 0 ? 'PASS' : 'FAIL',
    failures,
    compare: testCase.compare,
    expectedStatus: testCase.expectedStatus ?? null,
    legacy: {
      url: legacyResult.url,
      status: legacyResult.status,
      topLevelKeys: legacyResult.topLevelKeys,
    },
    rust: {
      url: rustResult.url,
      status: rustResult.status,
      topLevelKeys: rustResult.topLevelKeys,
      missingInRust: keysDiff.missingInRust,
      extraInRust: keysDiff.extraInRust,
    },
  }
}

function renderMarkdownReport(summary, rows, skippedCases) {
  const lines = []
  lines.push('# API Runtime Compare Report')
  lines.push('')
  lines.push(`- Generated at: ${new Date().toISOString()}`)
  lines.push(`- Total: ${summary.total}`)
  lines.push(`- Executed: ${summary.executed}`)
  lines.push(`- Skipped: ${summary.skipped}`)
  lines.push(`- Pass: ${summary.pass}`)
  lines.push(`- Fail: ${summary.fail}`)
  lines.push('')
  lines.push('| Case | Method | Path | Result | Legacy Status | Rust Status | Missing In Rust | Extra In Rust |')
  lines.push('|---|---|---|---|---|---|---|---|')

  for (const row of rows) {
    lines.push(
      `| ${row.id} | ${row.method} | ${row.path} | ${row.status} | ${row.legacy.status} | ${row.rust.status} | ${row.rust.missingInRust.join(', ')} | ${row.rust.extraInRust.join(', ')} |`,
    )
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

  const jsonPath = path.join(outDir, 'api-runtime-compare.json')
  const mdPath = path.join(outDir, 'api-runtime-compare.md')
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
  const casesPath = path.resolve(args.cases || DEFAULT_CASES_PATH)
  const outDir = path.resolve(args['out-dir'] || DEFAULT_OUT_DIR)
  const token = args.token
  const cases = loadCases(casesPath)
  const { activeCases, skippedCases } = prepareCases(cases, process.env)

  if (args.dryRun) {
    console.log(`[dry-run] cases file: ${casesPath}`)
    console.log(`[dry-run] loaded ${cases.length} cases`)
    console.log(`[dry-run] active ${activeCases.length} skipped ${skippedCases.length}`)
    for (const testCase of activeCases) {
      console.log(`- ${testCase.id}: ${testCase.method} ${testCase.path}`)
    }
    for (const skipped of skippedCases) {
      console.log(`- [skip] ${skipped.id}: missing env ${skipped.missingEnv.join(', ')}`)
    }
    return
  }

  if (activeCases.length === 0) {
    console.log('no runnable cases after env resolution (all skipped)')
    return
  }

  const legacyBase = normalizeBaseUrl(args['legacy-base'], 'legacy-base')
  const rustBase = normalizeBaseUrl(args['rust-base'], 'rust-base')
  const concurrency = parseOptionalInt(args.concurrency, 'concurrency') ?? 4
  const rows = []

  for (let index = 0; index < activeCases.length; index += concurrency) {
    const batch = activeCases.slice(index, index + concurrency)
    const batchRows = await Promise.all(
      batch.map(async (testCase) => {
        const [legacyResult, rustResult] = await Promise.all([
          runSideRequest({ sideName: 'legacy', baseUrl: legacyBase, testCase, token }),
          runSideRequest({ sideName: 'rust', baseUrl: rustBase, testCase, token }),
        ])
        return evaluateCase(testCase, legacyResult, rustResult)
      }),
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
