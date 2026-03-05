#!/usr/bin/env node

import crypto from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { createRequire } from 'node:module'

const SCRIPT_DIR = path.dirname(new URL(import.meta.url).pathname)
const PROJECT_ROOT = path.resolve(SCRIPT_DIR, '..')
const FRONTEND_ROOT = path.join(PROJECT_ROOT, 'frontend')
const DEFAULT_OUT_DIR = path.join(PROJECT_ROOT, 'reports', 'frontend-e2e')
const DEFAULT_BASE_URL = 'http://127.0.0.1:80'
const DEFAULT_TIMEOUT_MS = 30000
const AUTH_TOKEN_STORAGE_KEY = 'waoowaoo.auth_token'

const PHASE_SEQUENCE = [
  'auth-flow',
  'workspace',
  'project-workbench',
  'asset-hub',
  'profile',
  'admin-non-admin',
  'i18n-redirect',
  'not-found',
]

const WORKBENCH_STAGES = [
  {
    id: 'config',
    tabLabel: /1\.\s*Config/i,
    headingLabel: /Stage:\s*Config/i,
  },
  {
    id: 'script',
    tabLabel: /2\.\s*Script Draft/i,
    headingLabel: /Stage:\s*Script/i,
  },
  {
    id: 'assets',
    tabLabel: /3\.\s*Asset Analysis/i,
    headingLabel: /Stage:\s*Assets/i,
  },
  {
    id: 'prompts',
    tabLabel: /4\.\s*Prompt Workshop/i,
    headingLabel: /Stage:\s*Prompts/i,
  },
  {
    id: 'text-storyboard',
    tabLabel: /5\.\s*Text Storyboard/i,
    headingLabel: /Stage:\s*Text Storyboard/i,
  },
  {
    id: 'storyboard',
    tabLabel: /6\.\s*Storyboard Review/i,
    headingLabel: /Stage:\s*Storyboard/i,
  },
  {
    id: 'videos',
    tabLabel: /7\.\s*Video Generation/i,
    headingLabel: /Stage:\s*Videos/i,
  },
  {
    id: 'voice',
    tabLabel: /8\.\s*Voice Stage/i,
    headingLabel: /Stage:\s*Voice/i,
  },
  {
    id: 'editor',
    tabLabel: /9\.\s*Video Editor/i,
    headingLabel: /Stage:\s*Editor/i,
  },
]

function assert(condition, message) {
  if (!condition) {
    throw new Error(message)
  }
}

function parseBoolean(raw, name) {
  const normalized = String(raw ?? '')
    .trim()
    .toLowerCase()

  if (['1', 'true', 'yes', 'on'].includes(normalized)) {
    return true
  }
  if (['0', 'false', 'no', 'off'].includes(normalized)) {
    return false
  }

  throw new Error(`${name} must be true/false`)
}

function parseArgs(argv) {
  const args = {
    clean: true,
    dryRun: false,
    headless: true,
  }

  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i]
    assert(token.startsWith('--'), `Unsupported argument: ${token}`)

    if (token === '--dry-run') {
      args.dryRun = true
      continue
    }

    if (token === '--no-clean') {
      args.clean = false
      continue
    }

    if (token === '--clean') {
      const maybeValue = argv[i + 1]
      if (maybeValue && !maybeValue.startsWith('--')) {
        args.clean = parseBoolean(maybeValue, 'clean')
        i += 1
      } else {
        args.clean = true
      }
      continue
    }

    if (token === '--no-headless') {
      args.headless = false
      continue
    }

    if (token === '--headless') {
      const maybeValue = argv[i + 1]
      if (maybeValue && !maybeValue.startsWith('--')) {
        args.headless = parseBoolean(maybeValue, 'headless')
        i += 1
      } else {
        args.headless = true
      }
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

function safeJsonParse(raw) {
  try {
    return { ok: true, value: JSON.parse(raw) }
  } catch {
    return { ok: false, value: null }
  }
}

function isObject(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
}

function markdownCell(value) {
  return String(value ?? '')
    .replace(/\|/g, '\\|')
    .replace(/\n/g, ' ')
}

function truncateText(value, maxLength = 220) {
  const normalized = String(value ?? '')
    .replace(/\s+/g, ' ')
    .trim()

  if (normalized.length <= maxLength) {
    return normalized
  }
  return `${normalized.slice(0, maxLength - 3)}...`
}

function sleep(ms) {
  return new Promise((resolve) => {
    setTimeout(resolve, ms)
  })
}

function toSlug(value) {
  return String(value ?? '')
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 72)
}

function createIdentity() {
  const ts = Date.now()
  const nonce = crypto.randomUUID().slice(0, 8)

  return {
    username: `e2e-frontend-${ts}-${nonce}@test.local`,
    password: 'Test123456',
    projectName: `E2E Frontend ${ts}`,
    assetCharacterName: `E2E Character ${ts}`,
  }
}

function loadPlaywright() {
  const rootRequire = createRequire(import.meta.url)

  try {
    return rootRequire('playwright')
  } catch {
    // fall through to frontend scoped require
  }

  try {
    const frontendRequire = createRequire(path.join(FRONTEND_ROOT, 'package.json'))
    return frontendRequire('playwright')
  } catch {
    throw new Error(
      'playwright package is not installed. Run: cd frontend && npm install --save-dev playwright && npx playwright install chromium',
    )
  }
}

async function requestJson({ baseUrl, method, endpointPath, token, body, timeoutMs }) {
  const targetUrl = new URL(endpointPath, baseUrl).toString()
  const headers = {
    Accept: 'application/json',
  }
  if (token) {
    headers.Authorization = `Bearer ${token}`
  }

  let payload
  if (body !== undefined) {
    headers['Content-Type'] = 'application/json'
    payload = JSON.stringify(body)
  }

  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), timeoutMs)

  try {
    const response = await fetch(targetUrl, {
      method,
      headers,
      body: payload,
      signal: controller.signal,
    })

    const rawBody = await response.text()
    const parsed = safeJsonParse(rawBody)

    return {
      status: response.status,
      url: targetUrl,
      rawBody,
      json: parsed.ok ? parsed.value : null,
      parseError: parsed.ok ? null : 'response is not valid JSON',
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    throw new Error(`request failed ${method} ${endpointPath}: ${message}`)
  } finally {
    clearTimeout(timer)
  }
}

function expectStatusOneOf(response, expectedStatuses, context) {
  assert(
    expectedStatuses.includes(response.status),
    `${context}: expected status ${expectedStatuses.join('/')} got ${response.status}, body=${response.rawBody}`,
  )
}

function createPhase(id) {
  return {
    id,
    status: 'PASS',
    startedAt: new Date().toISOString(),
    endedAt: null,
    durationMs: 0,
    steps: [],
    error: null,
  }
}

