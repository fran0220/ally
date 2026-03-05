import { useTranslation } from 'react-i18next';

import { AppIcon } from '../../components/ui/icons';
import { GlassSurface } from '../../components/ui/primitives';

export function ProfilePreferences() {
  const { i18n } = useTranslation('profile');
  const isZh = i18n.language.toLowerCase().startsWith('zh');

  return (
    <div className="p-4 md:p-6">
      <GlassSurface className="space-y-3">
        <h2 className="flex items-center gap-2 text-lg font-semibold text-[var(--glass-text-primary)]">
          <AppIcon name="settingsHexMinor" className="h-4 w-4" />
          <span>{isZh ? '偏好设置 - 即将推出' : 'Preferences - Coming Soon'}</span>
        </h2>
        <p className="text-sm text-[var(--glass-text-secondary)]">
          {isZh ? '这里将支持语言、通知与工作流默认项等个性化配置。' : 'Language, notifications, and workflow defaults will be configurable here soon.'}
        </p>
      </GlassSurface>
    </div>
  );
}
