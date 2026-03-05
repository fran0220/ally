import { useTranslation } from 'react-i18next';

import { AppIcon } from '../../components/ui/icons';
import { GlassSurface } from '../../components/ui/primitives';

export function AdminUsers() {
  const { i18n } = useTranslation('common');
  const isZh = i18n.language.toLowerCase().startsWith('zh');

  return (
    <GlassSurface className="space-y-4" variant="elevated">
      <div className="flex items-center gap-2">
        <span className="inline-flex h-8 w-8 items-center justify-center rounded-lg bg-[var(--glass-bg-muted)] text-[var(--glass-text-secondary)]">
          <AppIcon name="userCircle" className="h-4 w-4" />
        </span>
        <h2 className="text-lg font-semibold text-[var(--glass-text-primary)]">
          {isZh ? '用户管理 - 即将推出' : 'User Management - Coming Soon'}
        </h2>
      </div>
      <p className="text-sm text-[var(--glass-text-secondary)]">
        {isZh
          ? '该模块将用于查看用户列表、权限分配与账号状态管理。'
          : 'This module will provide user list, role assignment, and account state management.'}
      </p>
    </GlassSurface>
  );
}