function summarizeDiagnostics(entries) {
  const summary = {
    consoleErrors: 0,
    consoleWarnings: 0,
    pageErrors: 0,
    requestFailed: 0,
  }

  for (const entry of entries) {
    if (entry.kind === 'console.error') {
      summary.consoleErrors += 1
    } else if (entry.kind === 'console.warning') {
      summary.consoleWarnings += 1
    } else if (entry.kind === 'pageerror') {
      summary.pageErrors += 1
    } else if (entry.kind === 'requestfailed') {
      summary.requestFailed += 1
    }
  }

  return summary
}

function isSevereDiagnostic(entry) {
  return entry.kind === 'console.error' || entry.kind === 'pageerror'
}

function formatDiagnostic(entry) {
  const phaseAndStep = `${entry.phaseId || '-phase'}/${entry.stepId || '-step'}`
  const locationParts = []
  if (entry.url) {
    locationParts.push(entry.url)
  }
  if (Number.isInteger(entry.line)) {
    locationParts.push(`L${entry.line}`)
  }
  if (Number.isInteger(entry.column)) {
    locationParts.push(`C${entry.column}`)
  }
  const location = locationParts.length > 0 ? ` @ ${locationParts.join(':')}` : ''
  return `[${phaseAndStep}] ${entry.kind}: ${truncateText(entry.text)}${location}`
}

function renderDiagnosticsLog(diagnostics) {
  if (diagnostics.length === 0) {
    return '[no diagnostics captured]\n'
  }

  const lines = []
  for (const entry of diagnostics) {
    const phaseAndStep = `${entry.phaseId || '-phase'}/${entry.stepId || '-step'}`
    const location = entry.url ? ` @ ${entry.url}` : ''
    const suffix = Number.isInteger(entry.line)
      ? `${location}:${entry.line}${Number.isInteger(entry.column) ? `:${entry.column}` : ''}`
      : location
    lines.push(
      `[${entry.ts}] [${entry.kind.toUpperCase()}] [${phaseAndStep}] ${truncateText(entry.text, 800)}${suffix}`,
    )
  }
  return `${lines.join('\n')}\n`
}

function buildSummary(report) {
  const phaseTotal = report.phases.length
  const phasePass = report.phases.filter((phase) => phase.status === 'PASS').length
  const phaseFail = phaseTotal - phasePass

  const allSteps = report.phases.flatMap((phase) => phase.steps)
  const stepTotal = allSteps.length
  const stepPass = allSteps.filter((step) => step.result === 'PASS').length
  const stepFail = stepTotal - stepPass

  const cleanupSteps = report.cleanup?.steps || []
  const cleanupPass = cleanupSteps.filter((step) => step.result === 'PASS').length
  const cleanupFail = cleanupSteps.length - cleanupPass

  const diagnostics = summarizeDiagnostics(report.diagnostics)

  return {
    phaseTotal,
    phasePass,
    phaseFail,
    stepTotal,
    stepPass,
    stepFail,
    cleanupTotal: cleanupSteps.length,
    cleanupPass,
    cleanupFail,
    screenshotTotal: report.artifacts.screenshots.length,
    ...diagnostics,
  }
}

function renderMarkdown(report) {
  const summary = buildSummary(report)
  const lines = []

  lines.push('# Frontend E2E Summary')
  lines.push('')
  lines.push(`- Generated at: ${report.generatedAt}`)
  lines.push(`- Base URL: ${report.baseUrl}`)
  lines.push(`- Headless: ${report.headless}`)
  lines.push(`- Timeout(ms): ${report.timeoutMs}`)
  lines.push(`- Clean requested: ${report.cleanRequested}`)
  lines.push(`- E2E user: ${report.identity.username}`)
  lines.push(`- Project ID: ${report.projectId || 'N/A'}`)
  lines.push(`- Phases: ${summary.phaseTotal} (pass=${summary.phasePass}, fail=${summary.phaseFail})`)
  lines.push(`- Steps: ${summary.stepTotal} (pass=${summary.stepPass}, fail=${summary.stepFail})`)
  lines.push(`- Cleanup steps: ${summary.cleanupTotal} (pass=${summary.cleanupPass}, fail=${summary.cleanupFail})`)
  lines.push(
    `- Diagnostics: console.error=${summary.consoleErrors}, console.warning=${summary.consoleWarnings}, pageerror=${summary.pageErrors}, requestfailed=${summary.requestFailed}`,
  )
  lines.push(`- Screenshots: ${summary.screenshotTotal}`)
  lines.push('')

  lines.push('## Phases')
  lines.push('')
  lines.push('| Phase | Result | Steps | Pass | Fail | Duration(ms) | Error |')
  lines.push('|---|---|---|---|---|---|---|')

  for (const phase of report.phases) {
    const passCount = phase.steps.filter((step) => step.result === 'PASS').length
    const failCount = phase.steps.length - passCount
    lines.push(
      `| ${phase.id} | ${phase.status} | ${phase.steps.length} | ${passCount} | ${failCount} | ${phase.durationMs} | ${markdownCell(phase.error || '')} |`,
    )
  }

  for (const phase of report.phases) {
    lines.push('')
    lines.push(`## ${phase.id}`)
    lines.push('')
    lines.push('| Step | Result | Method | Path | HTTP Status | Duration(ms) | C.Err | C.Warn | P.Err | Req.Fail | Screenshot | Note |')
    lines.push('|---|---|---|---|---|---|---|---|---|---|---|---|')

    for (const step of phase.steps) {
      lines.push(
        `| ${markdownCell(step.id)} | ${step.result} | ${step.method} | ${markdownCell(step.path)} | ${markdownCell(step.httpStatus)} | ${step.durationMs} | ${step.consoleErrors} | ${step.consoleWarnings} | ${step.pageErrors} | ${step.requestFailed} | ${markdownCell(step.screenshot || '')} | ${markdownCell(step.note)} |`,
      )
    }
  }

  lines.push('')
  lines.push('## Cleanup')
  lines.push('')
  if (!report.cleanup || report.cleanup.steps.length === 0) {
    lines.push('- no cleanup steps executed')
  } else {
    lines.push('| Step | Result | Method | Path | HTTP Status | Duration(ms) | Note |')
    lines.push('|---|---|---|---|---|---|---|')
    for (const step of report.cleanup.steps) {
      lines.push(
        `| ${markdownCell(step.id)} | ${step.result} | ${step.method} | ${markdownCell(step.path)} | ${markdownCell(step.httpStatus)} | ${step.durationMs} | ${markdownCell(step.note)} |`,
      )
    }
  }

  lines.push('')
  lines.push('## Artifacts')
  lines.push('')
  lines.push(`- Console log: ${report.artifacts.consoleLog || 'N/A'}`)
  if (report.artifacts.screenshots.length === 0) {
    lines.push('- Screenshots: none')
  } else {
    for (const screenshot of report.artifacts.screenshots) {
      lines.push(`- Screenshot: ${screenshot}`)
    }
  }

  const failures = []
  for (const phase of report.phases) {
    if (phase.error) {
      failures.push(`${phase.id}: ${phase.error}`)
    }
    for (const step of phase.steps) {
      if (step.result === 'FAIL') {
        failures.push(`${phase.id}/${step.id}: ${step.note}`)
      }
    }
  }
  if (report.cleanup) {
    for (const step of report.cleanup.steps) {
      if (step.result === 'FAIL') {
        failures.push(`cleanup/${step.id}: ${step.note}`)
      }
    }
  }

  lines.push('')
  lines.push('## Failures')
  lines.push('')
  if (failures.length === 0) {
    lines.push('- none')
  } else {
    for (const entry of failures) {
      lines.push(`- ${entry}`)
    }
  }

  const severeDiagnostics = report.diagnostics.filter(isSevereDiagnostic)
  lines.push('')
  lines.push('## Severe Diagnostics')
  lines.push('')
  if (severeDiagnostics.length === 0) {
    lines.push('- none')
  } else {
    for (const entry of severeDiagnostics.slice(0, 60)) {
      lines.push(`- ${markdownCell(formatDiagnostic(entry))}`)
    }
    if (severeDiagnostics.length > 60) {
      lines.push(`- ... plus ${severeDiagnostics.length - 60} more entries`) 
    }
  }

  lines.push('')
  return `${lines.join('\n')}\n`
}

