import type { ModelCapabilities } from '@/lib/model-config-contract'

export type UnifiedModelType = 'llm' | 'image' | 'video' | 'audio' | 'lipsync'

interface ParsedModelKeyStrict {
    provider: string
    modelId: string
}

function composeModelKey(provider: string, modelId: string): string {
    return `${provider.trim()}::${modelId.trim()}`
}

function parseModelKeyStrict(key: string | undefined | null): ParsedModelKeyStrict | null {
    if (!key) return null
    const normalized = key.trim()
    const markerIndex = normalized.indexOf('::')
    if (markerIndex <= 0) return null

    const provider = normalized.slice(0, markerIndex).trim()
    const modelId = normalized.slice(markerIndex + 2).trim()
    if (!provider || !modelId) return null

    return { provider, modelId }
}

export interface Provider {
    id: string
    name: string
    baseUrl?: string
    apiKey?: string
    hasApiKey?: boolean
    apiMode?: 'gemini-sdk' | 'openai-official'
}

export interface LlmCustomPricing {
    inputPerMillion?: number
    outputPerMillion?: number
}

export interface MediaCustomPricing {
    basePrice?: number
    optionPrices?: Record<string, Record<string, number>>
}

export interface CustomModelPricing {
    llm?: LlmCustomPricing
    image?: MediaCustomPricing
    video?: MediaCustomPricing
}

export interface CustomModel {
    modelId: string
    modelKey: string
    name: string
    type: UnifiedModelType
    provider: string
    providerName?: string
    price: number
    priceMin?: number
    priceMax?: number
    priceLabel?: string
    priceInput?: number
    priceOutput?: number
    enabled: boolean
    capabilities?: ModelCapabilities
    customPricing?: CustomModelPricing
}

const PROVIDER_DISPLAY_NAME_MAP: Record<string, { en: string; zh?: string }> = {
    ark: { en: 'Volcengine Ark', zh: '火山引擎 Ark' },
    google: { en: 'Google AI Studio' },
    openrouter: { en: 'OpenRouter' },
    minimax: { en: 'MiniMax Hailuo', zh: '海螺 MiniMax' },
    vidu: { en: 'Vidu', zh: '生数科技 Vidu' },
    fal: { en: 'FAL' },
    qwen: { en: 'Qwen' },
    'gemini-compatible': { en: 'Gemini Compatible' },
    'openai-compatible': { en: 'OpenAI Compatible' },
}

function isZhLocale(locale?: string): boolean {
    return typeof locale === 'string' && locale.toLowerCase().startsWith('zh')
}

export function getProviderKey(providerId?: string): string {
    if (!providerId) return ''
    const colonIndex = providerId.indexOf(':')
    return colonIndex === -1 ? providerId : providerId.slice(0, colonIndex)
}

export function getProviderDisplayName(providerId?: string, locale?: string): string {
    if (!providerId) return ''
    const providerKey = getProviderKey(providerId)
    const displayName = PROVIDER_DISPLAY_NAME_MAP[providerKey]
    if (!displayName) return providerId
    if (isZhLocale(locale) && displayName.zh) return displayName.zh
    return displayName.en
}

export function encodeModelKey(provider: string, modelId: string): string {
    return composeModelKey(provider, modelId)
}

export function parseModelKey(key: string | undefined | null): { provider: string, modelId: string } | null {
    const parsed = parseModelKeyStrict(key)
    if (!parsed) return null
    return {
        provider: parsed.provider,
        modelId: parsed.modelId,
    }
}

export function matchesModelKey(key: string | undefined | null, provider: string, modelId: string): boolean {
    const parsed = parseModelKeyStrict(key)
    if (!parsed) return false
    return parsed.provider === provider && parsed.modelId === modelId
}
