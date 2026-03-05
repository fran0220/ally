import { useMemo } from 'react';
import { NavLink, Outlet } from 'react-router-dom';
import { useTranslation } from 'react-i18next';

import { AppIcon, type AppIconName } from '../../components/ui/icons';
import { GlassSurface } from '../../components/ui/primitives';

interface AdminNavItem {
  to: 'ai-config' | 'users' | 'system';
  icon: AppIconName;
  label: string;
}

function navItemClass(isActive: boolean): string {
  return [
    'glass-btn-base h-10 w-full justify-start gap-2 px-3 text-sm',
    isActive
      ? 'glass-btn-primary text-white'
      : 'glass-btn-soft text-[var(--glass-text-secondary)] hover:text-[var(--glass-text-primary)]',
  ].join(' ');
}

export function AdminLayout() {
  const { t, i18n } = useTranslation(['apiConfig', 'common']);
  const isZh = i18n.language.toLowerCase().startsWith('zh');

  const navItems = useMemo<AdminNavItem[]>(
    () => [
      {
        to: 'ai-config',
        icon: 'settingsHex',
        label: t('apiConfig:title', { defaultValue: isZh ? 'AI 配置' : 'AI Config' }),
      },
      {
        to: 'users',
        icon: 'userCircle',
        label: isZh ? '用户管理' : 'User Management',
      },
      {
        to: 'system',
        icon: 'monitor',
        label: isZh ? '系统监控' : 'System Monitor',
      },
    ],
    [isZh, t],
  );

  return (
    <div className="page-shell py-8 md:py-10">
      <header className="mb-4 space-y-1">
        <h1 className="glass-page-title">{isZh ? '管理员设置' : 'Admin Settings'}</h1>
        <p className="glass-page-subtitle">
          {isZh ? '集中管理 AI 配置、用户和系统能力。' : 'Manage AI config, users, and system capabilities in one place.'}
        </p>
      </header>

      <div className="mb-4 md:hidden">
        <GlassSurface padded={false} className="overflow-x-auto">
          <nav className="flex min-w-max gap-2 p-2">
            {navItems.map((item) => (
              <NavLink key={item.to} to={item.to} className={({ isActive }) => navItemClass(isActive)}>
                <AppIcon name={item.icon} className="h-4 w-4" />
                <span>{item.label}</span>
              </NavLink>
            ))}
          </nav>
        </GlassSurface>
      </div>

      <div className="grid gap-4 md:grid-cols-[240px_minmax(0,1fr)]">
        <aside className="hidden md:block">
          <GlassSurface padded={false} className="sticky top-24">
            <nav className="flex flex-col gap-2 p-3">
              {navItems.map((item) => (
                <NavLink key={item.to} to={item.to} className={({ isActive }) => navItemClass(isActive)}>
                  <AppIcon name={item.icon} className="h-4 w-4" />
                  <span>{item.label}</span>
                </NavLink>
              ))}
            </nav>
          </GlassSurface>
        </aside>

        <section className="min-w-0">
          <Outlet />
        </section>
      </div>
    </div>
  );
}