function writeReports(outDir, report) {
  fs.mkdirSync(outDir, { recursive: true })

  const jsonPath = path.join(outDir, 'e2e-frontend-summary.json')
  const mdPath = path.join(outDir, 'e2e-frontend-summary.md')
  const consoleLogPath = path.join(outDir, 'e2e-frontend-console.log')

  const summary = buildSummary(report)
  const payload = {
    ...report,
    summary,
    artifacts: {
      ...report.artifacts,
      consoleLog: path.relative(PROJECT_ROOT, consoleLogPath),
    },
  }

  fs.writeFileSync(consoleLogPath, renderDiagnosticsLog(payload.diagnostics), 'utf8')
  fs.writeFileSync(jsonPath, JSON.stringify(payload, null, 2), 'utf8')
  fs.writeFileSync(mdPath, renderMarkdown(payload), 'utf8')

  return {
    jsonPath,
    mdPath,
    consoleLogPath,
    summary,
  }
}

function getCurrentPathname(page) {
  const raw = page.url()
  if (!raw) {
    return ''
  }

  try {
    return new URL(raw).pathname
  } catch {
    return raw
  }
}

async function clearAuthTokenInBrowser(page) {
  await page.evaluate((key) => {
    window.localStorage.removeItem(key)
    window.sessionStorage.clear()
  }, AUTH_TOKEN_STORAGE_KEY)
}

async function readAuthTokenInBrowser(page) {
  return page.evaluate((key) => {
    const token = window.localStorage.getItem(key)
    if (typeof token !== 'string') {
      return null
    }
    const normalized = token.trim()
    return normalized.length > 0 ? normalized : null
  }, AUTH_TOKEN_STORAGE_KEY)
}

function ensureToken(context) {
  assert(
    typeof context.token === 'string' && context.token.trim().length > 0,
    'auth token is missing; auth-flow did not complete successfully',
  )
  return context.token
}

function ensureProjectId(context) {
  assert(
    typeof context.projectId === 'string' && context.projectId.trim().length > 0,
    'project id is missing; workspace phase did not complete successfully',
  )
  return context.projectId
}

function attachDiagnostics(page, context) {
  const push = (entry) => {
    context.diagnostics.push({
      ts: new Date().toISOString(),
      phaseId: context.activePhaseId,
      stepId: context.activeStepId,
      ...entry,
    })
  }

  const onConsole = (message) => {
    const type = message.type()
    if (type !== 'error' && type !== 'warning') {
      return
    }

    const location = message.location()
    push({
      kind: type === 'error' ? 'console.error' : 'console.warning',
      text: message.text(),
      url: location?.url || page.url(),
      line: Number.isInteger(location?.lineNumber) ? location.lineNumber + 1 : null,
      column: Number.isInteger(location?.columnNumber) ? location.columnNumber + 1 : null,
    })
  }

  const onPageError = (error) => {
    push({
      kind: 'pageerror',
      text: error instanceof Error ? error.message : String(error),
      url: page.url(),
      line: null,
      column: null,
    })
  }

  const onRequestFailed = (request) => {
    const failure = request.failure()
    push({
      kind: 'requestfailed',
      text: `${request.method()} ${request.url()} :: ${failure?.errorText || 'request failed'}`,
      url: request.url(),
      line: null,
      column: null,
    })
  }

  page.on('console', onConsole)
  page.on('pageerror', onPageError)
  page.on('requestfailed', onRequestFailed)

  return () => {
    page.off('console', onConsole)
    page.off('pageerror', onPageError)
    page.off('requestfailed', onRequestFailed)
  }
}

async function takeScreenshot(context, phaseId, stepId) {
  const fileName = `${String(context.screenshotCounter + 1).padStart(2, '0')}-${toSlug(phaseId)}-${toSlug(stepId)}.png`
  const absolutePath = path.join(context.outDir, fileName)
  await context.page.screenshot({ path: absolutePath, fullPage: true })
  context.screenshotCounter += 1

  const relativePath = path.relative(PROJECT_ROOT, absolutePath)
  context.screenshots.push(relativePath)
  return relativePath
}

async function gotoPath(context, routePath) {
  const targetUrl = new URL(routePath, context.baseUrl).toString()
  const response = await context.page.goto(targetUrl, {
    waitUntil: 'domcontentloaded',
    timeout: context.timeoutMs,
  })
  await context.page.waitForTimeout(250)

  return {
    targetUrl,
    httpStatus: response ? response.status() : '',
  }
}

async function waitForPathname(page, expectedPathname, timeoutMs, contextLabel) {
  const matcher =
    typeof expectedPathname === 'string'
      ? (url) => url.pathname === expectedPathname
      : (url) => expectedPathname.test(url.pathname)

  await page.waitForURL(matcher, { timeout: timeoutMs })

  const pathname = getCurrentPathname(page)
  if (typeof expectedPathname === 'string') {
    assert(pathname === expectedPathname, `${contextLabel}: expected pathname ${expectedPathname}, got ${pathname}`)
  } else {
    assert(expectedPathname.test(pathname), `${contextLabel}: pathname ${pathname} does not match ${expectedPathname}`)
  }
}

function assertNoRuntimeErrorsSince(context, startIndex, label) {
  const recent = context.diagnostics.slice(startIndex)
  const severe = recent.filter(isSevereDiagnostic)

  if (severe.length === 0) {
    return
  }

  const invalidBaseUrl = severe.find((entry) => /failed to construct 'url': invalid base url/i.test(entry.text))
  if (invalidBaseUrl) {
    throw new Error(`${label}: detected Invalid base URL regression (${truncateText(invalidBaseUrl.text)})`)
  }

  const top = severe.slice(0, 3).map(formatDiagnostic)
  throw new Error(`${label}: detected ${severe.length} runtime error(s) :: ${top.join(' | ')}`)
}

