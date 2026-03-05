import { useTranslation } from 'react-i18next';

import { GlassSurface } from '../../components/ui/primitives';

export function AdminSystem() {
  const { i18n } = useTranslation('common');
  const isZh = i18n.language.toLowerCase().startsWith('zh');

  return (
    <GlassSurface className="space-y-2" variant="elevated">
      <p className="text-sm text-[var(--glass-text-secondary)]">
        {isZh
          ? '后续会提供服务健康状态、任务吞吐和异常告警等运维视图。'
          : 'Service health, task throughput, and alert panels will be available here soon.'}
      </p>
      <p className="text-xs text-[var(--glass-text-tertiary)]">
        {isZh ? '即将推出' : 'Coming Soon'}
      </p>
    </GlassSurface>
  );
}
