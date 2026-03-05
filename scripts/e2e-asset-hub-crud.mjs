#!/usr/bin/env node

import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'

const SCRIPT_DIR = path.dirname(new URL(import.meta.url).pathname)
const PROJECT_ROOT = path.resolve(SCRIPT_DIR, '..')
const DEFAULT_OUT_DIR = path.join(PROJECT_ROOT, 'reports')
const DEFAULT_REPORT_PATH = path.join(DEFAULT_OUT_DIR, 'e2e-asset-hub-crud.md')
const DEFAULT_TIMEOUT_MS = 20000

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

function toErrorMessage(error) {
  return error instanceof Error ? error.message : String(error)
}

function isObject(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
}

function asObject(value, label) {
  assert(value !== null && typeof value === 'object' && !Array.isArray(value), `${label} must be a JSON object`)
  return value
}

function asArray(value, label) {
  assert(Array.isArray(value), `${label} must be an array`)
  return value
}

function asNonEmptyString(value, label) {
  assert(typeof value === 'string' && value.trim().length > 0, `${label} must be a non-empty string`)
  return value
}

function collectSnakeCaseKeyPaths(value, pathLabel, paths) {
  if (Array.isArray(value)) {
    for (let index = 0; index < value.length; index += 1) {
      collectSnakeCaseKeyPaths(value[index], `${pathLabel}[${index}]`, paths)
    }
    return
  }

  if (!isObject(value)) {
    return
  }

  for (const [key, nestedValue] of Object.entries(value)) {
    const keyPath = `${pathLabel}.${key}`
    if (key.includes('_')) {
      paths.push(keyPath)
    }
    collectSnakeCaseKeyPaths(nestedValue, keyPath, paths)
  }
}

function assertNoSnakeCaseKeys(value, label) {
  const snakeCasePaths = []
  collectSnakeCaseKeyPaths(value, label, snakeCasePaths)
  assert(
    snakeCasePaths.length === 0,
    `${label} must not include snake_case keys: ${snakeCasePaths.join(', ')}`,
  )
}

function findObjectById(items, id, label) {
  const match = items.find((item) => item && typeof item === 'object' && !Array.isArray(item) && item.id === id)
  assert(match, `${label}: id=${id} not found`)
  return match
}

function assertStatus(stepId, actualStatus, expectedStatus) {
  const expected = Array.isArray(expectedStatus) ? expectedStatus : [expectedStatus]
  assert(
    expected.includes(actualStatus),
    `${stepId}: expected HTTP ${expected.join(' or ')}, got ${actualStatus}`,
  )
}

function expectJsonObject(stepId, response, expectedStatus) {
  assertStatus(stepId, response.status, expectedStatus)
  assert(response.parseError === null, `${stepId}: ${response.parseError}`)
  const body = asObject(response.json, `${stepId}: response body`)
  assertNoSnakeCaseKeys(body, `${stepId}: response body`)
  return body
}

function assertSuccess(stepId, payload) {
  assert(payload.success === true, `${stepId}: expected success=true`)
}

function sanitizeMarkdownCell(value) {
  return String(value ?? '')
    .replace(/\|/g, '\\|')
    .replace(/\n/g, '<br>')
}

async function runRequest({
  baseUrl,
  method,
  path: pathName,
  token,
  body,
  timeoutMs,
}) {
  const url = new URL(pathName, baseUrl).toString()
  const headers = {
    Accept: 'application/json',
  }

  if (token) {
    headers.Authorization = `Bearer ${token}`
  }
  if (body !== undefined) {
    headers['Content-Type'] = 'application/json'
  }

  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), timeoutMs)

  try {
    const response = await fetch(url, {
      method,
      headers,
      body: body !== undefined ? JSON.stringify(body) : undefined,
      signal: controller.signal,
    })

    const rawBody = await response.text()
    let parseError = null
    let json = null

    if (rawBody.length > 0) {
      const parsed = safeJsonParse(rawBody)
      if (parsed.ok) {
        json = parsed.value
      } else {
        parseError = 'response body is not valid JSON'
      }
    }

    return {
      url,
      status: response.status,
      rawBody,
      parseError,
      json,
    }
  } catch (error) {
    throw new Error(`${method} ${pathName} request failed: ${toErrorMessage(error)}`)
  } finally {
    clearTimeout(timer)
  }
}