async function assertNoErrorBoundaryVisible(page, label) {
  const unexpectedBoundary = page.getByText('Unexpected Application Error!').first()
  const invalidBaseMessage = page.getByText(/Failed to construct 'URL': Invalid base URL/i).first()

  if (await unexpectedBoundary.isVisible()) {
    throw new Error(`${label}: React Router error boundary is visible`)
  }

  if (await invalidBaseMessage.isVisible()) {
    throw new Error(`${label}: Invalid base URL crash message is visible in DOM`)
  }
}

async function ensureWorkbenchStageButtons(context) {
  const configTab = context.page.getByRole('button', { name: WORKBENCH_STAGES[0].tabLabel }).first()
  const manualCreateEpisode = context.page.getByRole('button', { name: /Manual Create Episode/i }).first()
  const deadline = Date.now() + context.timeoutMs

  while (Date.now() < deadline) {
    if (await configTab.isVisible()) {
      return { createdEpisode: false }
    }

    if (await manualCreateEpisode.isVisible()) {
      await manualCreateEpisode.click()
      await configTab.waitFor({ state: 'visible', timeout: context.timeoutMs })
      return { createdEpisode: true }
    }

    await sleep(200)
  }

  throw new Error('project workbench did not render stage tabs or manual episode controls in time')
}

async function findAssetCharacterIdByName(context, characterName) {
  if (!context.token) {
    return null
  }

  for (let attempt = 0; attempt < 4; attempt += 1) {
    const response = await requestJson({
      baseUrl: context.baseUrl,
      method: 'GET',
      endpointPath: '/api/asset-hub/characters',
      token: context.token,
      timeoutMs: context.timeoutMs,
    })

    if (response.status === 200 && response.parseError === null && isObject(response.json)) {
      const characters = Array.isArray(response.json.characters) ? response.json.characters : []
      const found = characters.find(
        (item) => isObject(item) && item.name === characterName && typeof item.id === 'string',
      )

      if (found) {
        return found.id
      }
    }

    await sleep(350)
  }

  return null
}

async function runStep(phase, metadata, context, executor) {
  const startedAt = Date.now()
  const diagnosticsStart = context.diagnostics.length
  const previousPhase = context.activePhaseId
  const previousStep = context.activeStepId

  context.activePhaseId = phase.id
  context.activeStepId = metadata.id

  try {
    const result = await executor()
    const diagnostics = summarizeDiagnostics(context.diagnostics.slice(diagnosticsStart))

    phase.steps.push({
      ...metadata,
      result: 'PASS',
      httpStatus: result.httpStatus ?? '',
      durationMs: Date.now() - startedAt,
      note: result.note ?? '',
      screenshot: result.screenshot ?? '',
      ...diagnostics,
    })

    return {
      ok: true,
      value: result.value ?? null,
    }
  } catch (error) {
    const diagnostics = summarizeDiagnostics(context.diagnostics.slice(diagnosticsStart))
    const message = error instanceof Error ? error.message : String(error)
    phase.status = 'FAIL'

    let screenshot = ''
    if (context.page) {
      try {
        screenshot = await takeScreenshot(context, phase.id, `${metadata.id}-fail`)
      } catch {
        screenshot = ''
      }
    }

    phase.steps.push({
      ...metadata,
      result: 'FAIL',
      httpStatus: '',
      durationMs: Date.now() - startedAt,
      note: message,
      screenshot,
      ...diagnostics,
    })

    return {
      ok: false,
      value: null,
    }
  } finally {
    context.activePhaseId = previousPhase
    context.activeStepId = previousStep
  }
}

function finalizePhase(phase, startedAtMs) {
  phase.endedAt = new Date().toISOString()
  phase.durationMs = Date.now() - startedAtMs
  if (phase.steps.some((step) => step.result === 'FAIL')) {
    phase.status = 'FAIL'
  }
}

async function runCleanupStep(cleanup, metadata, executor) {
  const startedAt = Date.now()

  try {
    const result = await executor()
    cleanup.steps.push({
      ...metadata,
      result: 'PASS',
      httpStatus: result.httpStatus ?? '',
      durationMs: Date.now() - startedAt,
      note: result.note ?? '',
    })
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    cleanup.steps.push({
      ...metadata,
      result: 'FAIL',
      httpStatus: '',
      durationMs: Date.now() - startedAt,
      note: message,
    })
  }
}

async function runAuthFlow(phase, context) {
  await runStep(
    phase,
    {
      id: 'landing-route-render',
      method: 'BROWSER',
      path: '/',
    },
    context,
    async () => {
      const navigation = await gotoPath(context, '/')
      await context.page.locator('main.page-shell').first().waitFor({ state: 'visible', timeout: context.timeoutMs })
      await context.page.getByText('React + Vite + Axum').first().waitFor({ state: 'visible', timeout: context.timeoutMs })

      const screenshot = await takeScreenshot(context, phase.id, 'landing-route-render')
      return {
        httpStatus: navigation.httpStatus,
        note: 'landing page rendered',
        screenshot,
      }
    },
  )

  const signupRender = await runStep(
    phase,
    {
      id: 'signup-form-render',
      method: 'BROWSER',
      path: '/auth/signup',
    },
    context,
    async () => {
      await clearAuthTokenInBrowser(context.page)
      const navigation = await gotoPath(context, '/auth/signup')

      await context.page.locator('#signup-name').waitFor({ state: 'visible', timeout: context.timeoutMs })
      await context.page.locator('#signup-password').waitFor({ state: 'visible', timeout: context.timeoutMs })

      const screenshot = await takeScreenshot(context, phase.id, 'signup-form-render')
      return {
        httpStatus: navigation.httpStatus,
        note: 'signup form visible',
        screenshot,
      }
    },
  )

  if (!signupRender.ok) {
    return
  }

  const signupSubmit = await runStep(
    phase,
    {
      id: 'signup-submit-redirect-workspace',
      method: 'BROWSER',
      path: '/auth/signup',
    },
    context,
    async () => {
      await context.page.locator('#signup-name').fill(context.identity.username)
      await context.page.locator('#signup-password').fill(context.identity.password)

      const submitButton = context.page.locator('form button[type="submit"]').first()
      await Promise.all([
        waitForPathname(context.page, '/workspace', context.timeoutMs, 'signup redirect'),
        submitButton.click(),
      ])

      const token = await readAuthTokenInBrowser(context.page)
      assert(token !== null, 'signup redirect: auth token missing in localStorage')
      context.token = token

      const screenshot = await takeScreenshot(context, phase.id, 'signup-submit-redirect-workspace')
      return {
        note: `redirected=${getCurrentPathname(context.page)}`,
        screenshot,
      }
    },
  )

  if (!signupSubmit.ok) {
    return
  }

  const signinRender = await runStep(
    phase,
    {
      id: 'signin-form-render-after-clear-storage',
      method: 'BROWSER',
      path: '/auth/signin',
    },
    context,
    async () => {
      await clearAuthTokenInBrowser(context.page)
      const tokenAfterClear = await readAuthTokenInBrowser(context.page)
      assert(tokenAfterClear === null, 'clear storage step failed to remove auth token')

      const navigation = await gotoPath(context, '/auth/signin')
      await context.page.locator('#signin-username').waitFor({ state: 'visible', timeout: context.timeoutMs })
      await context.page.locator('#signin-password').waitFor({ state: 'visible', timeout: context.timeoutMs })
      await waitForPathname(context.page, '/auth/signin', context.timeoutMs, 'signin route check')

      const screenshot = await takeScreenshot(context, phase.id, 'signin-form-render-after-clear-storage')
      return {
        httpStatus: navigation.httpStatus,
        note: 'signin form visible',
        screenshot,
      }
    },
  )

  if (!signinRender.ok) {
    return
  }

  const signinSubmit = await runStep(
    phase,
    {
      id: 'signin-submit-redirect-workspace',
      method: 'BROWSER',
      path: '/auth/signin',
    },
    context,
    async () => {
      await context.page.locator('#signin-username').fill(context.identity.username)
      await context.page.locator('#signin-password').fill(context.identity.password)

      const submitButton = context.page.locator('form button[type="submit"]').first()
      await Promise.all([
        waitForPathname(context.page, '/workspace', context.timeoutMs, 'signin redirect'),
        submitButton.click(),
      ])

      const token = await readAuthTokenInBrowser(context.page)
      assert(token !== null, 'signin redirect: auth token missing in localStorage')
      context.token = token

      const screenshot = await takeScreenshot(context, phase.id, 'signin-submit-redirect-workspace')
      return {
        note: `redirected=${getCurrentPathname(context.page)}`,
        screenshot,
      }
    },
  )

  if (!signinSubmit.ok) {
    return
  }

  await runStep(
    phase,
    {
      id: 'verify-auth-token-storage',
      method: 'BROWSER',
      path: '/workspace',
    },
    context,
    async () => {
      const token = await readAuthTokenInBrowser(context.page)
      assert(token !== null, `localStorage key ${AUTH_TOKEN_STORAGE_KEY} is missing`)
      context.token = token

      const screenshot = await takeScreenshot(context, phase.id, 'verify-auth-token-storage')
      return {
        note: `tokenPrefix=${token.slice(0, 12)}...`,
        screenshot,
      }
    },
  )
}

