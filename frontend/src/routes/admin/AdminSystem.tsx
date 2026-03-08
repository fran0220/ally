import { useTranslation } from 'react-i18next';

import { GlassSurface } from '../../components/ui/primitives';

export function AdminSystem() {
  const { t } = useTranslation('admin');

  return (
    <GlassSurface className="space-y-2" variant="elevated">
      <p className="text-sm text-[var(--glass-text-secondary)]">{t('systemMonitorDesc')}</p>
      <p className="text-xs text-[var(--glass-text-tertiary)]">{t('comingSoon')}</p>
    </GlassSurface>
  );
}
