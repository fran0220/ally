import { useMemo } from 'react'
import type { CustomModel } from '../../api-config'

interface UseApiConfigFiltersParams {
  models: CustomModel[]
}

const MODEL_TYPES: Array<'llm' | 'image' | 'video' | 'lipsync'> = ['llm', 'image', 'video', 'lipsync']

function isModelProviderType(type: CustomModel['type']): type is 'llm' | 'image' | 'video' | 'lipsync' {
  return MODEL_TYPES.includes(type as 'llm' | 'image' | 'video' | 'lipsync')
}

export function useApiConfigFilters({
  models,
}: UseApiConfigFiltersParams) {
  const enabledModelsByType = useMemo(() => {
    const grouped: Record<'llm' | 'image' | 'video' | 'lipsync', CustomModel[]> = {
      llm: [],
      image: [],
      video: [],
      lipsync: [],
    }

    for (const model of models) {
      if (!model.enabled) continue
      if (!isModelProviderType(model.type)) continue

      grouped[model.type].push(model)
    }

    return grouped
  }, [models])

  return {
    getEnabledModelsByType: (type: 'llm' | 'image' | 'video' | 'lipsync') => enabledModelsByType[type],
  }
}
