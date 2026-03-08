import { useTranslation } from 'react-i18next';

import { GlassSurface } from '../../components/ui/primitives';

export function ProfilePreferences() {
  const { t } = useTranslation('profile');

  return (
    <div className="p-4 md:p-6">
      <GlassSurface className="space-y-2">
        <p className="text-sm text-[var(--glass-text-secondary)]">{t('preferencesDesc')}</p>
        <p className="text-xs text-[var(--glass-text-tertiary)]">{t('comingSoon')}</p>
      </GlassSurface>
    </div>
  );
}
