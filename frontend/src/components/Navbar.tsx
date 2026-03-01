import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';

import { LanguageSwitcher } from './LanguageSwitcher';

export function Navbar() {
  const { t } = useTranslation(['common', 'nav', 'auth']);

  return (
    <header className="glass-nav sticky top-0 z-20">
      <div className="page-shell flex flex-wrap items-center justify-between gap-3 py-3">
        <Link to="/" className="text-lg font-semibold text-[var(--glass-text-primary)]">
          {t('common:appName')}
        </Link>
        <nav className="flex flex-wrap items-center gap-2 text-sm text-[var(--glass-text-secondary)]">
          <Link className="glass-btn-base glass-btn-ghost h-9 px-3" to="/workspace">
            {t('nav:workspace')}
          </Link>
          <Link className="glass-btn-base glass-btn-ghost h-9 px-3" to="/workspace/asset-hub">
            {t('nav:assetHub')}
          </Link>
          <Link className="glass-btn-base glass-btn-ghost h-9 px-3" to="/profile">
            {t('nav:profile')}
          </Link>
          <Link className="glass-btn-base glass-btn-ghost h-9 px-3" to="/admin/ai-config">
            {t('apiConfig:title')}
          </Link>
          <Link className="glass-btn-base glass-btn-secondary h-9 px-3" to="/auth/signin">
            {t('nav:signin')}
          </Link>
          <LanguageSwitcher />
        </nav>
      </div>
    </header>
  );
}
