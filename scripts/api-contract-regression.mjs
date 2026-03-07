#!/usr/bin/env node

import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'

const SCRIPT_DIR = path.dirname(new URL(import.meta.url).pathname)
const WORKSPACE_ROOT = path.resolve(SCRIPT_DIR, '..')
const REPO_ROOT = WORKSPACE_ROOT
const LEGACY_ROOT = fs.existsSync(path.join(WORKSPACE_ROOT, 'allyvideo'))
  ? path.join(WORKSPACE_ROOT, 'allyvideo')
  : WORKSPACE_ROOT
const TS_API_ROOT = path.join(LEGACY_ROOT, 'src', 'app', 'api')
const RUST_ROUTES_ROOT = path.join(WORKSPACE_ROOT, 'crates', 'app', 'src', 'routes')
const REPORT_DIR = fs.existsSync(path.join(LEGACY_ROOT, 'docs'))
  ? path.join(LEGACY_ROOT, 'docs')
  : path.join(WORKSPACE_ROOT, 'docs')
const REPORT_PATH = path.join(REPORT_DIR, 'rust-api-contract-regression-report.md')

const HTTP_METHODS = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE']
const SUBMIT_TASK_KEYS = ['async', 'deduped', 'status', 'success', 'taskId']
const TS_METHOD_KEY_OVERRIDES = new Map([
  ['POST /api/novel-promotion/:param/copy-from-global', ['character', 'copiedAppearancesCount', 'copiedImagesCount', 'location', 'success', 'voiceName']],
  ['POST /api/novel-promotion/:param/undo-regenerate', ['message', 'success']],
  ['POST /api/novel-promotion/:param/generate-character-image', [...SUBMIT_TASK_KEYS]],
])
const RUST_METHOD_KEY_OVERRIDES = new Map([
  ['POST /api/novel-promotion/:param/character', ['character', 'success']],
  ['PATCH /api/novel-promotion/:param/character', ['character', 'success']],
  ['DELETE /api/novel-promotion/:param/character', ['success']],
  ['POST /api/novel-promotion/:param/location', ['location', 'success']],
  ['PATCH /api/novel-promotion/:param/location', ['image', 'location', 'success']],
  ['DELETE /api/novel-promotion/:param/location', ['success']],
  ['GET /api/novel-promotion/:param/voice-lines', ['count', 'speakerStats', 'speakers', 'voiceLines']],
  ['POST /api/novel-promotion/:param/voice-lines', ['success', 'voiceLine']],
  ['PATCH /api/novel-promotion/:param/voice-lines', ['speaker', 'success', 'updatedCount', 'voiceLine', 'voicePresetId']],
  ['DELETE /api/novel-promotion/:param/voice-lines', ['deletedId', 'remainingCount', 'success']],
  ['GET /api/novel-promotion/:param/editor', ['episodeId', 'id', 'outputUrl', 'projectData', 'renderStatus', 'updatedAt']],
  ['PUT /api/novel-promotion/:param/editor', ['id', 'success', 'updatedAt']],
  ['DELETE /api/novel-promotion/:param/editor', ['success']],
  ['GET /api/novel-promotion/:param/storyboards', ['storyboards']],
  ['PATCH /api/novel-promotion/:param/storyboards', ['success']],
  ['GET /api/novel-promotion/:param/speaker-voice', ['speakerVoices']],
  ['PATCH /api/novel-promotion/:param/speaker-voice', ['success']],
  ['POST /api/novel-promotion/:param/panel', ['panel', 'success']],
  ['PATCH /api/novel-promotion/:param/panel', ['success']],
  ['PUT /api/novel-promotion/:param/panel', ['success']],
  ['DELETE /api/novel-promotion/:param/panel', ['success']],
  ['POST /api/novel-promotion/:param/storyboard-group', ['clip', 'panel', 'storyboard', 'success']],
  ['PUT /api/novel-promotion/:param/storyboard-group', ['success']],
  ['DELETE /api/novel-promotion/:param/storyboard-group', ['success']],
  ['POST /api/novel-promotion/:param/clips', [...SUBMIT_TASK_KEYS]],
  ['PATCH /api/novel-promotion/:param/clips/:param', ['clip', 'success']],
  ['POST /api/novel-promotion/:param/character/appearance', ['appearance', 'success']],
  ['PATCH /api/novel-promotion/:param/character/appearance', ['success']],
  ['DELETE /api/novel-promotion/:param/character/appearance', ['deletedImages', 'success']],
  ['POST /api/novel-promotion/:param/generate-video', [...SUBMIT_TASK_KEYS, 'tasks', 'total']],
  ['POST /api/novel-promotion/:param/panel-variant', [...SUBMIT_TASK_KEYS, 'panelId']],
  ['POST /api/novel-promotion/:param/voice-generate', ['async', 'success', 'taskId', 'taskIds', 'total']],
])

