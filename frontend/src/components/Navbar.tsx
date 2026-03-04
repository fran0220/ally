import { useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { Link, useNavigate } from 'react-router-dom';

import { clearSessionToken } from '../api/auth';
import { useHasAuthToken } from '../hooks/useHasAuthToken';
import { LanguageSwitcher } from './LanguageSwitcher';

export function Navbar() {
  const { t } = useTranslation(['common', 'nav', 'auth']);
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const isAuthenticated = useHasAuthToken();

  function handleLogout() {
    clearSessionToken();
    queryClient.clear();
    navigate('/auth/signin', { replace: true });
  }

  return (
    <header className="glass-nav sticky top-0 z-20">
      <div className="page-shell flex flex-wrap items-center justify-between gap-3 py-3 md:flex-nowrap">
        <Link to="/" className="text-lg font-semibold text-[var(--glass-text-primary)]">
          {t('common:appName')}
        </Link>
        <nav className="flex flex-wrap items-center gap-2 text-sm text-[var(--glass-text-secondary)] md:flex-nowrap md:justify-end">
          {isAuthenticated ? (
            <>
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
              <button className="glass-btn-base glass-btn-secondary h-9 px-3" type="button" onClick={handleLogout}>
                {t('nav:logout')}
              </button>
            </>
          ) : (
            <>
              <Link className="glass-btn-base glass-btn-secondary h-9 px-3" to="/auth/signin">
                {t('nav:signin')}
              </Link>
              <Link className="glass-btn-base glass-btn-ghost h-9 px-3" to="/auth/signup">
                {t('nav:signup')}
              </Link>
            </>
          )}
          <LanguageSwitcher />
        </nav>
      </div>
    </header>
  );
}