async function runWorkspace(phase, context) {
  const listStep = await runStep(
    phase,
    {
      id: 'workspace-list-render',
      method: 'BROWSER',
      path: '/workspace',
    },
    context,
    async () => {
      const navigation = await gotoPath(context, '/workspace')
      await waitForPathname(context.page, '/workspace', context.timeoutMs, 'workspace route')
      await context.page.locator('#workspace-search').waitFor({ state: 'visible', timeout: context.timeoutMs })

      const screenshot = await takeScreenshot(context, phase.id, 'workspace-list-render')
      return {
        httpStatus: navigation.httpStatus,
        note: 'workspace list rendered',
        screenshot,
      }
    },
  )

  if (!listStep.ok) {
    return
  }

  const createProject = await runStep(
    phase,
    {
      id: 'workspace-create-project-via-ui',
      method: 'BROWSER',
      path: '/workspace',
    },
    context,
    async () => {
      const createButton = context.page.locator('main.page-shell > header button').first()
      await createButton.waitFor({ state: 'visible', timeout: context.timeoutMs })
      await createButton.click()

      const nameInput = context.page.locator('#project-create-name')
      await nameInput.waitFor({ state: 'visible', timeout: context.timeoutMs })
      await nameInput.fill(context.identity.projectName)

      const descriptionInput = context.page.locator('#project-create-description')
      await descriptionInput.fill('Created by scripts/e2e-frontend.mjs')

      const createResponsePromise = context.page
        .waitForResponse(
          (response) => response.url().includes('/api/projects') && response.request().method() === 'POST',
          { timeout: context.timeoutMs },
        )
        .catch(() => null)

      await context.page.locator('form:has(#project-create-name) button[type="submit"]').first().click()
      const createResponse = await createResponsePromise

      await nameInput.waitFor({ state: 'hidden', timeout: context.timeoutMs })
      await context.page
        .locator('[role="link"]')
        .filter({ hasText: context.identity.projectName })
        .first()
        .waitFor({ state: 'visible', timeout: context.timeoutMs })

      const screenshot = await takeScreenshot(context, phase.id, 'workspace-create-project-via-ui')
      return {
        httpStatus: createResponse ? createResponse.status() : '',
        note: `projectName=${context.identity.projectName}`,
        screenshot,
      }
    },
  )

  if (!createProject.ok) {
    return
  }

  const verifyProject = await runStep(
    phase,
    {
      id: 'workspace-verify-project-in-list',
      method: 'BROWSER',
      path: '/workspace',
    },
    context,
    async () => {
      const projectCard = context.page
        .locator('[role="link"]')
        .filter({ hasText: context.identity.projectName })
        .first()

      await projectCard.waitFor({ state: 'visible', timeout: context.timeoutMs })

      const screenshot = await takeScreenshot(context, phase.id, 'workspace-verify-project-in-list')
      return {
        note: 'project card visible',
        screenshot,
      }
    },
  )

  if (!verifyProject.ok) {
    return
  }

  await runStep(
    phase,
    {
      id: 'workspace-open-project-workbench',
      method: 'BROWSER',
      path: '/workspace/:projectId',
    },
    context,
    async () => {
      const projectCard = context.page
        .locator('[role="link"]')
        .filter({ hasText: context.identity.projectName })
        .first()

      await Promise.all([
        waitForPathname(context.page, /^\/workspace\/[^/]+$/i, context.timeoutMs, 'open project workbench'),
        projectCard.click(),
      ])

      const pathname = getCurrentPathname(context.page)
      const match = pathname.match(/^\/workspace\/([^/]+)$/i)
      assert(match && match[1] !== 'asset-hub', `workspace open project: unexpected pathname ${pathname}`)
      context.projectId = decodeURIComponent(match[1])

      const screenshot = await takeScreenshot(context, phase.id, 'workspace-open-project-workbench')
      return {
        note: `projectId=${context.projectId}`,
        screenshot,
      }
    },
  )
}

