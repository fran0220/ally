import { useTranslation } from 'react-i18next';

import { GlassSurface } from '../../components/ui/primitives';

export function ProfilePreferences() {
  const { i18n } = useTranslation('profile');
  const isZh = i18n.language.toLowerCase().startsWith('zh');

  return (
    <div className="p-4 md:p-6">
      <GlassSurface className="space-y-2">
        <p className="text-sm text-[var(--glass-text-secondary)]">
          {isZh ? '这里将支持语言、通知与工作流默认项等个性化配置。' : 'Language, notifications, and workflow defaults will be configurable here soon.'}
        </p>
        <p className="text-xs text-[var(--glass-text-tertiary)]">
          {isZh ? '即将推出' : 'Coming Soon'}
        </p>
      </GlassSurface>
    </div>
  );
}
