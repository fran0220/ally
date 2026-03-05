#!/usr/bin/env node

import { randomUUID } from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'

const SCRIPT_DIR = path.dirname(new URL(import.meta.url).pathname)
const PROJECT_ROOT = path.resolve(SCRIPT_DIR, '..')
const DEFAULT_OUT_DIR = path.join(PROJECT_ROOT, 'reports')
const DEFAULT_TIMEOUT_MS = 15000

const MODEL_GROUP_KEYS = ['llm', 'image', 'video', 'audio', 'lipsync']

const STEP_PLAN = [
  'register user and obtain bearer token',
  'get /api/user-preference and verify default values',
  'patch /api/user-preference with analysisModel + videoRatio',
  'get /api/user-preference and verify first update',
  'patch /api/user-preference with artStyle + ttsRate',
  'patch /api/user-preference with empty object and expect 400',
  'get /api/user/models and verify group arrays',
  'verify /api/user/models item fields',
  'get /api/user/api-config and verify response shape',
  'put /api/user/api-config with providers update',
  'get /api/user/api-config and verify persisted providers',
  'post /api/user/api-config/test-connection without apiKey and expect 400',
  'post /api/user/api-config/test-connection with invalid provider and expect 400',
]

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

function assertJsonObject(value, name) {
  assert(value !== null && typeof value === 'object' && !Array.isArray(value), `${name} must be a JSON object`)
}

function requireNonEmptyString(value, name) {
  assert(typeof value === 'string', `${name} must be a string`)
  const normalized = value.trim()
  assert(normalized.length > 0, `${name} must be a non-empty string`)
  return normalized
}

function normalizeOptionalString(value, name) {
  if (value === undefined || value === null) {
    return null
  }
  assert(typeof value === 'string', `${name} must be a string or null`)
  return value
}

function expectStatus(result, expectedStatus, context) {
  assert(
    result.status === expectedStatus,
    `${context}: expected status ${expectedStatus}, got ${result.status}, body=${result.rawBody}`,
  )
}

function readErrorMessage(payload, context) {
  assertJsonObject(payload, `${context}: payload`)
  return requireNonEmptyString(payload.message, `${context}: payload.message`)
}

async function requestJson({ baseUrl, requestPath, method, token, body, timeoutMs }) {
  const targetUrl = new URL(requestPath, baseUrl).toString()
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), timeoutMs)

  const headers = {
    Accept: 'application/json',
  }
  if (token) {
    headers.Authorization = `Bearer ${token}`
  }
  if (body !== undefined) {
    headers['Content-Type'] = 'application/json'
  }

  try {
    const response = await fetch(targetUrl, {
      method,
      headers,
      body: body !== undefined ? JSON.stringify(body) : undefined,
      signal: controller.signal,
    })

    const rawBody = await response.text()
    let json = null
    if (rawBody.length > 0) {
      const parsed = safeJsonParse(rawBody)
      assert(
        parsed.ok,
        `${method} ${requestPath} returned non-JSON body (status=${response.status}) body=${rawBody}`,
      )
      json = parsed.value
    }

    return {
      url: targetUrl,
      status: response.status,
      json,
      rawBody,
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    if (error instanceof Error && error.name === 'AbortError') {
      throw new Error(`${method} ${requestPath} timed out after ${timeoutMs}ms`)
    }
    throw new Error(`${method} ${requestPath} request failed: ${message}`)
  } finally {
    clearTimeout(timer)
  }
}

function verifyPreferenceDefaults(preference, context) {
  assertJsonObject(preference, `${context}: preference`)
  requireNonEmptyString(preference.id, `${context}: preference.id`)
  requireNonEmptyString(preference.userId, `${context}: preference.userId`)
  requireNonEmptyString(preference.createdAt, `${context}: preference.createdAt`)
  requireNonEmptyString(preference.updatedAt, `${context}: preference.updatedAt`)

  assert(preference.videoRatio === '9:16', `${context}: preference.videoRatio should be 9:16`)
  assert(preference.artStyle === 'american-comic', `${context}: preference.artStyle should be american-comic`)
  assert(preference.ttsRate === '+50%', `${context}: preference.ttsRate should be +50%`)
}

function verifyModelGroupsShape(payload, context) {
  assertJsonObject(payload, `${context}: payload`)

  for (const key of MODEL_GROUP_KEYS) {
    assert(Array.isArray(payload[key]), `${context}: ${key} must be an array`)
  }
}

