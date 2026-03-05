import { useTranslation } from 'react-i18next';

import { AppIcon } from '../../components/ui/icons';
import { GlassSurface } from '../../components/ui/primitives';

export function AdminSystem() {
  const { i18n } = useTranslation('common');
  const isZh = i18n.language.toLowerCase().startsWith('zh');

  return (
    <GlassSurface className="space-y-4" variant="elevated">
      <div className="flex items-center gap-2">
        <span className="inline-flex h-8 w-8 items-center justify-center rounded-lg bg-[var(--glass-bg-muted)] text-[var(--glass-text-secondary)]">
          <AppIcon name="monitor" className="h-4 w-4" />
        </span>
        <h2 className="text-lg font-semibold text-[var(--glass-text-primary)]">
          {isZh ? '系统监控 - 即将推出' : 'System Monitor - Coming Soon'}
        </h2>
      </div>
      <p className="text-sm text-[var(--glass-text-secondary)]">
        {isZh
          ? '后续会提供服务健康状态、任务吞吐和异常告警等运维视图。'
          : 'Service health, task throughput, and alert panels will be available here soon.'}
      </p>
    </GlassSurface>
  );
}