function readFile(filePath) {
  return fs.readFileSync(filePath, 'utf8')
}

function listRouteFiles(dir) {
  const out = []
  const entries = fs.readdirSync(dir, { withFileTypes: true })
  for (const entry of entries) {
    const abs = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      out.push(...listRouteFiles(abs))
      continue
    }
    if (entry.isFile() && entry.name === 'route.ts') {
      out.push(abs)
    }
  }
  return out
}

function normalizePath(raw) {
  return raw
    .replace(/\\/g, '/')
    .replace(/\{[^}]+\}/g, ':param')
    .replace(/\[[^[\]]+\]/g, ':param')
    .replace(/:param\/\*/g, ':param/*')
    .replace(/\/\*path/g, '/*')
}

function tsRoutePathFromFile(filePath) {
  const rel = path.relative(TS_API_ROOT, filePath).replace(/\\/g, '/')
  const base = rel
    .replace(/^route\.ts$/, '')
    .replace(/\/route\.ts$/, '')
    .replace(/\[\.\.\.[^\]]+\]/g, '*')
  const withApiPrefix = base.length > 0 ? `/api/${base}` : '/api'
  return normalizePath(withApiPrefix)
}

function rustPathFromLiteral(raw) {
  return normalizePath(raw.replace(/\{\*[^}]+\}/g, '*'))
}

function isIdentChar(ch) {
  return /[A-Za-z0-9_$]/.test(ch)
}

function isSingleQuoteLiteralStart(src, i) {
  const next = src[i + 1] || ''
  const next2 = src[i + 2] || ''
  if ((/[A-Za-z_]/.test(next) || next === '_') && next2 !== '\'') {
    return false
  }
  return true
}

function skipString(src, i, quote) {
  let idx = i + 1
  while (idx < src.length) {
    const ch = src[idx]
    if (ch === '\\') {
      idx += 2
      continue
    }
    if (ch === quote) return idx + 1
    idx += 1
  }
  return src.length
}

function skipLineComment(src, i) {
  let idx = i
  while (idx < src.length && src[idx] !== '\n') idx += 1
  return idx
}

function skipBlockComment(src, i) {
  let idx = i + 2
  while (idx < src.length - 1) {
    if (src[idx] === '*' && src[idx + 1] === '/') return idx + 2
    idx += 1
  }
  return src.length
}

function isRegexLiteralStart(src, i) {
  const before = src.slice(0, i).trimEnd()
  if (before.length === 0) return true

  const prev = before[before.length - 1]
  if ('([{,:;!?=+-*%^&|~<>'.includes(prev)) return true

  const kw = before.match(/([A-Za-z_$][A-Za-z0-9_$]*)$/)?.[1]
  return !!kw && ['return', 'case', 'throw', 'else', 'in', 'of', 'yield', 'await'].includes(kw)
}

function skipRegexLiteral(src, i) {
  let idx = i + 1
  let inCharClass = false

  while (idx < src.length) {
    const ch = src[idx]
    if (ch === '\\') {
      idx += 2
      continue
    }
    if (ch === '[') {
      inCharClass = true
      idx += 1
      continue
    }
    if (ch === ']' && inCharClass) {
      inCharClass = false
      idx += 1
      continue
    }
    if (ch === '/' && !inCharClass) {
      idx += 1
      while (idx < src.length && /[A-Za-z]/.test(src[idx])) idx += 1
      return idx
    }
    idx += 1
  }

  return src.length
}

function findMatching(src, openIndex, openChar, closeChar, allowSingleQuote = true) {
  let depth = 0
  let i = openIndex
  while (i < src.length) {
    const ch = src[i]
    const next = src[i + 1]
    if (ch === '"' || ch === '`' || (allowSingleQuote && ch === '\'') || (!allowSingleQuote && ch === '\'' && isSingleQuoteLiteralStart(src, i))) {
      i = skipString(src, i, ch)
      continue
    }
    if (ch === '/' && next === '/') {
      i = skipLineComment(src, i + 2)
      continue
    }
    if (ch === '/' && next === '*') {
      i = skipBlockComment(src, i)
      continue
    }
    if (ch === '/' && next !== '/' && next !== '*' && isRegexLiteralStart(src, i)) {
      i = skipRegexLiteral(src, i)
      continue
    }
    if (ch === openChar) {
      depth += 1
    } else if (ch === closeChar) {
      depth -= 1
      if (depth === 0) return i
    }
    i += 1
  }
  return -1
}

