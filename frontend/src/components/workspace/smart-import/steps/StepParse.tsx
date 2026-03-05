import { useTranslations } from '@/compat/next-intl'

export default function StepParse() {
  const t = useTranslations('smartImport')

  return (
    <div className="min-h-[calc(100vh-200px)] flex items-center justify-center p-8">
      <div className="text-center">
        <div className="flex gap-1.5 justify-center mb-8">
          {[0, 1, 2, 3, 4].map((i) => (
            <div
              key={i}
              className="w-3 h-12 bg-[var(--glass-accent-from)] rounded-full animate-pulse"
              style={{
                animationDelay: `${i * 0.1}s`,
                animationDuration: '1s',
              }}
            />
          ))}
        </div>
        <h2 className="text-xl font-semibold text-[var(--glass-text-primary)] mb-2">{t('analyzing.title')}</h2>
        <p className="text-[var(--glass-text-secondary)]">{t('analyzing.description')}</p>
        <p className="text-sm text-[var(--glass-text-tertiary)] mt-2">{t('analyzing.autoSave')}</p>
      </div>
    </div>
  )
}
