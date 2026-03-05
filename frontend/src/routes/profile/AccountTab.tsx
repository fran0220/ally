import { useMemo, useSyncExternalStore } from 'react';
import { useTranslation } from 'react-i18next';

import { getAuthToken, subscribeAuthToken } from '../../api/client';
import { AppIcon } from '../../components/ui/icons';
import { GlassButton, GlassField, GlassInput, GlassSurface } from '../../components/ui/primitives';

interface AccessTokenClaims {
  sub: string;
  username: string;
  role: string;
  iat?: number;
}

function decodeBase64Url(value: string): string | null {
  const normalized = value.replace(/-/g, '+').replace(/_/g, '/');
  const padded = normalized.padEnd(Math.ceil(normalized.length / 4) * 4, '=');

  try {
    const binary = atob(padded);
    const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
    return new TextDecoder().decode(bytes);
  } catch {
    return null;
  }
}

function parseAccessTokenClaims(token: string | null): AccessTokenClaims | null {
  if (!token) {
    return null;
  }

  const [, payload] = token.split('.');
  if (!payload) {
    return null;
  }

  const decoded = decodeBase64Url(payload);
  if (!decoded) {
    return null;
  }

  try {
    const parsed = JSON.parse(decoded) as Record<string, unknown>;
    if (
      typeof parsed.sub !== 'string' ||
      typeof parsed.username !== 'string' ||
      typeof parsed.role !== 'string'
    ) {
      return null;
    }

    return {
      sub: parsed.sub,
      username: parsed.username,
      role: parsed.role,
      iat: typeof parsed.iat === 'number' ? parsed.iat : undefined,
    };
  } catch {
    return null;
  }
}

function formatDateTime(timestampSeconds: number | undefined, locale: string): string | null {
  if (!timestampSeconds || !Number.isFinite(timestampSeconds)) {
    return null;
  }

  try {
    return new Intl.DateTimeFormat(locale, {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    }).format(new Date(timestampSeconds * 1000));
  } catch {
    return null;
  }
}

export function AccountTab() {
  const { i18n } = useTranslation(['profile', 'common']);
  const isZh = i18n.language.toLowerCase().startsWith('zh');
  const token = useSyncExternalStore(subscribeAuthToken, getAuthToken, () => null);
  const claims = useMemo(() => parseAccessTokenClaims(token), [token]);

  const fallback = isZh ? '未提供' : 'Not provided';
  const username = claims?.username ?? fallback;
  const role =
    claims?.role === 'admin'
      ? isZh
        ? '管理员'
        : 'Administrator'
      : claims?.role === 'user'
        ? isZh
          ? '普通用户'
          : 'User'
        : claims?.role ?? fallback;
  const tokenIssuedAt = formatDateTime(claims?.iat, i18n.language);

  return (
    <div className="space-y-4 p-4 md:p-6">
      <GlassSurface className="space-y-4">
        <h2 className="flex items-center gap-2 text-lg font-semibold text-[var(--glass-text-primary)]">
          <AppIcon name="userCircle" className="h-5 w-5" />
          <span>{isZh ? '账户信息' : 'Account Information'}</span>
        </h2>

        <div className="grid gap-3 md:grid-cols-2">
          <GlassField label={isZh ? '用户名' : 'Username'}>
            <GlassInput value={username} readOnly />
          </GlassField>

          <GlassField label={isZh ? '邮箱' : 'Email'}>
            <GlassInput value={fallback} readOnly />
          </GlassField>

          <GlassField label={isZh ? '角色' : 'Role'}>
            <GlassInput value={role} readOnly />
          </GlassField>

          <GlassField
            label={isZh ? '注册时间' : 'Registered At'}
            hint={
              tokenIssuedAt
                ? isZh
                  ? '当前显示的是令牌签发时间（后端尚未返回注册时间字段）。'
                  : 'Showing token issued time temporarily until a dedicated registration field is exposed.'
                : undefined
            }
          >
            <GlassInput value={tokenIssuedAt ?? fallback} readOnly />
          </GlassField>
        </div>
      </GlassSurface>

      <GlassSurface className="space-y-3">
        <h3 className="flex items-center gap-2 text-base font-semibold text-[var(--glass-text-primary)]">
          <AppIcon name="lock" className="h-4 w-4" />
          <span>{isZh ? '安全设置' : 'Security'}</span>
        </h3>
        <p className="text-sm text-[var(--glass-text-secondary)]">
          {isZh ? '修改密码功能正在开发中。' : 'Password update flow is under development.'}
        </p>
        <GlassButton variant="soft" disabled>
          {isZh ? '修改密码（即将推出）' : 'Change Password (Coming Soon)'}
        </GlassButton>
      </GlassSurface>

      <GlassSurface className="space-y-3">
        <h3 className="flex items-center gap-2 text-base font-semibold text-[var(--glass-text-primary)]">
          <AppIcon name="image" className="h-4 w-4" />
          <span>{isZh ? '头像设置' : 'Avatar'}</span>
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