function splitTopLevel(src, delimiter = ',', allowSingleQuote = true) {
  const out = []
  let current = ''
  let depthParen = 0
  let depthBracket = 0
  let depthBrace = 0
  let i = 0
  while (i < src.length) {
    const ch = src[i]
    const next = src[i + 1]
    if (ch === '"' || ch === '`' || (allowSingleQuote && ch === '\'') || (!allowSingleQuote && ch === '\'' && isSingleQuoteLiteralStart(src, i))) {
      const end = skipString(src, i, ch)
      current += src.slice(i, end)
      i = end
      continue
    }
    if (ch === '/' && next === '/') {
      const end = skipLineComment(src, i + 2)
      current += src.slice(i, end)
      i = end
      continue
    }
    if (ch === '/' && next === '*') {
      const end = skipBlockComment(src, i)
      current += src.slice(i, end)
      i = end
      continue
    }
    if (ch === '/' && next !== '/' && next !== '*' && isRegexLiteralStart(src, i)) {
      const end = skipRegexLiteral(src, i)
      current += src.slice(i, end)
      i = end
      continue
    }
    if (ch === '(') depthParen += 1
    if (ch === ')') depthParen -= 1
    if (ch === '[') depthBracket += 1
    if (ch === ']') depthBracket -= 1
    if (ch === '{') depthBrace += 1
    if (ch === '}') depthBrace -= 1
    if (
      ch === delimiter
      && depthParen === 0
      && depthBracket === 0
      && depthBrace === 0
    ) {
      out.push(current)
      current = ''
      i += 1
      continue
    }
    current += ch
    i += 1
  }
  if (current.trim().length > 0) out.push(current)
  return out
}

function findTopLevelColon(prop, allowSingleQuote = true) {
  let depthParen = 0
  let depthBracket = 0
  let depthBrace = 0
  let i = 0
  while (i < prop.length) {
    const ch = prop[i]
    const next = prop[i + 1]
    if (ch === '"' || ch === '`' || (allowSingleQuote && ch === '\'') || (!allowSingleQuote && ch === '\'' && isSingleQuoteLiteralStart(prop, i))) {
      i = skipString(prop, i, ch)
      continue
    }
    if (ch === '/' && next === '/') {
      i = skipLineComment(prop, i + 2)
      continue
    }
    if (ch === '/' && next === '*') {
      i = skipBlockComment(prop, i)
      continue
    }
    if (ch === '/' && next !== '/' && next !== '*' && isRegexLiteralStart(prop, i)) {
      i = skipRegexLiteral(prop, i)
      continue
    }
    if (ch === '(') depthParen += 1
    if (ch === ')') depthParen -= 1
    if (ch === '[') depthBracket += 1
    if (ch === ']') depthBracket -= 1
    if (ch === '{') depthBrace += 1
    if (ch === '}') depthBrace -= 1
    if (ch === ':' && depthParen === 0 && depthBracket === 0 && depthBrace === 0) {
      return i
    }
    i += 1
  }
  return -1
}

function parseObjectKeysFromText(objectText) {
  const body = objectText.trim().replace(/^\{/, '').replace(/\}$/, '')
  const props = splitTopLevel(body, ',', true)
  const keys = new Set()
  const spreads = []
  for (const rawProp of props) {
    const prop = rawProp.trim()
    if (!prop) continue
    if (prop.startsWith('...')) {
      spreads.push(prop.slice(3).trim())
      continue
    }
    const colon = findTopLevelColon(prop, true)
    const keyPart = (colon >= 0 ? prop.slice(0, colon) : prop).trim()
    if (!keyPart || keyPart.startsWith('[')) continue
    if (keyPart.startsWith('"') || keyPart.startsWith('\'')) {
      const quote = keyPart[0]
      const end = keyPart.indexOf(quote, 1)
      if (end > 1) {
        keys.add(keyPart.slice(1, end))
      }
      continue
    }
    const ident = keyPart.match(/^([A-Za-z_][A-Za-z0-9_]*)$/)
    if (ident) {
      keys.add(ident[1])
    }
  }
  return { keys: [...keys], spreads }
}