function verifyModelItems(payload, context) {
  verifyModelGroupsShape(payload, context)

  for (const key of MODEL_GROUP_KEYS) {
    const group = payload[key]
    for (let index = 0; index < group.length; index += 1) {
      const item = group[index]
      assertJsonObject(item, `${context}: ${key}[${index}]`)
      requireNonEmptyString(item.value, `${context}: ${key}[${index}].value`)
      requireNonEmptyString(item.label, `${context}: ${key}[${index}].label`)
      requireNonEmptyString(item.provider, `${context}: ${key}[${index}].provider`)

      if (item.providerName !== undefined && item.providerName !== null) {
        requireNonEmptyString(item.providerName, `${context}: ${key}[${index}].providerName`)
      }
    }
  }
}

function verifyApiConfigShape(payload, context) {
  assertJsonObject(payload, `${context}: payload`)
  assert(Array.isArray(payload.models), `${context}: models must be an array`)
  assert(Array.isArray(payload.providers), `${context}: providers must be an array`)
  assertJsonObject(payload.defaultModels, `${context}: defaultModels`)
  assertJsonObject(payload.capabilityDefaults, `${context}: capabilityDefaults`)
}

function verifyProviders(actualProviders, expectedProviders, context) {
  assert(Array.isArray(actualProviders), `${context}: providers must be an array`)
  assert(
    actualProviders.length === expectedProviders.length,
    `${context}: providers length mismatch expected=${expectedProviders.length} actual=${actualProviders.length}`,
  )

  const expectedById = new Map()
  for (const provider of expectedProviders) {
    expectedById.set(provider.id.toLowerCase(), provider)
  }

  const seen = new Set()

  for (let index = 0; index < actualProviders.length; index += 1) {
    const actual = actualProviders[index]
    assertJsonObject(actual, `${context}: providers[${index}]`)
    const id = requireNonEmptyString(actual.id, `${context}: providers[${index}].id`)
    const expected = expectedById.get(id.toLowerCase())
    assert(expected, `${context}: unexpected provider id ${id}`)
    assert(!seen.has(id.toLowerCase()), `${context}: duplicate provider id ${id}`)
    seen.add(id.toLowerCase())

    const name = requireNonEmptyString(actual.name, `${context}: providers[${index}].name`)
    assert(name === expected.name, `${context}: provider ${id} name mismatch`) 

    const baseUrl = normalizeOptionalString(actual.baseUrl, `${context}: providers[${index}].baseUrl`)
    const apiMode = normalizeOptionalString(actual.apiMode, `${context}: providers[${index}].apiMode`)
    assert(baseUrl === expected.baseUrl, `${context}: provider ${id} baseUrl mismatch`) 
    assert(apiMode === expected.apiMode, `${context}: provider ${id} apiMode mismatch`) 

    assert(typeof actual.hasApiKey === 'boolean', `${context}: providers[${index}].hasApiKey must be boolean`)
    assert(actual.hasApiKey === false, `${context}: provider ${id} hasApiKey should be false`)

    assert(typeof actual.apiKey === 'string', `${context}: providers[${index}].apiKey must be string`)
    assert(actual.apiKey === '', `${context}: provider ${id} apiKey should be empty string`)
  }

  for (const expected of expectedProviders) {
    assert(seen.has(expected.id.toLowerCase()), `${context}: missing provider ${expected.id}`)
  }
}

function summarizeModelGroups(payload) {
  return MODEL_GROUP_KEYS.map((key) => `${key}=${payload[key].length}`).join(', ')
}

function escapeMarkdownCell(value) {
  return String(value).replace(/\|/g, '\\|').replace(/\n/g, '<br/>')
}