async function runStep(rows, step, execute) {
  const startedAt = Date.now()
  try {
    const result = await execute()
    rows.push({
      id: step.id,
      name: step.name,
      method: step.method,
      path: result.path ?? step.path,
      status: 'PASS',
      httpStatus: result.httpStatus ?? '',
      detail: result.detail ?? 'ok',
      elapsedMs: Date.now() - startedAt,
    })
  } catch (error) {
    rows.push({
      id: step.id,
      name: step.name,
      method: step.method,
      path: step.path,
      status: 'FAIL',
      httpStatus: '',
      detail: toErrorMessage(error),
      elapsedMs: Date.now() - startedAt,
    })
  }
}

function renderMarkdownReport({ baseUrl, username, rows }) {
  const summary = {
    total: rows.length,
    pass: rows.filter((row) => row.status === 'PASS').length,
    fail: rows.filter((row) => row.status === 'FAIL').length,
  }

  const lines = []
  lines.push('# Asset Hub CRUD E2E Report')
  lines.push('')
  lines.push(`- Generated at: ${new Date().toISOString()}`)
  lines.push(`- Base URL: ${baseUrl}`)
  lines.push(`- Test User: ${username}`)
  lines.push(`- Total: ${summary.total}`)
  lines.push(`- Pass: ${summary.pass}`)
  lines.push(`- Fail: ${summary.fail}`)
  lines.push('')
  lines.push('| Step | Name | Method | Path | Result | HTTP | Detail | Elapsed(ms) |')
  lines.push('|---|---|---|---|---|---|---|---|')

  for (const row of rows) {
    lines.push(
      `| ${sanitizeMarkdownCell(row.id)} | ${sanitizeMarkdownCell(row.name)} | ${sanitizeMarkdownCell(row.method)} | ${sanitizeMarkdownCell(row.path)} | ${sanitizeMarkdownCell(row.status)} | ${sanitizeMarkdownCell(row.httpStatus)} | ${sanitizeMarkdownCell(row.detail)} | ${sanitizeMarkdownCell(row.elapsedMs)} |`,
    )
  }

  lines.push('')
  lines.push('## Failures')
  lines.push('')

  const failed = rows.filter((row) => row.status === 'FAIL')
  if (failed.length === 0) {
    lines.push('- none')
  } else {
    for (const row of failed) {
      lines.push(`- ${row.id} (${row.method} ${row.path}): ${row.detail}`)
    }
  }

  lines.push('')
  return `${lines.join('\n')}\n`
}

function writeReport(reportPath, content) {
  fs.mkdirSync(path.dirname(reportPath), { recursive: true })
  fs.writeFileSync(reportPath, content, 'utf8')
}

