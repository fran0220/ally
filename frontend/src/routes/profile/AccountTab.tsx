import { useTranslation } from 'react-i18next';

import { useCurrentUser } from '../../hooks/useCurrentUser';
import { GlassButton, GlassField, GlassInput, GlassSurface } from '../../components/ui/primitives';

export function AccountTab() {
  const { t } = useTranslation('profile');
  const { username, role: rawRole } = useCurrentUser();

  const fallback = t('notProvided');
  const displayUsername = username ?? fallback;
  const role =
    rawRole === 'admin'
      ? t('admin')
      : rawRole === 'user'
        ? t('user')
        : rawRole ?? fallback;

  return (
    <div className="space-y-4 p-4 md:p-6">
      <GlassSurface className="space-y-4">
        <div className="grid gap-3 md:grid-cols-2">
          <GlassField label={t('username')}>
            <GlassInput value={displayUsername} readOnly />
          </GlassField>

          <GlassField label={t('email')}>
            <GlassInput value={fallback} readOnly />
          </GlassField>

          <GlassField label={t('role')}>
            <GlassInput value={role} readOnly />
          </GlassField>
        </div>
      </GlassSurface>

      <GlassSurface className="space-y-3">
        <h3 className="text-sm font-semibold text-[var(--glass-text-secondary)]">{t('security')}</h3>
        <p className="text-sm text-[var(--glass-text-secondary)]">{t('securityDesc')}</p>
        <GlassButton variant="soft" disabled>
          {`${t('changePassword')} (${t('comingSoon')})`}
        </GlassButton>
      </GlassSurface>

      <GlassSurface className="space-y-3">
        <h3 className="text-sm font-semibold text-[var(--glass-text-secondary)]">{t('avatar')}</h3>
        <p className="text-sm text-[var(--glass-text-secondary)]">{t('avatarDesc')}</p>
        <GlassButton variant="soft" disabled>
          {`${t('uploadAvatar')} (${t('comingSoon')})`}
        </GlassButton>
      </GlassSurface>
    </div>
  );
}