async function runProjectWorkbench(phase, context) {
  const projectId = ensureProjectId(context)
  const phaseDiagnosticsStart = context.diagnostics.length

  const loadStep = await runStep(
    phase,
    {
      id: 'workbench-load-without-crash',
      method: 'BROWSER',
      path: `/workspace/${projectId}`,
    },
    context,
    async () => {
      const diagnosticsStart = context.diagnostics.length
      const navigation = await gotoPath(context, `/workspace/${encodeURIComponent(projectId)}`)

      await context.page.locator('main.page-shell').first().waitFor({ state: 'visible', timeout: context.timeoutMs })
      const stageStatus = await ensureWorkbenchStageButtons(context)

      await assertNoErrorBoundaryVisible(context.page, 'project workbench load')
      assertNoRuntimeErrorsSince(context, diagnosticsStart, 'project workbench load')

      const screenshot = await takeScreenshot(context, phase.id, 'workbench-load-without-crash')
      return {
        httpStatus: navigation.httpStatus,
        note: stageStatus.createdEpisode ? 'created initial episode' : 'episode already present',
        screenshot,
      }
    },
  )

  if (!loadStep.ok) {
    return
  }

  const tabsStep = await runStep(
    phase,
    {
      id: 'workbench-verify-nine-stage-tabs',
      method: 'BROWSER',
      path: `/workspace/${projectId}`,
    },
    context,
    async () => {
      for (const stage of WORKBENCH_STAGES) {
        const tab = context.page.getByRole('button', { name: stage.tabLabel }).first()
        await tab.waitFor({ state: 'visible', timeout: context.timeoutMs })
      }

      const screenshot = await takeScreenshot(context, phase.id, 'workbench-verify-nine-stage-tabs')
      return {
        note: `stages=${WORKBENCH_STAGES.map((stage) => stage.id).join(',')}`,
        screenshot,
      }
    },
  )

  if (!tabsStep.ok) {
    return
  }

  await runStep(
    phase,
    {
      id: 'workbench-switch-all-stage-tabs',
      method: 'BROWSER',
      path: `/workspace/${projectId}`,
    },
    context,
    async () => {
      const diagnosticsStart = context.diagnostics.length
      const visited = []

      for (const stage of WORKBENCH_STAGES) {
        const tab = context.page.getByRole('button', { name: stage.tabLabel }).first()
        await tab.click()

        const heading = context.page.getByRole('heading', { name: stage.headingLabel }).first()
        await heading.waitFor({ state: 'visible', timeout: context.timeoutMs })
        visited.push(stage.id)
      }

      await assertNoErrorBoundaryVisible(context.page, 'project workbench stage switch')
      assertNoRuntimeErrorsSince(context, diagnosticsStart, 'project workbench stage switch')

      const screenshot = await takeScreenshot(context, phase.id, 'workbench-switch-all-stage-tabs')
      return {
        note: `visited=${visited.join(' -> ')}`,
        screenshot,
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'workbench-console-error-check',
      method: 'BROWSER',
      path: `/workspace/${projectId}`,
    },
    context,
    async () => {
      const phaseDiagnostics = context.diagnostics.slice(phaseDiagnosticsStart)
      const severe = phaseDiagnostics.filter(isSevereDiagnostic)
      assert(
        severe.length === 0,
        `project workbench diagnostics contain severe errors: ${severe.slice(0, 3).map(formatDiagnostic).join(' | ')}`,
      )

      const totals = summarizeDiagnostics(phaseDiagnostics)
      const screenshot = await takeScreenshot(context, phase.id, 'workbench-console-error-check')
      return {
        note: `consoleErrors=${totals.consoleErrors}, consoleWarnings=${totals.consoleWarnings}, pageErrors=${totals.pageErrors}`,
        screenshot,
      }
    },
  )
}

async function runAssetHub(phase, context) {
  const phaseDiagnosticsStart = context.diagnostics.length

  const loadStep = await runStep(
    phase,
    {
      id: 'asset-hub-load-without-crash',
      method: 'BROWSER',
      path: '/workspace/asset-hub',
    },
    context,
    async () => {
      const diagnosticsStart = context.diagnostics.length
      const navigation = await gotoPath(context, '/workspace/asset-hub')

      await context.page.locator('main.page-shell').first().waitFor({ state: 'visible', timeout: context.timeoutMs })
      await context.page
        .getByText(/SSE (Connected|Reconnecting)/i)
        .first()
        .waitFor({ state: 'visible', timeout: context.timeoutMs })

      await assertNoErrorBoundaryVisible(context.page, 'asset hub load')
      assertNoRuntimeErrorsSince(context, diagnosticsStart, 'asset hub load')

      const screenshot = await takeScreenshot(context, phase.id, 'asset-hub-load-without-crash')
      return {
        httpStatus: navigation.httpStatus,
        note: 'asset hub route rendered',
        screenshot,
      }
    },
  )

  if (!loadStep.ok) {
    return
  }

  const sectionStep = await runStep(
    phase,
    {
      id: 'asset-hub-verify-character-location-voice-sections',
      method: 'BROWSER',
      path: '/workspace/asset-hub',
    },
    context,
    async () => {
      const kpiCards = context.page.locator('main .glass-kpi')
      await kpiCards.first().waitFor({ state: 'visible', timeout: context.timeoutMs })

      const kpiCount = await kpiCards.count()
      assert(kpiCount >= 3, `asset hub sections: expected >=3 KPI cards, got ${kpiCount}`)

      const sectionHeadingCount = await context.page.locator('main .glass-surface h3').count()
      assert(sectionHeadingCount >= 3, `asset hub sections: expected >=3 section headings, got ${sectionHeadingCount}`)

      const screenshot = await takeScreenshot(context, phase.id, 'asset-hub-verify-character-location-voice-sections')
      return {
        note: `kpi=${kpiCount}, headings=${sectionHeadingCount}`,
        screenshot,
      }
    },
  )

  if (!sectionStep.ok) {
    return
  }

  await runStep(
    phase,
    {
      id: 'asset-hub-create-character-form',
      method: 'BROWSER',
      path: '/workspace/asset-hub',
    },
    context,
    async () => {
      const diagnosticsStart = context.diagnostics.length

      const summaryPanel = context.page
        .locator('main .glass-surface')
        .filter({ has: context.page.locator('.glass-kpi') })
        .first()

      await summaryPanel.waitFor({ state: 'visible', timeout: context.timeoutMs })

      const actionButtons = summaryPanel.locator('button')
      const actionButtonCount = await actionButtons.count()
      assert(actionButtonCount >= 1, 'asset hub create character: action buttons are missing')

      await actionButtons.first().click()
      const nameInput = context.page.locator('#character-name')
      await nameInput.waitFor({ state: 'visible', timeout: context.timeoutMs })
      await nameInput.fill(context.identity.assetCharacterName)
      await context.page
        .locator('#character-description')
        .fill('Created by scripts/e2e-frontend.mjs regression coverage')

      await context.page.locator('form:has(#character-name) button[type="submit"]').first().click()
      await nameInput.waitFor({ state: 'hidden', timeout: context.timeoutMs })
      await context.page
        .getByText(context.identity.assetCharacterName)
        .first()
        .waitFor({ state: 'visible', timeout: context.timeoutMs })

      assertNoRuntimeErrorsSince(context, diagnosticsStart, 'asset hub create character')

      const createdCharacterId = await findAssetCharacterIdByName(context, context.identity.assetCharacterName)
      if (createdCharacterId) {
        context.assetCharacterIds.push(createdCharacterId)
      }

      const screenshot = await takeScreenshot(context, phase.id, 'asset-hub-create-character-form')
      return {
        note: createdCharacterId
          ? `character=${context.identity.assetCharacterName}, characterId=${createdCharacterId}`
          : `character=${context.identity.assetCharacterName}, characterId=not-found`,
        screenshot,
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'asset-hub-console-error-check',
      method: 'BROWSER',
      path: '/workspace/asset-hub',
    },
    context,
    async () => {
      const phaseDiagnostics = context.diagnostics.slice(phaseDiagnosticsStart)
      const severe = phaseDiagnostics.filter(isSevereDiagnostic)
      assert(
        severe.length === 0,
        `asset hub diagnostics contain severe errors: ${severe.slice(0, 3).map(formatDiagnostic).join(' | ')}`,
      )

      const totals = summarizeDiagnostics(phaseDiagnostics)
      const screenshot = await takeScreenshot(context, phase.id, 'asset-hub-console-error-check')
      return {
        note: `consoleErrors=${totals.consoleErrors}, consoleWarnings=${totals.consoleWarnings}, pageErrors=${totals.pageErrors}`,
        screenshot,
      }
    },
  )
}

async function runProfile(phase, context) {
  const profileLoad = await runStep(
    phase,
    {
      id: 'profile-route-load',
      method: 'BROWSER',
      path: '/profile',
    },
    context,
    async () => {
      const diagnosticsStart = context.diagnostics.length
      const navigation = await gotoPath(context, '/profile')

      const container = context.page.locator('section.glass-surface-elevated').first()
      await container.waitFor({ state: 'visible', timeout: context.timeoutMs })

      await assertNoErrorBoundaryVisible(context.page, 'profile route')
      assertNoRuntimeErrorsSince(context, diagnosticsStart, 'profile route')

      const screenshot = await takeScreenshot(context, phase.id, 'profile-route-load')
      return {
        httpStatus: navigation.httpStatus,
        note: 'profile route rendered',
        screenshot,
      }
    },
  )

  if (!profileLoad.ok) {
    return
  }

  await runStep(
    phase,
    {
      id: 'profile-verify-provider-model-ui',
      method: 'BROWSER',
      path: '/profile',
    },
    context,
    async () => {
      const container = context.page.locator('section.glass-surface-elevated').first()
      await container.waitFor({ state: 'visible', timeout: context.timeoutMs })

      let controlCount = 0
      const controls = container.locator('input,select,button,textarea')
      const deadline = Date.now() + context.timeoutMs

      while (Date.now() < deadline) {
        controlCount = await controls.count()
        if (controlCount > 0) {
          break
        }
        await sleep(200)
      }

      assert(controlCount > 0, 'profile route: provider/model configuration controls are not visible')

      const screenshot = await takeScreenshot(context, phase.id, 'profile-verify-provider-model-ui')
      return {
        note: `controls=${controlCount}`,
        screenshot,
      }
    },
  )
}

async function runAdminNonAdmin(phase, context) {
  const adminLoad = await runStep(
    phase,
    {
      id: 'admin-ai-config-route-load',
      method: 'BROWSER',
      path: '/admin/ai-config',
    },
    context,
    async () => {
      const navigation = await gotoPath(context, '/admin/ai-config')
      await context.page.locator('main.page-shell').first().waitFor({ state: 'visible', timeout: context.timeoutMs })

      const screenshot = await takeScreenshot(context, phase.id, 'admin-ai-config-route-load')
      return {
        httpStatus: navigation.httpStatus,
        note: `path=${getCurrentPathname(context.page)}`,
        screenshot,
      }
    },
  )

  if (!adminLoad.ok) {
    return
  }

  await runStep(
    phase,
    {
      id: 'admin-ai-config-non-admin-state',
      method: 'BROWSER',
      path: '/admin/ai-config',
    },
    context,
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/api/admin/ai-config',
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })
      expectStatusOneOf(response, [401, 403], 'admin ai-config non-admin API status')

      const emptyState = context.page.getByText('No config loaded').first()
      await emptyState.waitFor({ state: 'visible', timeout: context.timeoutMs })

      const screenshot = await takeScreenshot(context, phase.id, 'admin-ai-config-non-admin-state')
      return {
        httpStatus: response.status,
        note: `non-admin status=${response.status}`,
        screenshot,
      }
    },
  )
}