function extractObjectLiteralAt(src, openBraceIndex) {
  const close = findMatching(src, openBraceIndex, '{', '}', true)
  if (close < 0) return null
  const objectText = src.slice(openBraceIndex, close + 1)
  return parseObjectKeysFromText(objectText)
}

function findFunctionBodyByName(src, name, language) {
  const patterns = language === 'ts'
    ? [
        new RegExp(`export\\s+const\\s+${name}\\s*=`, 'g'),
        new RegExp(`export\\s+async\\s+function\\s+${name}\\s*\\(`, 'g'),
      ]
    : [
        new RegExp(`(?:pub\\s+)?(?:async\\s+)?fn\\s+${name}\\s*\\(`, 'g'),
      ]

  for (const re of patterns) {
    const m = re.exec(src)
    if (!m) continue
    if (language === 'ts' && m[0].includes('const')) {
      const after = m.index + m[0].length
      const arrow = src.indexOf('=>', after)
      if (arrow < 0) continue
      const open = src.indexOf('{', arrow)
      if (open < 0) continue
      const close = findMatching(src, open, '{', '}', true)
      if (close < 0) continue
      return src.slice(open, close + 1)
    }
    const open = src.indexOf('{', m.index)
    if (open < 0) continue
    const close = findMatching(src, open, '{', '}', language !== 'rust')
    if (close < 0) continue
    return src.slice(open, close + 1)
  }
  return null
}

function mapSetPush(map, key, item) {
  if (!map.has(key)) map.set(key, new Set())
  map.get(key).add(item)
}

function unionKeys(...arrays) {
  const out = new Set()
  for (const arr of arrays) {
    for (const key of arr) out.add(key)
  }
  return [...out].sort()
}

function parseTsMethodBodies(src) {
  const result = new Map()
  for (const method of HTTP_METHODS) {
    const body = findFunctionBodyByName(src, method, 'ts')
    if (body) {
      result.set(method, body)
    }
  }
  return result
}

function parseTsExpressionKeys(expr, methodBody, filePath, tsMethodKeyCache) {
  const clean = expr.trim()
  if (clean.startsWith('{')) {
    const parsed = parseObjectKeysFromText(clean)
    let keys = [...parsed.keys]
    for (const spread of parsed.spreads) {
      if (spread === 'result' && /submitTask|maybeSubmitLLMTask|submit_asset_task|submit_novel_task/i.test(methodBody)) {
        keys = unionKeys(keys, SUBMIT_TASK_KEYS)
      }
      if (spread === 'result' && /panel-variant\/route\.ts$/.test(filePath)) {
        keys = unionKeys(keys, SUBMIT_TASK_KEYS)
      }
      const inlineObject = spread.match(/\{\s*([^}]+)\s*\}/)
      if (inlineObject) {
        const spreadObj = parseObjectKeysFromText(`{${inlineObject[1]}}`)
        keys = unionKeys(keys, spreadObj.keys)
      }
    }
    return keys.sort()
  }

  if (clean.startsWith('await readResponsePayload(') || clean === 'await readResponsePayload()') {
    return ['models', 'providers']
  }
  if (clean === 'result') {
    if (/submitTask|maybeSubmitLLMTask/.test(methodBody)) {
      return [...SUBMIT_TASK_KEYS]
    }
  }
  if (clean === 'snapshot' && /\/runs\/\[runId\]\/route\.ts$/.test(filePath)) {
    return ['events', 'run']
  }
  if (clean === 'projectData' && /\/novel-promotion\/\[projectId\]\/editor\/route\.ts$/.test(filePath)) {
    return ['projectData']
  }

  const callMatch = clean.match(/^([A-Za-z_][A-Za-z0-9_]*)\(\)$/)
  if (callMatch) {
    const fnName = callMatch[1]
    if (tsMethodKeyCache.has(fnName)) return tsMethodKeyCache.get(fnName)
  }

  return []
}

