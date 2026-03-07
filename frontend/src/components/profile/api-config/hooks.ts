import { logError as _ulogError } from '@/lib/logging/core'
import { getUserApiConfig, updateUserApiConfig } from '@/api/user'

import { useState, useEffect, useRef, useCallback } from 'react'
import {
    CustomModel,
    encodeModelKey,
} from './types'
import type { CapabilitySelections, CapabilityValue } from '@/lib/model-config-contract'

interface DefaultModels {
    analysisModel?: string
    characterModel?: string
    locationModel?: string
    storyboardModel?: string
    editModel?: string
    videoModel?: string
    lipSyncModel?: string
}

interface UseProvidersReturn {
    models: CustomModel[]
    defaultModels: DefaultModels
    capabilityDefaults: CapabilitySelections
    loading: boolean
    saveStatus: 'idle' | 'saving' | 'saved' | 'error'
    updateDefaultModel: (field: keyof DefaultModels, modelKey: string, capabilityFieldsToDefault?: Array<{ field: string; options: CapabilityValue[] }>) => void
    updateCapabilityDefault: (modelKey: string, field: string, value: string | number | boolean | null) => void
    getModelsByType: (type: CustomModel['type']) => CustomModel[]
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return !!value && typeof value === 'object' && !Array.isArray(value)
}

function normalizeModels(rawModels: CustomModel[], providerNameById: Map<string, string>): CustomModel[] {
    const seen = new Set<string>()
    const normalizedModels: CustomModel[] = []

    for (const model of rawModels) {
        const modelKey = model.modelKey || encodeModelKey(model.provider, model.modelId)
        if (seen.has(modelKey)) continue
        seen.add(modelKey)

        const providerName = providerNameById.get(model.provider) || model.providerName
        normalizedModels.push({
            ...model,
            modelKey,
            ...(providerName ? { providerName } : {}),
        })
    }

    return normalizedModels
}

export function useProviders(): UseProvidersReturn {
    const [models, setModels] = useState<CustomModel[]>([])
    const [defaultModels, setDefaultModels] = useState<DefaultModels>({})
    const [capabilityDefaults, setCapabilityDefaults] = useState<CapabilitySelections>({})
    const [loading, setLoading] = useState(true)
    const [saveStatus, setSaveStatus] = useState<'idle' | 'saving' | 'saved' | 'error'>('idle')
    const saveStatusTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
    const initializedRef = useRef(false)

    const latestDefaultModelsRef = useRef(defaultModels)
    const latestCapabilityDefaultsRef = useRef(capabilityDefaults)
    useEffect(() => { latestDefaultModelsRef.current = defaultModels }, [defaultModels])
    useEffect(() => { latestCapabilityDefaultsRef.current = capabilityDefaults }, [capabilityDefaults])

    useEffect(() => {
        return () => {
            if (saveStatusTimeoutRef.current) {
                clearTimeout(saveStatusTimeoutRef.current)
                saveStatusTimeoutRef.current = null
            }
        }
    }, [])

    useEffect(() => {
        void fetchConfig()
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [])

    async function fetchConfig() {
        initializedRef.current = false
        let loadedSuccessfully = false
        try {
            const data = await getUserApiConfig()
            const providerNameById = new Map(
                (data.providers || []).map((provider) => [provider.id, provider.name] as const),
            )
            const normalizedModels = normalizeModels(data.models || [], providerNameById)
            const nextDefaultModels = data.defaultModels || {}
            const nextCapabilityDefaults = isRecord(data.capabilityDefaults)
                ? (data.capabilityDefaults as CapabilitySelections)
                : {}

            setModels(normalizedModels)
            setDefaultModels(nextDefaultModels)
            setCapabilityDefaults(nextCapabilityDefaults)
            latestDefaultModelsRef.current = nextDefaultModels
            latestCapabilityDefaultsRef.current = nextCapabilityDefaults
            setSaveStatus('idle')
            loadedSuccessfully = true
        } catch (error) {
            _ulogError('获取配置失败:', error)
            setSaveStatus('error')
        } finally {
            setLoading(false)
            if (loadedSuccessfully) {
                initializedRef.current = true
            }
        }
    }

    const performSave = useCallback(async (overrides?: {
        defaultModels?: DefaultModels
        capabilityDefaults?: CapabilitySelections
    }, optimistic = false) => {
        if (!initializedRef.current) {
            return
        }

        if (saveStatusTimeoutRef.current) {
            clearTimeout(saveStatusTimeoutRef.current)
            saveStatusTimeoutRef.current = null
        }

        if (optimistic) {
            setSaveStatus('saved')
            saveStatusTimeoutRef.current = setTimeout(() => setSaveStatus('idle'), 3000)
        } else {
            setSaveStatus('saving')
        }

        try {
            const currentDefaultModels = overrides?.defaultModels ?? latestDefaultModelsRef.current
            const currentCapabilityDefaults = overrides?.capabilityDefaults ?? latestCapabilityDefaultsRef.current

            await updateUserApiConfig({
                defaultModels: currentDefaultModels,
                capabilityDefaults: currentCapabilityDefaults,
            })

            if (!optimistic) {
                setSaveStatus('saved')
                saveStatusTimeoutRef.current = setTimeout(() => setSaveStatus('idle'), 3000)
            }
        } catch (error) {
            _ulogError('保存失败:', error)
            setSaveStatus('error')
        }
    }, []) // 无依赖，所有值均从 ref 读取

    const updateDefaultModel = useCallback((
        field: keyof DefaultModels,
        modelKey: string,
        capabilityFieldsToDefault?: Array<{ field: string; options: CapabilityValue[] }>,
    ) => {
        setDefaultModels(prev => {
            const next = { ...prev, [field]: modelKey }
            latestDefaultModelsRef.current = next

            if (capabilityFieldsToDefault && capabilityFieldsToDefault.length > 0) {
                setCapabilityDefaults(prevCap => {
                    const nextCap: CapabilitySelections = { ...prevCap }
                    const existing = { ...(nextCap[modelKey] || {}) }
                    let changed = false
                    for (const def of capabilityFieldsToDefault) {
                        const firstOption = def.options[0]
                        if (existing[def.field] === undefined && firstOption !== undefined) {
                            existing[def.field] = firstOption
                            changed = true
                        }
                    }
                    if (changed) {
                        nextCap[modelKey] = existing
                        latestCapabilityDefaultsRef.current = nextCap
                        void performSave({ defaultModels: next, capabilityDefaults: nextCap }, true)
                        return nextCap
                    }
                    void performSave({ defaultModels: next }, true)
                    return prevCap
                })
            } else {
                void performSave({ defaultModels: next }, true)
            }
            return next
        })
    }, [performSave])

    const updateCapabilityDefault = useCallback((modelKey: string, field: string, value: string | number | boolean | null) => {
        setCapabilityDefaults((previous) => {
            const next: CapabilitySelections = { ...previous }
            const current = { ...(next[modelKey] || {}) }
            if (value === null) {
                delete current[field]
            } else {
                current[field] = value
            }

            if (Object.keys(current).length === 0) {
                delete next[modelKey]
            } else {
                next[modelKey] = current
            }
            latestCapabilityDefaultsRef.current = next
            void performSave({ capabilityDefaults: next }, true)
            return next
        })
    }, [performSave])

    const getModelsByType = useCallback((type: CustomModel['type']) => {
        return models.filter(m => m.type === type)
    }, [models])

    return {
        models,
        defaultModels,
        capabilityDefaults,
        loading,
        saveStatus,
        updateDefaultModel,
        updateCapabilityDefault,
        getModelsByType
    }
}