async function runI18nRedirect(phase, context) {
  const zhStep = await runStep(
    phase,
    {
      id: 'locale-zh-workspace-redirect',
      method: 'BROWSER',
      path: '/zh/workspace',
    },
    context,
    async () => {
      const navigation = await gotoPath(context, '/zh/workspace')
      await waitForPathname(context.page, '/workspace', context.timeoutMs, 'zh/workspace redirect')

      const screenshot = await takeScreenshot(context, phase.id, 'locale-zh-workspace-redirect')
      return {
        httpStatus: navigation.httpStatus,
        note: `redirected=${getCurrentPathname(context.page)}`,
        screenshot,
      }
    },
  )

  if (!zhStep.ok) {
    return
  }

  await runStep(
    phase,
    {
      id: 'locale-en-workspace-redirect',
      method: 'BROWSER',
      path: '/en/workspace',
    },
    context,
    async () => {
      const navigation = await gotoPath(context, '/en/workspace')
      await waitForPathname(context.page, '/workspace', context.timeoutMs, 'en/workspace redirect')

      const screenshot = await takeScreenshot(context, phase.id, 'locale-en-workspace-redirect')
      return {
        httpStatus: navigation.httpStatus,
        note: `redirected=${getCurrentPathname(context.page)}`,
        screenshot,
      }
    },
  )
}

async function runNotFound(phase, context) {
  await runStep(
    phase,
    {
      id: 'not-found-route',
      method: 'BROWSER',
      path: '/nonexistent-page',
    },
    context,
    async () => {
      const navigation = await gotoPath(context, '/nonexistent-page')

      await context.page
        .getByRole('heading', { name: /Page not found/i })
        .first()
        .waitFor({ state: 'visible', timeout: context.timeoutMs })

      const screenshot = await takeScreenshot(context, phase.id, 'not-found-route')
      return {
        httpStatus: navigation.httpStatus,
        note: 'not found page visible',
        screenshot,
      }
    },
  )
}

