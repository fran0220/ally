import { NavLink, Outlet } from 'react-router-dom';
import { useTranslation } from 'react-i18next';

import { AppIcon, type AppIconName } from '../../components/ui/icons';
import { GlassSurface } from '../../components/ui/primitives';

interface ProfileTab {
  to: 'account' | 'api-config' | 'preferences';
  icon: AppIconName;
  label: string;
}

function tabClass(isActive: boolean): string {
  return [
    'glass-btn-base h-9 gap-2 px-3 text-sm',
    isActive
      ? 'glass-btn-primary text-white'
      : 'glass-btn-soft text-[var(--glass-text-secondary)] hover:text-[var(--glass-text-primary)]',
  ].join(' ');
}

export function Profile() {
  const { t } = useTranslation('profile');

  const tabs: ProfileTab[] = [
    {
      to: 'account',
      icon: 'userCircle',
      label: t('account'),
    },
    {
      to: 'api-config',
      icon: 'settingsHex',
      label: t('profile:apiConfig'),
    },
    {
      to: 'preferences',
      icon: 'settingsHexMinor',
      label: t('preferences'),
    },
  ];

  return (
    <main className="page-shell py-10">
      <header className="mb-4 space-y-1">
        <h1 className="glass-page-title">{t('personalAccount')}</h1>
        <p className="glass-page-subtitle">{t('subtitle')}</p>
      </header>

      <GlassSurface className="mb-4" padded={false}>
        <nav className="flex flex-wrap gap-2 p-2">
          {tabs.map((tab) => (
            <NavLink key={tab.to} to={tab.to} className={({ isActive }) => tabClass(isActive)}>
              <AppIcon name={tab.icon} className="h-4 w-4" />
              <span>{tab.label}</span>
            </NavLink>
          ))}
        </nav>
      </GlassSurface>

      <section className="glass-surface-elevated overflow-hidden">
        <Outlet />
      </section>
    </main>
  );
}
