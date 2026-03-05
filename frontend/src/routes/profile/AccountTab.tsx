import { useTranslation } from 'react-i18next';

import { useCurrentUser } from '../../hooks/useCurrentUser';
import { GlassButton, GlassField, GlassInput, GlassSurface } from '../../components/ui/primitives';

export function AccountTab() {
  const { i18n } = useTranslation(['profile', 'common']);
  const isZh = i18n.language.toLowerCase().startsWith('zh');
  const { username, role: rawRole } = useCurrentUser();

  const fallback = isZh ? '未提供' : 'Not provided';
  const displayUsername = username ?? fallback;
  const role =
    rawRole === 'admin'
      ? isZh
        ? '管理员'
        : 'Administrator'
      : rawRole === 'user'
        ? isZh
          ? '普通用户'
          : 'User'
        : rawRole ?? fallback;

  return (
    <div className="space-y-4 p-4 md:p-6">
      <GlassSurface className="space-y-4">
        <div className="grid gap-3 md:grid-cols-2">
          <GlassField label={isZh ? '用户名' : 'Username'}>
            <GlassInput value={displayUsername} readOnly />
          </GlassField>

          <GlassField label={isZh ? '邮箱' : 'Email'}>
            <GlassInput value={fallback} readOnly />
          </GlassField>

          <GlassField label={isZh ? '角色' : 'Role'}>
            <GlassInput value={role} readOnly />
          </GlassField>
        </div>
      </GlassSurface>

      <GlassSurface className="space-y-3">
        <h3 className="text-sm font-semibold text-[var(--glass-text-secondary)]">
          {isZh ? '安全设置' : 'Security'}
        </h3>
        <p className="text-sm text-[var(--glass-text-secondary)]">
          {isZh ? '修改密码功能正在开发中。' : 'Password update flow is under development.'}
        </p>
        <GlassButton variant="soft" disabled>
          {isZh ? '修改密码（即将推出）' : 'Change Password (Coming Soon)'}
        </GlassButton>
      </GlassSurface>

      <GlassSurface className="space-y-3">
        <h3 className="text-sm font-semibold text-[var(--glass-text-secondary)]">
          {isZh ? '头像设置' : 'Avatar'}
        </h3>
        <p className="text-sm text-[var(--glass-text-secondary)]">
          {isZh ? '头像上传区域预留完成，后续会接入上传和裁剪流程。' : 'Avatar upload area is reserved for a future upload and crop flow.'}
        </p>
        <GlassButton variant="soft" disabled>
          {isZh ? '上传头像（即将推出）' : 'Upload Avatar (Coming Soon)'}
        </GlassButton>
      </GlassSurface>
    </div>
  );
}