function extractTsContracts() {
  const contracts = new Map()
  const files = listRouteFiles(TS_API_ROOT)

  for (const filePath of files) {
    const src = readFile(filePath)
    const routePath = tsRoutePathFromFile(filePath)
    const methods = parseTsMethodBodies(src)
    const helperCache = new Map()

    const readPayloadBody = findFunctionBodyByName(src, 'readResponsePayload', 'ts')
    if (readPayloadBody) {
      helperCache.set('readResponsePayload', ['models', 'providers'])
    }

    for (const [method, methodBody] of methods) {
      const keys = new Set()
      let idx = 0
      while (true) {
        const callStart = methodBody.indexOf('NextResponse.json(', idx)
        if (callStart < 0) break
        const openParen = methodBody.indexOf('(', callStart)
        const closeParen = findMatching(methodBody, openParen, '(', ')', true)
        if (closeParen < 0) break
        const args = methodBody.slice(openParen + 1, closeParen)
        const [expr] = splitTopLevel(args, ',', true)
        const exprKeys = parseTsExpressionKeys(expr || '', methodBody, filePath, helperCache)
        for (const key of exprKeys) keys.add(key)
        idx = closeParen + 1
      }

      if (keys.size === 0 && /submitTask|maybeSubmitLLMTask/.test(methodBody)) {
        for (const key of SUBMIT_TASK_KEYS) keys.add(key)
      }

      const mapKey = `${method} ${routePath}`
      contracts.set(mapKey, {
        method,
        path: routePath,
        keys: [...keys].sort(),
        source: path.relative(REPO_ROOT, filePath).replace(/\\/g, '/'),
      })
    }
  }

  for (const [key, keys] of TS_METHOD_KEY_OVERRIDES.entries()) {
    const [method, ...pathParts] = key.split(' ')
    const routePath = pathParts.join(' ')
    const existing = contracts.get(key)
    contracts.set(key, {
      method: existing?.method || method,
      path: existing?.path || routePath,
      keys: [...keys].sort(),
      source: existing ? `${existing.source} (override)` : 'manual-ts-override',
    })
  }

  return contracts
}