function renderMarkdownReport(report) {
  const passed = report.steps.filter((step) => step.status === 'PASS').length
  const failed = report.steps.filter((step) => step.status === 'FAIL').length

  const lines = []
  lines.push('# E2E User Config Report')
  lines.push('')
  lines.push(`- Generated at: ${report.generatedAt}`)
  lines.push(`- Base URL: ${report.baseUrl}`)
  lines.push(`- Timeout(ms): ${report.timeoutMs}`)
  lines.push(`- Test User: ${report.testUser || 'N/A'}`)
  lines.push(`- Result: ${report.success ? 'PASS' : 'FAIL'}`)
  lines.push(`- Steps: ${report.steps.length}`)
  lines.push(`- Pass: ${passed}`)
  lines.push(`- Fail: ${failed}`)
  lines.push('')
  lines.push('| Step | Name | Result | Elapsed(ms) | Detail |')
  lines.push('|---|---|---|---:|---|')

  for (const step of report.steps) {
    lines.push(
      `| ${escapeMarkdownCell(step.id)} | ${escapeMarkdownCell(step.name)} | ${step.status} | ${step.elapsedMs} | ${escapeMarkdownCell(step.detail)} |`,
    )
  }

  if (report.error) {
    lines.push('')
    lines.push('## Error')
    lines.push('')
    lines.push(`- ${report.error}`)
  }

  lines.push('')
  return `${lines.join('\n')}\n`
}

function writeReports(outDir, report) {
  fs.mkdirSync(outDir, { recursive: true })
  const jsonPath = path.join(outDir, 'e2e-user-config.json')
  const mdPath = path.join(outDir, 'e2e-user-config.md')

  fs.writeFileSync(jsonPath, JSON.stringify(report, null, 2), 'utf8')
  fs.writeFileSync(mdPath, renderMarkdownReport(report), 'utf8')

  return { jsonPath, mdPath }
}

async function runStep(report, id, name, handler) {
  const startedAt = Date.now()
  const entry = {
    id,
    name,
    status: 'PASS',
    elapsedMs: 0,
    detail: 'ok',
  }

  try {
    const detail = await handler()
    if (typeof detail === 'string' && detail.trim().length > 0) {
      entry.detail = detail
    }
    report.steps.push(entry)
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    entry.status = 'FAIL'
    entry.detail = message
    report.steps.push(entry)
    report.error = `${id}: ${message}`
    throw error
  } finally {
    entry.elapsedMs = Date.now() - startedAt
  }
}