async function main() {
  const args = parseArgs(process.argv.slice(2))
  const baseUrl = normalizeBaseUrl(args.base, 'base')
  const timeoutMs = parseOptionalInt(args['timeout-ms'], 'timeout-ms') ?? DEFAULT_TIMEOUT_MS
  const reportPath = path.resolve(args.report || DEFAULT_REPORT_PATH)

  const timestamp = Date.now()
  const username = `e2e-asset-crud-${timestamp}@test.com`
  const password = `E2E-pass-${timestamp}`
  const tokenState = {
    token: '',
  }

  const state = {
    folderId: '',
    characterId: '',
    locationId: '',
    voiceId: '',
  }

  if (args.dryRun) {
    console.log(`[dry-run] base: ${baseUrl}`)
    console.log(`[dry-run] timeoutMs: ${timeoutMs}`)
    console.log(`[dry-run] report: ${reportPath}`)
    console.log(`[dry-run] username: ${username}`)
    return
  }

  const rows = []

  const requireToken = () => {
    return asNonEmptyString(tokenState.token, 'auth token')
  }

  await runStep(
    rows,
    {
      id: '00',
      name: 'Register test user and get token',
      method: 'POST',
      path: '/api/auth/register',
    },
    async () => {
      const response = await runRequest({
        baseUrl,
        method: 'POST',
        path: '/api/auth/register',
        body: {
          username,
          name: username,
          password,
        },
        timeoutMs,
      })

      const payload = expectJsonObject('00', response, [200, 201])
      tokenState.token = asNonEmptyString(payload.token, '00 token')

      return {
        httpStatus: response.status,
        detail: 'token acquired',
      }
    },
  )

  await runStep(
    rows,
    {
      id: '01',
      name: 'Create folder',
      method: 'POST',
      path: '/api/asset-hub/folders',
    },
    async () => {
      const response = await runRequest({
        baseUrl,
        method: 'POST',
        path: '/api/asset-hub/folders',
        token: requireToken(),
        body: { name: 'E2E Folder' },
        timeoutMs,
      })

      const payload = expectJsonObject('01', response, 200)
      assertSuccess('01', payload)
      const folder = asObject(payload.folder, '01 folder')
      state.folderId = asNonEmptyString(folder.id, '01 folder.id')
      assert(folder.name === 'E2E Folder', `01: expected folder name E2E Folder, got ${folder.name}`)

      return {
        httpStatus: response.status,
        detail: `folderId=${state.folderId}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: '02',
      name: 'List folders and verify created item',
      method: 'GET',
      path: '/api/asset-hub/folders',
    },
    async () => {
      const folderId = asNonEmptyString(state.folderId, '02 folderId')
      const response = await runRequest({
        baseUrl,
        method: 'GET',
        path: '/api/asset-hub/folders',
        token: requireToken(),
        timeoutMs,
      })

      const payload = expectJsonObject('02', response, 200)
      const folders = asArray(payload.folders, '02 folders')
      const folder = findObjectById(folders, folderId, '02 folders')
      assert(folder.name === 'E2E Folder', `02: expected listed folder name E2E Folder, got ${folder.name}`)

      return {
        httpStatus: response.status,
        detail: `verified folderId=${folderId}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: '03',
      name: 'Update folder name',
      method: 'PATCH',
      path: '/api/asset-hub/folders/{folderId}',
    },
    async () => {
      const folderId = asNonEmptyString(state.folderId, '03 folderId')
      const pathName = `/api/asset-hub/folders/${encodeURIComponent(folderId)}`
      const response = await runRequest({
        baseUrl,
        method: 'PATCH',
        path: pathName,
        token: requireToken(),
        body: { name: 'Renamed' },
        timeoutMs,
      })

      const payload = expectJsonObject('03', response, 200)
      assertSuccess('03', payload)
      const folder = asObject(payload.folder, '03 folder')
      assert(folder.id === folderId, `03: expected folder.id=${folderId}, got ${folder.id}`)
      assert(folder.name === 'Renamed', `03: expected folder name Renamed, got ${folder.name}`)

      return {
        path: pathName,
        httpStatus: response.status,
        detail: `updated folderId=${folderId}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: '04',
      name: 'Delete folder',
      method: 'DELETE',
      path: '/api/asset-hub/folders/{folderId}',
    },
    async () => {
      const folderId = asNonEmptyString(state.folderId, '04 folderId')
      const pathName = `/api/asset-hub/folders/${encodeURIComponent(folderId)}`
      const response = await runRequest({
        baseUrl,
        method: 'DELETE',
        path: pathName,
        token: requireToken(),
        timeoutMs,
      })

      const payload = expectJsonObject('04', response, 200)
      assertSuccess('04', payload)

      return {
        path: pathName,
        httpStatus: response.status,
        detail: `deleted folderId=${folderId}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: '05',
      name: 'Create character',
      method: 'POST',
      path: '/api/asset-hub/characters',
    },
    async () => {
      const response = await runRequest({
        baseUrl,
        method: 'POST',
        path: '/api/asset-hub/characters',
        token: requireToken(),
        body: {
          name: 'E2E Hero',
          folderId: null,
        },
        timeoutMs,
      })

      const payload = expectJsonObject('05', response, 200)
      assertSuccess('05', payload)
      const character = asObject(payload.character, '05 character')
      state.characterId = asNonEmptyString(character.id, '05 character.id')
      assert(character.name === 'E2E Hero', `05: expected character name E2E Hero, got ${character.name}`)
      assert(character.folderId === null, `05: expected character.folderId=null, got ${character.folderId}`)
      const appearances = asArray(character.appearances, '05 character.appearances')
      const defaultAppearance = appearances.find(
        (item) => item && typeof item === 'object' && !Array.isArray(item) && item.appearanceIndex === 0,
      )
      assert(defaultAppearance, '05: expected default appearance with appearanceIndex=0')

      return {
        httpStatus: response.status,
        detail: `characterId=${state.characterId}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: '06',
      name: 'List characters and verify created item',
      method: 'GET',
      path: '/api/asset-hub/characters',
    },
    async () => {
      const characterId = asNonEmptyString(state.characterId, '06 characterId')
      const response = await runRequest({
        baseUrl,
        method: 'GET',
        path: '/api/asset-hub/characters',
        token: requireToken(),
        timeoutMs,
      })

      const payload = expectJsonObject('06', response, 200)
      const characters = asArray(payload.characters, '06 characters')
      const character = findObjectById(characters, characterId, '06 characters')
      assert(character.name === 'E2E Hero', `06: expected listed character name E2E Hero, got ${character.name}`)

      return {
        httpStatus: response.status,
        detail: `verified characterId=${characterId}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: '07',
      name: 'Update character name',
      method: 'PATCH',
      path: '/api/asset-hub/characters/{characterId}',
    },
    async () => {
      const characterId = asNonEmptyString(state.characterId, '07 characterId')
      const pathName = `/api/asset-hub/characters/${encodeURIComponent(characterId)}`
      const response = await runRequest({
        baseUrl,
        method: 'PATCH',
        path: pathName,
        token: requireToken(),
        body: {
          name: 'Updated Hero',
        },
        timeoutMs,
      })

      const payload = expectJsonObject('07', response, 200)
      assertSuccess('07', payload)
      const character = asObject(payload.character, '07 character')
      assert(character.id === characterId, `07: expected character.id=${characterId}, got ${character.id}`)
      assert(character.name === 'Updated Hero', `07: expected character name Updated Hero, got ${character.name}`)

      return {
        path: pathName,
        httpStatus: response.status,
        detail: `updated characterId=${characterId}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: '08',
      name: 'Update character appearance description',
      method: 'PATCH',
      path: '/api/asset-hub/characters/{characterId}/appearances/0',
    },
    async () => {
      const characterId = asNonEmptyString(state.characterId, '08 characterId')
      const pathName = `/api/asset-hub/characters/${encodeURIComponent(characterId)}/appearances/0`
      const patchResponse = await runRequest({
        baseUrl,
        method: 'PATCH',
        path: pathName,
        token: requireToken(),
        body: {
          description: 'test desc',
        },
        timeoutMs,
      })

      const patchPayload = expectJsonObject('08', patchResponse, 200)
      assertSuccess('08', patchPayload)

      const detailResponse = await runRequest({
        baseUrl,
        method: 'GET',
        path: `/api/asset-hub/characters/${encodeURIComponent(characterId)}`,
        token: requireToken(),
        timeoutMs,
      })
      const detailPayload = expectJsonObject('08.detail', detailResponse, 200)
      const character = asObject(detailPayload.character, '08.detail character')
      const appearances = asArray(character.appearances, '08.detail character.appearances')
      const appearance = appearances.find(
        (item) => item && typeof item === 'object' && !Array.isArray(item) && item.appearanceIndex === 0,
      )
      assert(appearance, '08.detail: expected appearanceIndex=0 to exist')
      assert(appearance.description === 'test desc', `08.detail: expected appearance description test desc, got ${appearance.description}`)

      return {
        path: pathName,
        httpStatus: patchResponse.status,
        detail: `updated appearance for characterId=${characterId}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: '09',
      name: 'Delete character',
      method: 'DELETE',
      path: '/api/asset-hub/characters/{characterId}',
    },
    async () => {
      const characterId = asNonEmptyString(state.characterId, '09 characterId')
      const pathName = `/api/asset-hub/characters/${encodeURIComponent(characterId)}`
      const deleteResponse = await runRequest({
        baseUrl,
        method: 'DELETE',
        path: pathName,
        token: requireToken(),
        timeoutMs,
      })
      const deletePayload = expectJsonObject('09', deleteResponse, 200)
      assertSuccess('09', deletePayload)

      const listResponse = await runRequest({
        baseUrl,
        method: 'GET',
        path: '/api/asset-hub/characters',
        token: requireToken(),
        timeoutMs,
      })
      const listPayload = expectJsonObject('09.list', listResponse, 200)
      const characters = asArray(listPayload.characters, '09.list characters')
      const stillExists = characters.some(
        (item) => item && typeof item === 'object' && !Array.isArray(item) && item.id === characterId,
      )
      assert(!stillExists, `09.list: characterId=${characterId} should be deleted`)

      return {
        path: pathName,
        httpStatus: deleteResponse.status,
        detail: `deleted characterId=${characterId}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: '10',
      name: 'Create location',
      method: 'POST',
      path: '/api/asset-hub/locations',
    },
    async () => {
      const response = await runRequest({
        baseUrl,
        method: 'POST',
        path: '/api/asset-hub/locations',
        token: requireToken(),
        body: {
          name: 'E2E Location',
        },
        timeoutMs,
      })

      const payload = expectJsonObject('10', response, 200)
      assertSuccess('10', payload)
      const location = asObject(payload.location, '10 location')
      state.locationId = asNonEmptyString(location.id, '10 location.id')
      assert(location.name === 'E2E Location', `10: expected location name E2E Location, got ${location.name}`)
      asArray(location.images, '10 location.images')

      return {
        httpStatus: response.status,
        detail: `locationId=${state.locationId}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: '11',
      name: 'List locations and verify created item',
      method: 'GET',
      path: '/api/asset-hub/locations',
    },
    async () => {
      const locationId = asNonEmptyString(state.locationId, '11 locationId')
      const response = await runRequest({
        baseUrl,
        method: 'GET',
        path: '/api/asset-hub/locations',
        token: requireToken(),
        timeoutMs,
      })

      const payload = expectJsonObject('11', response, 200)
      const locations = asArray(payload.locations, '11 locations')
      const location = findObjectById(locations, locationId, '11 locations')
      assert(location.name === 'E2E Location', `11: expected listed location name E2E Location, got ${location.name}`)

      return {
        httpStatus: response.status,
        detail: `verified locationId=${locationId}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: '12',
      name: 'Update location name',
      method: 'PATCH',
      path: '/api/asset-hub/locations/{locationId}',
    },
    async () => {
      const locationId = asNonEmptyString(state.locationId, '12 locationId')
      const pathName = `/api/asset-hub/locations/${encodeURIComponent(locationId)}`
      const response = await runRequest({
        baseUrl,
        method: 'PATCH',
        path: pathName,
        token: requireToken(),
        body: {
          name: 'Updated Location',
        },
        timeoutMs,
      })

      const payload = expectJsonObject('12', response, 200)
      assertSuccess('12', payload)
      const location = asObject(payload.location, '12 location')
      assert(location.id === locationId, `12: expected location.id=${locationId}, got ${location.id}`)
      assert(location.name === 'Updated Location', `12: expected location name Updated Location, got ${location.name}`)

      return {
        path: pathName,
        httpStatus: response.status,
        detail: `updated locationId=${locationId}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: '13',
      name: 'Delete location',
      method: 'DELETE',
      path: '/api/asset-hub/locations/{locationId}',
    },
    async () => {
      const locationId = asNonEmptyString(state.locationId, '13 locationId')
      const pathName = `/api/asset-hub/locations/${encodeURIComponent(locationId)}`
      const deleteResponse = await runRequest({
        baseUrl,
        method: 'DELETE',
        path: pathName,
        token: requireToken(),
        timeoutMs,
      })

      const deletePayload = expectJsonObject('13', deleteResponse, 200)
      assertSuccess('13', deletePayload)

      const listResponse = await runRequest({
        baseUrl,
        method: 'GET',
        path: '/api/asset-hub/locations',
        token: requireToken(),
        timeoutMs,
      })
      const listPayload = expectJsonObject('13.list', listResponse, 200)
      const locations = asArray(listPayload.locations, '13.list locations')
      const stillExists = locations.some(
        (item) => item && typeof item === 'object' && !Array.isArray(item) && item.id === locationId,
      )
      assert(!stillExists, `13.list: locationId=${locationId} should be deleted`)

      return {
        path: pathName,
        httpStatus: deleteResponse.status,
        detail: `deleted locationId=${locationId}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: '14',
      name: 'Create voice',
      method: 'POST',
      path: '/api/asset-hub/voices',
    },
    async () => {
      const response = await runRequest({
        baseUrl,
        method: 'POST',
        path: '/api/asset-hub/voices',
        token: requireToken(),
        body: {
          name: 'E2E Voice',
          voiceType: 'edge-tts',
        },
        timeoutMs,
      })

      const payload = expectJsonObject('14', response, 200)
      assertSuccess('14', payload)
      const voice = asObject(payload.voice, '14 voice')
      state.voiceId = asNonEmptyString(voice.id, '14 voice.id')
      assert(voice.name === 'E2E Voice', `14: expected voice name E2E Voice, got ${voice.name}`)
      assert(voice.voiceType === 'edge-tts', `14: expected voiceType edge-tts, got ${voice.voiceType}`)

      return {
        httpStatus: response.status,
        detail: `voiceId=${state.voiceId}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: '15',
      name: 'List voices and verify created item',
      method: 'GET',
      path: '/api/asset-hub/voices',
    },
    async () => {
      const voiceId = asNonEmptyString(state.voiceId, '15 voiceId')
      const response = await runRequest({
        baseUrl,
        method: 'GET',
        path: '/api/asset-hub/voices',
        token: requireToken(),
        timeoutMs,
      })

      const payload = expectJsonObject('15', response, 200)
      const voices = asArray(payload.voices, '15 voices')
      const voice = findObjectById(voices, voiceId, '15 voices')
      assert(voice.name === 'E2E Voice', `15: expected listed voice name E2E Voice, got ${voice.name}`)

      return {
        httpStatus: response.status,
        detail: `verified voiceId=${voiceId}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: '16',
      name: 'Update voice name',
      method: 'PATCH',
      path: '/api/asset-hub/voices/{voiceId}',
    },
    async () => {
      const voiceId = asNonEmptyString(state.voiceId, '16 voiceId')
      const pathName = `/api/asset-hub/voices/${encodeURIComponent(voiceId)}`
      const response = await runRequest({
        baseUrl,
        method: 'PATCH',
        path: pathName,
        token: requireToken(),
        body: {
          name: 'Updated Voice',
        },
        timeoutMs,
      })

      const payload = expectJsonObject('16', response, 200)
      assertSuccess('16', payload)
      const voice = asObject(payload.voice, '16 voice')
      assert(voice.id === voiceId, `16: expected voice.id=${voiceId}, got ${voice.id}`)
      assert(voice.name === 'Updated Voice', `16: expected voice name Updated Voice, got ${voice.name}`)

      return {
        path: pathName,
        httpStatus: response.status,
        detail: `updated voiceId=${voiceId}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: '17',
      name: 'Delete voice',
      method: 'DELETE',
      path: '/api/asset-hub/voices/{voiceId}',
    },
    async () => {
      const voiceId = asNonEmptyString(state.voiceId, '17 voiceId')
      const pathName = `/api/asset-hub/voices/${encodeURIComponent(voiceId)}`
      const deleteResponse = await runRequest({
        baseUrl,
        method: 'DELETE',
        path: pathName,
        token: requireToken(),
        timeoutMs,
      })

      const deletePayload = expectJsonObject('17', deleteResponse, 200)
      assertSuccess('17', deletePayload)

      const listResponse = await runRequest({
        baseUrl,
        method: 'GET',
        path: '/api/asset-hub/voices',
        token: requireToken(),
        timeoutMs,
      })
      const listPayload = expectJsonObject('17.list', listResponse, 200)
      const voices = asArray(listPayload.voices, '17.list voices')
      const stillExists = voices.some(
        (item) => item && typeof item === 'object' && !Array.isArray(item) && item.id === voiceId,
      )
      assert(!stillExists, `17.list: voiceId=${voiceId} should be deleted`)

      return {
        path: pathName,
        httpStatus: deleteResponse.status,
        detail: `deleted voiceId=${voiceId}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: '18',
      name: 'Picker API character type',
      method: 'GET',
      path: '/api/asset-hub/picker?type=character',
    },
    async () => {
      const response = await runRequest({
        baseUrl,
        method: 'GET',
        path: '/api/asset-hub/picker?type=character',
        token: requireToken(),
        timeoutMs,
      })

      const payload = expectJsonObject('18', response, 200)
      const characters = asArray(payload.characters, '18 characters')

      return {
        httpStatus: response.status,
        detail: `characters=${characters.length}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: '19',
      name: 'Picker API location type',
      method: 'GET',
      path: '/api/asset-hub/picker?type=location',
    },
    async () => {
      const response = await runRequest({
        baseUrl,
        method: 'GET',
        path: '/api/asset-hub/picker?type=location',
        token: requireToken(),
        timeoutMs,
      })

      const payload = expectJsonObject('19', response, 200)
      const locations = asArray(payload.locations, '19 locations')

      return {
        httpStatus: response.status,
        detail: `locations=${locations.length}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: '20',
      name: 'Bind character with globalVoiceId',
      method: 'PATCH',
      path: '/api/asset-hub/characters/{characterId}',
    },
    async () => {
      const createCharacterResponse = await runRequest({
        baseUrl,
        method: 'POST',
        path: '/api/asset-hub/characters',
        token: requireToken(),
        body: {
          name: 'E2E Bind Hero',
          folderId: null,
        },
        timeoutMs,
      })
      const createCharacterPayload = expectJsonObject('20.create-character', createCharacterResponse, 200)
      assertSuccess('20.create-character', createCharacterPayload)
      const bindCharacter = asObject(createCharacterPayload.character, '20.create-character character')
      const bindCharacterId = asNonEmptyString(bindCharacter.id, '20.create-character character.id')

      const createVoiceResponse = await runRequest({
        baseUrl,
        method: 'POST',
        path: '/api/asset-hub/voices',
        token: requireToken(),
        body: {
          name: 'E2E Bind Voice',
          voiceType: 'edge-tts',
        },
        timeoutMs,
      })
      const createVoicePayload = expectJsonObject('20.create-voice', createVoiceResponse, 200)
      assertSuccess('20.create-voice', createVoicePayload)
      const bindVoice = asObject(createVoicePayload.voice, '20.create-voice voice')
      const bindVoiceId = asNonEmptyString(bindVoice.id, '20.create-voice voice.id')

      const bindPath = `/api/asset-hub/characters/${encodeURIComponent(bindCharacterId)}`
      const bindResponse = await runRequest({
        baseUrl,
        method: 'PATCH',
        path: bindPath,
        token: requireToken(),
        body: {
          globalVoiceId: bindVoiceId,
        },
        timeoutMs,
      })
      const bindPayload = expectJsonObject('20.bind', bindResponse, 200)
      assertSuccess('20.bind', bindPayload)
      const boundCharacter = asObject(bindPayload.character, '20.bind character')
      assert(
        boundCharacter.globalVoiceId === bindVoiceId,
        `20.bind: expected globalVoiceId=${bindVoiceId}, got ${boundCharacter.globalVoiceId}`,
      )

      const detailResponse = await runRequest({
        baseUrl,
        method: 'GET',
        path: `/api/asset-hub/characters/${encodeURIComponent(bindCharacterId)}`,
        token: requireToken(),
        timeoutMs,
      })
      const detailPayload = expectJsonObject('20.detail', detailResponse, 200)
      const detailCharacter = asObject(detailPayload.character, '20.detail character')
      assert(
        detailCharacter.globalVoiceId === bindVoiceId,
        `20.detail: expected persisted globalVoiceId=${bindVoiceId}, got ${detailCharacter.globalVoiceId}`,
      )

      const cleanupCharacterResponse = await runRequest({
        baseUrl,
        method: 'DELETE',
        path: `/api/asset-hub/characters/${encodeURIComponent(bindCharacterId)}`,
        token: requireToken(),
        timeoutMs,
      })
      const cleanupCharacterPayload = expectJsonObject(
        '20.cleanup-character',
        cleanupCharacterResponse,
        200,
      )
      assertSuccess('20.cleanup-character', cleanupCharacterPayload)

      const cleanupVoiceResponse = await runRequest({
        baseUrl,
        method: 'DELETE',
        path: `/api/asset-hub/voices/${encodeURIComponent(bindVoiceId)}`,
        token: requireToken(),
        timeoutMs,
      })
      const cleanupVoicePayload = expectJsonObject('20.cleanup-voice', cleanupVoiceResponse, 200)
      assertSuccess('20.cleanup-voice', cleanupVoicePayload)

      return {
        path: bindPath,
        httpStatus: bindResponse.status,
        detail: `characterId=${bindCharacterId} globalVoiceId=${bindVoiceId} cleanup=ok`,
      }
    },
  )

  const markdownReport = renderMarkdownReport({
    baseUrl,
    username,
    rows,
  })
  writeReport(reportPath, markdownReport)

  const pass = rows.filter((row) => row.status === 'PASS').length
  const fail = rows.filter((row) => row.status === 'FAIL').length

  console.log(`wrote ${reportPath}`)
  console.log(`PASS=${pass} FAIL=${fail} TOTAL=${rows.length}`)

  if (fail > 0) {
    process.exitCode = 2
  }
}

main().catch((error) => {
  console.error(toErrorMessage(error))
  process.exitCode = 1
})
