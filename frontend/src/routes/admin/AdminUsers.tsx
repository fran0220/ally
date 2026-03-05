import { useTranslation } from 'react-i18next';

import { GlassSurface } from '../../components/ui/primitives';

export function AdminUsers() {
  const { i18n } = useTranslation('common');
  const isZh = i18n.language.toLowerCase().startsWith('zh');

  return (
    <GlassSurface className="space-y-2" variant="elevated">
      <p className="text-sm text-[var(--glass-text-secondary)]">
        {isZh
          ? '该模块将用于查看用户列表、权限分配与账号状态管理。'
          : 'This module will provide user list, role assignment, and account state management.'}
      </p>
      <p className="text-xs text-[var(--glass-text-tertiary)]">
        {isZh ? '即将推出' : 'Coming Soon'}
      </p>
    </GlassSurface>
  );
}
