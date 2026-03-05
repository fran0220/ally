import { useTranslation } from 'react-i18next'
import { ProviderAdvancedFields } from './provider-card/ProviderAdvancedFields'
import { ProviderBaseFields } from './provider-card/ProviderBaseFields'
import { ProviderCardShell } from './provider-card/ProviderCardShell'
import { useProviderCardState } from './provider-card/hooks/useProviderCardState'
import type { ProviderCardProps, ProviderCardTranslator } from './provider-card/types'

export function ProviderCard({
  provider,
  models,
  allModels,
  defaultModels,
  onToggleModel,
  onUpdateApiKey,
  onUpdateBaseUrl,
  onDeleteModel,
  onUpdateModel,
  onDeleteProvider,
  onAddModel,
}: ProviderCardProps) {
  const { t: translate } = useTranslation('apiConfig')
  const t: ProviderCardTranslator = (key, values) => {
    const translated = translate(key, values as Record<string, unknown> | undefined)
    return typeof translated === 'string' ? translated : String(translated)
  }

  const state = useProviderCardState({
    provider,
    models,
    allModels,
    defaultModels,
    onUpdateApiKey,
    onUpdateBaseUrl,
    onUpdateModel,
    onAddModel,
    t,
  })

  return (
    <ProviderCardShell provider={provider} onDeleteProvider={onDeleteProvider} t={t} state={state}>
      <ProviderBaseFields provider={provider} t={t} state={state} />
      <ProviderAdvancedFields
        provider={provider}
        onToggleModel={onToggleModel}
        onDeleteModel={onDeleteModel}
        onUpdateModel={onUpdateModel}
        t={t}
        state={state}
      />
    </ProviderCardShell>
  )
}

export default ProviderCard