async function runCleanup(report, context) {
  const cleanup = {
    requested: context.clean,
    startedAt: new Date().toISOString(),
    endedAt: null,
    steps: [],
  }
  report.cleanup = cleanup

  if (!context.token && context.page) {
    try {
      context.token = await readAuthTokenInBrowser(context.page)
    } catch {
      context.token = null
    }
  }

  if (!context.clean) {
    cleanup.endedAt = new Date().toISOString()
    return
  }

  const uniqueCharacterIds = [...new Set(context.assetCharacterIds)]
  for (const characterId of uniqueCharacterIds) {
    await runCleanupStep(
      cleanup,
      {
        id: `delete-asset-character-${characterId}`,
        method: 'DELETE',
        path: `/api/asset-hub/characters/${characterId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'DELETE',
          endpointPath: `/api/asset-hub/characters/${encodeURIComponent(characterId)}`,
          token: ensureToken(context),
          timeoutMs: context.timeoutMs,
        })

        expectStatusOneOf(response, [200, 404], `cleanup asset character ${characterId}`)

        return {
          httpStatus: response.status,
          note: 'done',
        }
      },
    )
  }

  if (context.token && context.projectId) {
    await runCleanupStep(
      cleanup,
      {
        id: 'delete-project',
        method: 'DELETE',
        path: `/api/projects/${context.projectId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'DELETE',
          endpointPath: `/api/projects/${encodeURIComponent(context.projectId)}`,
          token: context.token,
          timeoutMs: context.timeoutMs,
        })

        expectStatusOneOf(response, [200, 404], `cleanup project ${context.projectId}`)

        return {
          httpStatus: response.status,
          note: `projectId=${context.projectId}`,
        }
      },
    )
  }

  cleanup.endedAt = new Date().toISOString()
}

async function startBrowser(context, playwright) {
  assert(playwright && playwright.chromium, 'playwright chromium launcher is unavailable')

  context.browser = await playwright.chromium.launch({
    headless: context.headless,
  })

  context.browserContext = await context.browser.newContext({
    viewport: {
      width: 1440,
      height: 960,
    },
  })

  context.page = await context.browserContext.newPage()
  context.page.setDefaultTimeout(context.timeoutMs)
  context.page.setDefaultNavigationTimeout(context.timeoutMs)

  context.detachDiagnostics = attachDiagnostics(context.page, context)
}

async function closeBrowser(context) {
  if (typeof context.detachDiagnostics === 'function') {
    try {
      context.detachDiagnostics()
    } catch {
      // ignore detach failures
    }
  }
  context.detachDiagnostics = null

  if (context.browserContext) {
    try {
      await context.browserContext.close()
    } catch {
      // ignore close failures
    }
  }
  context.browserContext = null
  context.page = null

  if (context.browser) {
    try {
      await context.browser.close()
    } catch {
      // ignore close failures
    }
  }
  context.browser = null
}

async function main() {
  const args = parseArgs(process.argv.slice(2))
  const baseUrl = normalizeBaseUrl(args.base || args['frontend-base'] || DEFAULT_BASE_URL, 'base')
  const outDir = path.resolve(args['out-dir'] || DEFAULT_OUT_DIR)
  const timeoutMs = parseOptionalInt(args['timeout-ms'], 'timeout-ms', DEFAULT_TIMEOUT_MS)

  if (args.dryRun) {
    console.log(`[dry-run] base: ${baseUrl}`)
    console.log(`[dry-run] timeout-ms: ${timeoutMs}`)
    console.log(`[dry-run] headless: ${args.headless}`)
    console.log(`[dry-run] clean: ${args.clean}`)
    console.log(`[dry-run] report-json: ${path.join(outDir, 'e2e-frontend-summary.json')}`)
    console.log(`[dry-run] report-md: ${path.join(outDir, 'e2e-frontend-summary.md')}`)
    console.log(`[dry-run] console-log: ${path.join(outDir, 'e2e-frontend-console.log')}`)
    console.log(`[dry-run] screenshots-dir: ${outDir}`)
    console.log(`[dry-run] phases: ${PHASE_SEQUENCE.join(' -> ')}`)
    return
  }

  fs.mkdirSync(outDir, { recursive: true })

  const identity = createIdentity()
  const diagnostics = []

  const report = {
    generatedAt: new Date().toISOString(),
    baseUrl,
    timeoutMs,
    headless: args.headless,
    cleanRequested: args.clean,
    identity: {
      username: identity.username,
    },
    projectId: null,
    phases: [],
    cleanup: null,
    diagnostics,
    artifacts: {
      screenshots: [],
      consoleLog: null,
    },
  }

  const context = {
    baseUrl,
    timeoutMs,
    headless: args.headless,
    clean: args.clean,
    outDir,
    identity,
    token: null,
    projectId: null,
    assetCharacterIds: [],
    browser: null,
    browserContext: null,
    page: null,
    detachDiagnostics: null,
    activePhaseId: null,
    activeStepId: null,
    diagnostics,
    screenshotCounter: 0,
    screenshots: [],
  }

  const phaseHandlers = {
    'auth-flow': runAuthFlow,
    workspace: runWorkspace,
    'project-workbench': runProjectWorkbench,
    'asset-hub': runAssetHub,
    profile: runProfile,
    'admin-non-admin': runAdminNonAdmin,
    'i18n-redirect': runI18nRedirect,
    'not-found': runNotFound,
  }

  let bootstrapError = null

  try {
    const playwright = loadPlaywright()
    await startBrowser(context, playwright)

    for (const phaseId of PHASE_SEQUENCE) {
      const phase = createPhase(phaseId)
      report.phases.push(phase)
      const startedAtMs = Date.now()

      try {
        await phaseHandlers[phaseId](phase, context)
      } catch (error) {
        phase.status = 'FAIL'
        phase.error = error instanceof Error ? error.message : String(error)
      } finally {
        finalizePhase(phase, startedAtMs)
      }
    }
  } catch (error) {
    bootstrapError = error instanceof Error ? error.message : String(error)
  }

  if (bootstrapError) {
    const bootstrapPhase = createPhase('bootstrap')
    bootstrapPhase.status = 'FAIL'
    bootstrapPhase.error = bootstrapError
    bootstrapPhase.endedAt = new Date().toISOString()
    bootstrapPhase.durationMs = 0
    report.phases.unshift(bootstrapPhase)
  }

  report.projectId = context.projectId

  try {
    await runCleanup(report, context)
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    report.cleanup = report.cleanup || {
      requested: context.clean,
      startedAt: new Date().toISOString(),
      endedAt: new Date().toISOString(),
      steps: [],
    }
    report.cleanup.steps.push({
      id: 'cleanup-fatal-error',
      method: 'SYSTEM',
      path: 'cleanup',
      result: 'FAIL',
      httpStatus: '',
      durationMs: 0,
      note: message,
    })
  }

  await closeBrowser(context)

  report.artifacts.screenshots = context.screenshots

  const { jsonPath, mdPath, consoleLogPath, summary } = writeReports(outDir, report)
  console.log(`wrote ${jsonPath}`)
  console.log(`wrote ${mdPath}`)
  console.log(`wrote ${consoleLogPath}`)
  console.log(
    `PHASE_PASS=${summary.phasePass} PHASE_FAIL=${summary.phaseFail} STEP_PASS=${summary.stepPass} STEP_FAIL=${summary.stepFail} CLEANUP_FAIL=${summary.cleanupFail} CONSOLE_ERROR=${summary.consoleErrors} PAGE_ERROR=${summary.pageErrors}`,
  )

  if (summary.phaseFail > 0 || summary.stepFail > 0 || summary.cleanupFail > 0 || bootstrapError) {
    process.exitCode = 1
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error))
  process.exitCode = 1
})