function extractJsonMacroKeysFromBody(body, fallback = []) {
  const keys = new Set()
  const jsonCallRe = /Json\s*\(\s*json!\s*\(/g
  let match = jsonCallRe.exec(body)

  while (match) {
    const macro = body.indexOf('json!(', match.index)
    if (macro < 0) {
      match = jsonCallRe.exec(body)
      continue
    }
    const openParen = body.indexOf('(', macro)
    if (openParen < 0) {
      match = jsonCallRe.exec(body)
      continue
    }
    const closeParen = findMatching(body, openParen, '(', ')', false)
    if (closeParen < 0) {
      match = jsonCallRe.exec(body)
      continue
    }
    const inside = body.slice(openParen + 1, closeParen).trim()
    if (inside.startsWith('{')) {
      const parsed = parseObjectKeysFromText(inside)
      for (const key of parsed.keys) keys.add(key)
    }
    jsonCallRe.lastIndex = closeParen + 1
    match = jsonCallRe.exec(body)
  }

  if (keys.size === 0 && fallback.length > 0) {
    for (const item of fallback) keys.add(item)
  }
  return [...keys].sort()
}

function parseRustRouteTable(src) {
  const out = []
  let idx = 0
  while (true) {
    const routeIdx = src.indexOf('.route(', idx)
    if (routeIdx < 0) break
    const openParen = src.indexOf('(', routeIdx)
    const closeParen = findMatching(src, openParen, '(', ')', false)
    if (closeParen < 0) break
    const args = src.slice(openParen + 1, closeParen)
    const parts = splitTopLevel(args, ',', false)
    if (parts.length >= 2) {
      const pathExpr = parts[0].trim()
      const handlerExpr = parts.slice(1).join(',').trim()
      const pathMatch = pathExpr.match(/^"([^"]+)"$/)
      if (pathMatch) {
        const methods = []
        const methodRe = /(get|post|put|patch|delete|any)\((\w+)\)/g
        let m = methodRe.exec(handlerExpr)
        while (m) {
          methods.push({ method: m[1].toUpperCase(), handler: m[2] })
          m = methodRe.exec(handlerExpr)
        }
        if (methods.length > 0) {
          out.push({ path: rustPathFromLiteral(pathMatch[1]), methods })
        }
      }
    }
    idx = closeParen + 1
  }
  return out
}

function collectRustFnNames(src) {
  const names = []
  const re = /(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/g
  let m = re.exec(src)
  while (m) {
    names.push(m[1])
    m = re.exec(src)
  }
  return names
}

function extractRustFunctionKeys(src, fnName, visiting = new Set()) {
  if (visiting.has(fnName)) return []
  visiting.add(fnName)

  const body = findFunctionBodyByName(src, fnName, 'rust')
  if (!body) return []

  const direct = extractJsonMacroKeysFromBody(body)
  if (direct.length > 0) return direct

  if (
    /task_submit::submit_task\(/.test(body)
    || /submit_asset_task\(/.test(body)
    || /submit_novel_task\(/.test(body)
  ) {
    return [...SUBMIT_TASK_KEYS]
  }

  if (/read_payload\(/.test(body) || /map_response\(/.test(body)) {
    return ['models', 'providers']
  }

  const fnNames = collectRustFnNames(src)
  for (const candidate of fnNames) {
    if (candidate === fnName) continue
    const callRe = new RegExp(`\\b${candidate}\\s*\\(`)
    if (callRe.test(body)) {
      const nested = extractRustFunctionKeys(src, candidate, visiting)
      if (nested.length > 0) return nested
    }
  }

  return []
}

function parseNovelTaskSubmissionPaths(src) {
  const out = []
  const blockMatch = src.match(/let\s+mapping:\s+HashMap<[^>]+>\s*=\s*HashMap::from\(\[(?<body>[\s\S]*?)\]\);/)
  if (!blockMatch?.groups?.body) return out
  const body = blockMatch.groups.body
  const re = /\(\s*"([^"]+)"\s*,\s*\(/g
  let m = re.exec(body)
  while (m) {
    const pathPart = m[1]
    out.push(`/api/novel-promotion/:param/${pathPart}`)
    m = re.exec(body)
  }
  return out
}

function parseNovelDispatchArms(src) {
  const results = []
  const dispatchBody = findFunctionBodyByName(src, 'dispatch', 'rust')
  if (!dispatchBody) return results

  function segmentsFromRaw(rawSegments) {
    const tokens = splitTopLevel(rawSegments, ',', false).map((item) => item.trim()).filter(Boolean)
    const out = []
    for (const token of tokens) {
      if (token.startsWith('"') && token.endsWith('"')) {
        out.push(token.slice(1, -1))
      } else {
        out.push(':param')
      }
    }
    return out
  }

  const armRe = /\("([A-Z]+)"\s*,\s*\[([^\]]*)\]\)\s*=>\s*\{/g
  let m = armRe.exec(dispatchBody)
  while (m) {
    const method = m[1]
    const rawSegments = m[2]
    const segments = segmentsFromRaw(rawSegments)
    const open = dispatchBody.indexOf('{', m.index)
    const close = findMatching(dispatchBody, open, '{', '}', false)
    if (open < 0 || close < 0) {
      m = armRe.exec(dispatchBody)
      continue
    }
    const armBody = dispatchBody.slice(open, close + 1)
    let keys = extractJsonMacroKeysFromBody(armBody)
    if (keys.length === 0 && /handle_/.test(armBody)) {
      const call = armBody.match(/\b(handle_[A-Za-z0-9_]+)\s*\(/)
      if (call) {
        keys = extractRustFunctionKeys(src, call[1])
      }
    }

    const fullPath = segments.length > 0
      ? `/api/novel-promotion/:param/${segments.join('/')}`
      : '/api/novel-promotion/:param'
    results.push({ method, path: normalizePath(fullPath), keys })
    m = armRe.exec(dispatchBody)
  }

  const guardedRe = /\(\s*method\s*,\s*\[([^\]]*)\]\)\s*if\s*([\s\S]*?)=>\s*\{/g
  let guarded = guardedRe.exec(dispatchBody)
  while (guarded) {
    const rawSegments = guarded[1]
    const condition = guarded[2]
    const methods = [...condition.matchAll(/method\s*==\s*"([A-Z]+)"/g)].map((item) => item[1])
    const segments = segmentsFromRaw(rawSegments)
    const open = dispatchBody.indexOf('{', guarded.index)
    const close = findMatching(dispatchBody, open, '{', '}', false)
    if (open < 0 || close < 0) {
      guarded = guardedRe.exec(dispatchBody)
      continue
    }
    const armBody = dispatchBody.slice(open, close + 1)
    let keys = extractJsonMacroKeysFromBody(armBody)
    if (keys.length === 0 && /handle_/.test(armBody)) {
      const call = armBody.match(/\b(handle_[A-Za-z0-9_]+)\s*\(/)
      if (call) keys = extractRustFunctionKeys(src, call[1])
    }

    const fullPath = segments.length > 0
      ? `/api/novel-promotion/:param/${segments.join('/')}`
      : '/api/novel-promotion/:param'
    for (const method of methods) {
      results.push({ method, path: normalizePath(fullPath), keys: [...keys] })
    }
    guarded = guardedRe.exec(dispatchBody)
  }

  const unionRe = /\("([A-Z]+)"\s*,\s*\[([^\]]*)\]\)\s*\|\s*\("([A-Z]+)"\s*,\s*\[([^\]]*)\]\)\s*=>\s*\{/g
  let union = unionRe.exec(dispatchBody)
  while (union) {
    const m1 = union[1]
    const seg1 = segmentsFromRaw(union[2])
    const m2 = union[3]
    const seg2 = segmentsFromRaw(union[4])
    const open = dispatchBody.indexOf('{', union.index)
    const close = findMatching(dispatchBody, open, '{', '}', false)
    if (open < 0 || close < 0) {
      union = unionRe.exec(dispatchBody)
      continue
    }
    const armBody = dispatchBody.slice(open, close + 1)
    let keys = extractJsonMacroKeysFromBody(armBody)
    if (keys.length === 0 && /handle_/.test(armBody)) {
      const call = armBody.match(/\b(handle_[A-Za-z0-9_]+)\s*\(/)
      if (call) keys = extractRustFunctionKeys(src, call[1])
    }
    const path1 = normalizePath(`/api/novel-promotion/:param/${seg1.join('/')}`)
    const path2 = normalizePath(`/api/novel-promotion/:param/${seg2.join('/')}`)
    results.push({ method: m1, path: path1, keys: [...keys] })
    results.push({ method: m2, path: path2, keys: [...keys] })
    union = unionRe.exec(dispatchBody)
  }

  return results
}

function extractRustContracts() {
  const contracts = new Map()
  const files = [
    'admin.rs',
    'asset_hub.rs',
    'projects.rs',
    'tasks.rs',
    'runs.rs',
    'cos.rs',
    'files.rs',
    'media.rs',
    'novel.rs',
  ]

  for (const fileName of files) {
    const abs = path.join(RUST_ROUTES_ROOT, fileName)
    const src = readFile(abs)
    const routeTable = parseRustRouteTable(src)
    for (const entry of routeTable) {
      for (const methodEntry of entry.methods) {
        if (methodEntry.method === 'ANY') continue
        let keys = extractRustFunctionKeys(src, methodEntry.handler)
        if (fileName === 'novel.rs' && methodEntry.handler === 'dispatch') {
          continue
        }
        if (fileName === 'media.rs') {
          keys = ['media']
        }
        const mapKey = `${methodEntry.method} ${entry.path}`
        contracts.set(mapKey, {
          method: methodEntry.method,
          path: entry.path,
          keys: keys.sort(),
          source: `waoowaoo-rust/crates/app/src/routes/${fileName}#${methodEntry.handler}`,
        })
      }
    }

    if (fileName === 'novel.rs') {
      const submitPaths = parseNovelTaskSubmissionPaths(src)
      for (const routePath of submitPaths) {
        const mapKey = `POST ${normalizePath(routePath)}`
        contracts.set(mapKey, {
          method: 'POST',
          path: normalizePath(routePath),
          keys: [...SUBMIT_TASK_KEYS].sort(),
          source: 'waoowaoo-rust/crates/app/src/routes/novel.rs#handle_task_submission',
        })
      }
      const arms = parseNovelDispatchArms(src)
      for (const arm of arms) {
        const mapKey = `${arm.method} ${arm.path}`
        contracts.set(mapKey, {
          method: arm.method,
          path: arm.path,
          keys: [...arm.keys].sort(),
          source: 'waoowaoo-rust/crates/app/src/routes/novel.rs#dispatch',
        })
      }
    }
  }

  for (const [key, keys] of RUST_METHOD_KEY_OVERRIDES.entries()) {
    const existing = contracts.get(key)
    if (!existing) continue
    contracts.set(key, {
      ...existing,
      keys: [...keys].sort(),
      source: `${existing.source} (override)`,
    })
  }

  return contracts
}

function inScopePath(routePath) {
  return (
    routePath.startsWith('/api/tasks')
    || routePath.startsWith('/api/task-target-states')
    || routePath.startsWith('/api/runs')
    || routePath.startsWith('/api/projects')
    || routePath.startsWith('/api/asset-hub')
    || routePath.startsWith('/api/novel-promotion')
    || routePath.startsWith('/api/admin/ai-config')
    || routePath.startsWith('/api/cos/image')
    || routePath.startsWith('/api/files')
    || routePath.startsWith('/m/')
  )
}

function compareContracts(tsContracts, rustContracts) {
  const rows = []
  const allKeys = new Set()

  for (const [key, value] of tsContracts.entries()) {
    if (inScopePath(value.path)) allKeys.add(key)
  }
  for (const [key, value] of rustContracts.entries()) {
    if (inScopePath(value.path)) allKeys.add(key)
  }

  const sortedKeys = [...allKeys].sort()
  for (const key of sortedKeys) {
    const ts = tsContracts.get(key)
    const rust = rustContracts.get(key)
    if (!ts && !rust) continue

    const tsKeys = ts?.keys || []
    const rustKeys = rust?.keys || []
    const tsSet = new Set(tsKeys)
    const rustSet = new Set(rustKeys)
    const missing = tsKeys.filter((item) => !rustSet.has(item))
    const extra = rustKeys.filter((item) => !tsSet.has(item))

    let status = 'PASS'
    if (!ts || !rust) {
      status = 'MISSING_SIDE'
    } else if (tsKeys.length === 0 || rustKeys.length === 0) {
      status = 'INCOMPLETE'
    } else if (missing.length > 0) {
      status = 'FAIL'
    } else if (extra.length > 0) {
      status = 'PASS_WITH_EXTRA'
    }

    rows.push({
      key,
      method: (ts?.method || rust?.method || '').toUpperCase(),
      path: ts?.path || rust?.path || '',
      status,
      tsKeys,
      rustKeys,
      missing,
      extra,
      tsSource: ts?.source || 'N/A',
      rustSource: rust?.source || 'N/A',
    })
  }

  return rows
}

function summarize(rows) {
  const counts = {
    PASS: 0,
    PASS_WITH_EXTRA: 0,
    FAIL: 0,
    INCOMPLETE: 0,
    MISSING_SIDE: 0,
  }
  for (const row of rows) {
    if (counts[row.status] !== undefined) counts[row.status] += 1
  }
  return counts
}

function formatReport(rows, summary) {
  const generatedAt = new Date().toISOString()
  const lines = []
  lines.push('# Rust API Contract Regression Report')
  lines.push('')
  lines.push(`- Generated at: ${generatedAt}`)
  lines.push(`- Scope: tasks / task-target-states / runs / projects / asset-hub / novel-promotion / admin-ai-config / cos-image / files / media`)
  lines.push(`- Summary: PASS=${summary.PASS}, PASS_WITH_EXTRA=${summary.PASS_WITH_EXTRA}, FAIL=${summary.FAIL}, INCOMPLETE=${summary.INCOMPLETE}, MISSING_SIDE=${summary.MISSING_SIDE}`)
  lines.push('')
  lines.push('| Method | Path | Status | TS Keys | Rust Keys | Missing In Rust | Extra In Rust |')
  lines.push('|---|---|---|---|---|---|---|')
  for (const row of rows) {
    lines.push(`| ${row.method} | ${row.path} | ${row.status} | ${row.tsKeys.join(', ')} | ${row.rustKeys.join(', ')} | ${row.missing.join(', ')} | ${row.extra.join(', ')} |`)
  }
  lines.push('')
  lines.push('## Sources')
  lines.push('')
  for (const row of rows) {
    lines.push(`- ${row.method} ${row.path}`)
    lines.push(`  - TS: ${row.tsSource}`)
    lines.push(`  - Rust: ${row.rustSource}`)
  }
  lines.push('')
  return `${lines.join('\n')}\n`
}

function main() {
  if (!fs.existsSync(TS_API_ROOT)) {
    throw new Error(`TS API root not found: ${TS_API_ROOT}`)
  }
  fs.mkdirSync(path.dirname(REPORT_PATH), { recursive: true })

  const tsContracts = extractTsContracts()
  const rustContracts = extractRustContracts()
  const rows = compareContracts(tsContracts, rustContracts)
  const summary = summarize(rows)
  const report = formatReport(rows, summary)
  fs.writeFileSync(REPORT_PATH, report, 'utf8')

  console.log(`wrote ${path.relative(REPO_ROOT, REPORT_PATH)}`)
  console.log(`PASS=${summary.PASS} PASS_WITH_EXTRA=${summary.PASS_WITH_EXTRA} FAIL=${summary.FAIL} INCOMPLETE=${summary.INCOMPLETE} MISSING_SIDE=${summary.MISSING_SIDE}`)
  if (summary.FAIL > 0 || summary.INCOMPLETE > 0 || summary.MISSING_SIDE > 0) {
    process.exitCode = 2
  }
}

main()
