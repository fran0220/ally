import { useState } from 'react';
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
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  function closeMobileMenu() {
    setMobileMenuOpen(false);
  }

  function handleLogout() {
    clearSessionToken();
    queryClient.clear();
    closeMobileMenu();
    navigate('/auth/signin', { replace: true });
  }

  return (
    <header className="glass-nav sticky top-0 z-20">
      <div className="page-shell py-3">
        <div className="flex items-center justify-between gap-3">
          <Link to="/" className="text-lg font-semibold text-[var(--glass-text-primary)]" onClick={closeMobileMenu}>
            {t('common:appName')}
          </Link>

          <button
            type="button"
            aria-controls="mobile-nav-menu"
            aria-expanded={mobileMenuOpen}
            aria-label={mobileMenuOpen ? 'Close navigation menu' : 'Open navigation menu'}
            className="glass-btn-base glass-btn-ghost h-11 min-h-[44px] w-11 p-0 md:hidden"
            onClick={() => setMobileMenuOpen((open) => !open)}
          >
            <svg
              aria-hidden="true"
              viewBox="0 0 24 24"
              className="h-5 w-5 text-[var(--glass-text-primary)]"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.8"
              strokeLinecap="round"
            >
              <line x1="4" y1="6.5" x2="20" y2="6.5" />
              <line x1="4" y1="12" x2="20" y2="12" />
              <line x1="4" y1="17.5" x2="20" y2="17.5" />
            </svg>
          </button>

          <nav className="hidden items-center gap-2 text-sm text-[var(--glass-text-secondary)] md:flex md:justify-end">
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
      </div>

      <div
        id="mobile-nav-menu"
        className={`md:hidden ${
          mobileMenuOpen ? 'pointer-events-auto border-t border-[var(--glass-stroke-soft)]' : 'pointer-events-none'
        }`}
      >
        <nav
          className={`page-shell overflow-hidden text-sm text-[var(--glass-text-secondary)] transition-[max-height,opacity,transform] duration-[320ms] [transition-timing-function:cubic-bezier(0.68,-0.55,0.265,1.55)] ${
            mobileMenuOpen ? 'max-h-[32rem] translate-y-0 opacity-100 py-3' : 'max-h-0 -translate-y-1 opacity-0 py-0'
          }`}
        >
          <div className="flex flex-col gap-2 pb-2">
            {isAuthenticated ? (
              <>
                <Link
                  className="glass-btn-base glass-btn-ghost h-11 min-h-[44px] w-full justify-start px-4"
                  to="/workspace"
                  onClick={closeMobileMenu}
                >
                  {t('nav:workspace')}
                </Link>
                <Link
                  className="glass-btn-base glass-btn-ghost h-11 min-h-[44px] w-full justify-start px-4"
                  to="/workspace/asset-hub"
                  onClick={closeMobileMenu}
                >
                  {t('nav:assetHub')}
                </Link>
                <Link
                  className="glass-btn-base glass-btn-ghost h-11 min-h-[44px] w-full justify-start px-4"
                  to="/profile"
                  onClick={closeMobileMenu}
                >
                  {t('nav:profile')}
                </Link>
                <Link
                  className="glass-btn-base glass-btn-ghost h-11 min-h-[44px] w-full justify-start px-4"
                  to="/admin/ai-config"
                  onClick={closeMobileMenu}
                >
                  {t('apiConfig:title')}
                </Link>
                <button
                  className="glass-btn-base glass-btn-secondary h-11 min-h-[44px] w-full justify-start px-4"
                  type="button"
                  onClick={handleLogout}
                >
                  {t('nav:logout')}
                </button>
              </>
            ) : (
              <>
                <Link
                  className="glass-btn-base glass-btn-secondary h-11 min-h-[44px] w-full justify-start px-4"
                  to="/auth/signin"
                  onClick={closeMobileMenu}
                >
                  {t('nav:signin')}
                </Link>
                <Link
                  className="glass-btn-base glass-btn-ghost h-11 min-h-[44px] w-full justify-start px-4"
                  to="/auth/signup"
                  onClick={closeMobileMenu}
                >
                  {t('nav:signup')}
                </Link>
              </>
            )}
          </div>

          <div className="pb-3">
            <LanguageSwitcher />
          </div>
        </nav>
      </div>
    </header>
  );
}