async function main() {
  const args = parseArgs(process.argv.slice(2))
  const baseUrl = normalizeBaseUrl(args.base || args['rust-base'], 'base')
  const timeoutMs = parseOptionalInt(args['timeout-ms'], 'timeout-ms', DEFAULT_TIMEOUT_MS)
  const outDir = path.resolve(args['out-dir'] || DEFAULT_OUT_DIR)

  if (args.dryRun) {
    console.log(`[dry-run] base: ${baseUrl}`)
    console.log(`[dry-run] timeout-ms: ${timeoutMs}`)
    console.log(`[dry-run] out-dir: ${outDir}`)
    console.log(`[dry-run] steps: ${STEP_PLAN.length}`)
    for (let index = 0; index < STEP_PLAN.length; index += 1) {
      console.log(`- ${index + 1}. ${STEP_PLAN[index]}`)
    }
    return
  }

  const userSeed = `${Date.now()}-${randomUUID().slice(0, 8)}`
  const username = `e2e-user-${userSeed}`
  const password = `E2E-${userSeed}-pass`

  const providersUpdatePayload = [
    {
      id: `e2e-openai-${userSeed}`,
      name: 'E2E OpenAI Provider',
      baseUrl: 'https://example.openai.local/v1',
      apiMode: 'openai-official',
    },
    {
      id: `e2e-gemini-${userSeed}`,
      name: 'E2E Gemini Provider',
      baseUrl: 'https://example.gemini.local/v1',
      apiMode: 'gemini-sdk',
    },
  ]

  const report = {
    generatedAt: new Date().toISOString(),
    success: false,
    baseUrl,
    timeoutMs,
    testUser: username,
    steps: [],
    error: null,
  }

  let token = ''
  let userId = ''
  let preferenceId = ''
  let modelsPayload = null

  try {
    await runStep(report, 'step-01-register', 'Register user and obtain token', async () => {
      const response = await requestJson({
        baseUrl,
        requestPath: '/api/auth/register',
        method: 'POST',
        body: {
          username,
          name: username,
          password,
        },
        timeoutMs,
      })

      expectStatus(response, 201, 'register')
      assertJsonObject(response.json, 'register response')
      token = requireNonEmptyString(response.json.token, 'register response token')

      assertJsonObject(response.json.user, 'register response user')
      userId = requireNonEmptyString(response.json.user.id, 'register response user.id')
      const returnedName = requireNonEmptyString(response.json.user.name, 'register response user.name')
      assert(returnedName === username, 'register response user.name mismatch')
      requireNonEmptyString(response.json.user.role, 'register response user.role')

      return `status=201 userId=${userId}`
    })

    await runStep(report, 'step-02-preference-defaults', 'GET /api/user-preference defaults', async () => {
      const response = await requestJson({
        baseUrl,
        requestPath: '/api/user-preference',
        method: 'GET',
        token,
        timeoutMs,
      })

      expectStatus(response, 200, 'get user-preference')
      assertJsonObject(response.json, 'get user-preference response')
      assertJsonObject(response.json.preference, 'get user-preference response.preference')

      const preference = response.json.preference
      verifyPreferenceDefaults(preference, 'default preference check')
      preferenceId = requireNonEmptyString(preference.id, 'default preference id')
      assert(preference.userId === userId, 'default preference userId mismatch')

      return 'videoRatio=9:16 artStyle=american-comic ttsRate=+50%'
    })

    await runStep(
      report,
      'step-03-preference-patch-analysis-video-ratio',
      'PATCH /api/user-preference analysisModel + videoRatio',
      async () => {
        const response = await requestJson({
          baseUrl,
          requestPath: '/api/user-preference',
          method: 'PATCH',
          token,
          body: {
            analysisModel: 'google/gemini-3-pro-preview',
            videoRatio: '16:9',
          },
          timeoutMs,
        })

        expectStatus(response, 200, 'patch user-preference analysis/videoRatio')
        assertJsonObject(response.json, 'patch user-preference response')
        assertJsonObject(response.json.preference, 'patch user-preference response.preference')

        const preference = response.json.preference
        assert(preference.id === preferenceId, 'patch response preference id changed unexpectedly')
        assert(
          preference.analysisModel === 'google/gemini-3-pro-preview',
          'analysisModel was not updated to google/gemini-3-pro-preview',
        )
        assert(preference.videoRatio === '16:9', 'videoRatio was not updated to 16:9')

        return 'analysisModel=google/gemini-3-pro-preview videoRatio=16:9'
      },
    )

    await runStep(report, 'step-04-preference-readback-1', 'GET /api/user-preference readback #1', async () => {
      const response = await requestJson({
        baseUrl,
        requestPath: '/api/user-preference',
        method: 'GET',
        token,
        timeoutMs,
      })

      expectStatus(response, 200, 'readback user-preference #1')
      assertJsonObject(response.json, 'readback user-preference #1 response')
      assertJsonObject(response.json.preference, 'readback user-preference #1 response.preference')

      const preference = response.json.preference
      assert(preference.analysisModel === 'google/gemini-3-pro-preview', 'readback #1 analysisModel mismatch')
      assert(preference.videoRatio === '16:9', 'readback #1 videoRatio mismatch')

      return 'analysisModel and videoRatio persisted'
    })

    await runStep(
      report,
      'step-05-preference-patch-art-style-tts',
      'PATCH /api/user-preference artStyle + ttsRate',
      async () => {
        const response = await requestJson({
          baseUrl,
          requestPath: '/api/user-preference',
          method: 'PATCH',
          token,
          body: {
            artStyle: 'realistic',
            ttsRate: '+100%',
          },
          timeoutMs,
        })

        expectStatus(response, 200, 'patch user-preference artStyle/ttsRate')
        assertJsonObject(response.json, 'patch user-preference #2 response')
        assertJsonObject(response.json.preference, 'patch user-preference #2 response.preference')

        const preference = response.json.preference
        assert(preference.artStyle === 'realistic', 'artStyle was not updated to realistic')
        assert(preference.ttsRate === '+100%', 'ttsRate was not updated to +100%')
        assert(preference.analysisModel === 'google/gemini-3-pro-preview', 'analysisModel changed unexpectedly')

        return 'artStyle=realistic ttsRate=+100%'
      },
    )

    await runStep(
      report,
      'step-06-preference-empty-patch-400',
      'PATCH /api/user-preference with empty object should fail',
      async () => {
        const response = await requestJson({
          baseUrl,
          requestPath: '/api/user-preference',
          method: 'PATCH',
          token,
          body: {},
          timeoutMs,
        })

        expectStatus(response, 400, 'patch user-preference empty object')
        const message = readErrorMessage(response.json, 'patch user-preference empty object')
        assert(
          message === 'no allowed fields to update',
          `unexpected empty-patch error message: ${message}`,
        )

        return 'status=400 message=no allowed fields to update'
      },
    )

    await runStep(report, 'step-07-user-models-shape', 'GET /api/user/models group arrays', async () => {
      const response = await requestJson({
        baseUrl,
        requestPath: '/api/user/models',
        method: 'GET',
        token,
        timeoutMs,
      })

      expectStatus(response, 200, 'get user/models')
      verifyModelGroupsShape(response.json, 'user/models payload')
      modelsPayload = response.json

      return summarizeModelGroups(response.json)
    })

    await runStep(report, 'step-08-user-models-items', 'Verify /api/user/models item fields', async () => {
      assertJsonObject(modelsPayload, 'saved user/models payload')
      verifyModelItems(modelsPayload, 'user/models payload item checks')

      return 'all model entries expose value/label/provider'
    })

    await runStep(report, 'step-09-api-config-read', 'GET /api/user/api-config', async () => {
      const response = await requestJson({
        baseUrl,
        requestPath: '/api/user/api-config',
        method: 'GET',
        token,
        timeoutMs,
      })

      expectStatus(response, 200, 'get user/api-config')
      verifyApiConfigShape(response.json, 'api-config get payload')

      return `models=${response.json.models.length} providers=${response.json.providers.length}`
    })

    await runStep(report, 'step-10-api-config-update-providers', 'PUT /api/user/api-config providers', async () => {
      const response = await requestJson({
        baseUrl,
        requestPath: '/api/user/api-config',
        method: 'PUT',
        token,
        body: {
          providers: providersUpdatePayload,
        },
        timeoutMs,
      })

      expectStatus(response, 200, 'put user/api-config providers')
      verifyApiConfigShape(response.json, 'api-config put payload')
      verifyProviders(
        response.json.providers,
        providersUpdatePayload,
        'api-config put providers verification',
      )

      return providersUpdatePayload.map((provider) => provider.id).join(', ')
    })

    await runStep(
      report,
      'step-11-api-config-readback-providers',
      'GET /api/user/api-config readback providers',
      async () => {
        const response = await requestJson({
          baseUrl,
          requestPath: '/api/user/api-config',
          method: 'GET',
          token,
          timeoutMs,
        })

        expectStatus(response, 200, 'get user/api-config readback')
        verifyApiConfigShape(response.json, 'api-config readback payload')
        verifyProviders(
          response.json.providers,
          providersUpdatePayload,
          'api-config readback providers verification',
        )

        return `providers persisted: ${providersUpdatePayload.length}`
      },
    )

    await runStep(
      report,
      'step-12-test-connection-missing-api-key',
      'POST /api/user/api-config/test-connection without apiKey',
      async () => {
        const response = await requestJson({
          baseUrl,
          requestPath: '/api/user/api-config/test-connection',
          method: 'POST',
          token,
          body: {
            provider: 'openai',
          },
          timeoutMs,
        })

        expectStatus(response, 400, 'test-connection without apiKey')
        const message = readErrorMessage(response.json, 'test-connection without apiKey')
        assert(
          message === 'missing required field apiKey',
          `unexpected missing-apiKey message: ${message}`,
        )

        return 'status=400 message=missing required field apiKey'
      },
    )

    await runStep(
      report,
      'step-13-test-connection-invalid-provider',
      'POST /api/user/api-config/test-connection invalid provider',
      async () => {
        const response = await requestJson({
          baseUrl,
          requestPath: '/api/user/api-config/test-connection',
          method: 'POST',
          token,
          body: {
            provider: 'invalid-provider',
            apiKey: 'invalid-key',
          },
          timeoutMs,
        })

        expectStatus(response, 400, 'test-connection invalid provider')
        const message = readErrorMessage(response.json, 'test-connection invalid provider')
        assert(
          message === 'unsupported provider: invalid-provider',
          `unexpected invalid-provider message: ${message}`,
        )

        return 'status=400 message=unsupported provider: invalid-provider'
      },
    )

    report.success = true
  } catch {
    report.success = false
  }

  const { jsonPath, mdPath } = writeReports(outDir, report)
  console.log(`wrote ${jsonPath}`)
  console.log(`wrote ${mdPath}`)
  console.log(`RESULT=${report.success ? 'PASS' : 'FAIL'}`)

  if (!report.success) {
    process.exitCode = 2
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error))
  process.exitCode = 1
})
