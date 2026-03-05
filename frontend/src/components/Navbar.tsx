import { useEffect, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { Link, useNavigate } from 'react-router-dom';

import { logout } from '../api/auth';
import { useAuthSession } from '../hooks/useAuthSession';
import { queryKeys } from '../lib/query-keys';
import { LanguageSwitcher } from './LanguageSwitcher';

export function Navbar() {
  const { t } = useTranslation(['common', 'nav', 'auth']);
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const { user, isAuthenticated, isBootstrapping } = useAuthSession();
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [avatarMenuOpen, setAvatarMenuOpen] = useState(false);
  const [loggingOut, setLoggingOut] = useState(false);
  const avatarRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handler(event: MouseEvent) {
      if (avatarRef.current && !avatarRef.current.contains(event.target as Node)) {
        setAvatarMenuOpen(false);
      }
    }
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, []);

  function closeMobileMenu() {
    setMobileMenuOpen(false);
  }

  async function handleLogout() {
    if (loggingOut) {
      return;
    }

    setLoggingOut(true);

    try {
      await logout();
    } catch {
      // Local/session state is still cleared in logout(), then we continue with client-side cleanup.
    } finally {
      queryClient.setQueryData(queryKeys.auth.session(), null);
      queryClient.removeQueries({
        predicate: (query) => query.queryKey[0] !== 'auth',
      });
      closeMobileMenu();
      setAvatarMenuOpen(false);
      navigate('/auth/signin', { replace: true });
      setLoggingOut(false);
    }
  }

  const username = user?.name ?? null;
  const isAdmin = user?.role === 'admin';
  const avatarLetter = username ? username.charAt(0).toUpperCase() || '?' : '?';

  return (
    <header className="glass-nav sticky top-0 z-20">
      <div className="page-shell py-2">
        <div className="flex items-center justify-between gap-3">
          <Link to="/" className="text-lg font-semibold text-[var(--glass-text-primary)]" onClick={closeMobileMenu}>
            {t('common:appName')}
          </Link>

          {/* Mobile hamburger */}
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

          {/* Desktop nav */}
          <nav className="hidden items-center gap-2 text-sm text-[var(--glass-text-secondary)] md:flex md:justify-end">
            {isAuthenticated ? (
              <>
                <Link className="glass-btn-base glass-btn-ghost h-8 px-3" to="/workspace">
                  {t('nav:workspace')}
                </Link>

                {/* Avatar dropdown */}
                <div ref={avatarRef} className="relative">
                  <button
                    type="button"
                    className="glass-btn-base glass-btn-ghost flex h-8 items-center gap-2 px-2"
                    onClick={() => setAvatarMenuOpen((open) => !open)}
                  >
                    <span className="flex h-7 w-7 items-center justify-center rounded-full bg-[var(--glass-tone-info-bg)] text-xs font-semibold text-[var(--glass-tone-info-fg)]">
                      {avatarLetter}
                    </span>
                    <svg
                      aria-hidden="true"
                      viewBox="0 0 20 20"
                      className={`h-4 w-4 transition-transform ${avatarMenuOpen ? 'rotate-180' : ''}`}
                      fill="currentColor"
                    >
                      <path
                        fillRule="evenodd"
                        d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z"
                        clipRule="evenodd"
                      />
                    </svg>
                  </button>

                  {avatarMenuOpen ? (
                    <div className="glass-surface absolute right-0 top-full z-30 mt-1 w-52 overflow-hidden rounded-[var(--glass-radius-md)] border border-[var(--glass-stroke-soft)] shadow-lg">
                      <div className="border-b border-[var(--glass-stroke-soft)] px-3 py-2">
                        <p className="truncate text-sm font-medium text-[var(--glass-text-primary)]">{username}</p>
                      </div>
                      <div className="flex flex-col py-1">
                        <Link
                          className="px-3 py-2 text-sm text-[var(--glass-text-secondary)] hover:bg-[var(--glass-bg-hover)]"
                          to="/profile/api-config"
                          onClick={() => setAvatarMenuOpen(false)}
                        >
                          {t('nav:modelSettings')}
                        </Link>
                        <Link
                          className="px-3 py-2 text-sm text-[var(--glass-text-secondary)] hover:bg-[var(--glass-bg-hover)]"
                          to="/profile/account"
                          onClick={() => setAvatarMenuOpen(false)}
                        >
                          {t('nav:account')}
                        </Link>
                        {isAdmin ? (
                          <Link
                            className="px-3 py-2 text-sm text-[var(--glass-text-secondary)] hover:bg-[var(--glass-bg-hover)]"
                            to="/admin"
                            onClick={() => setAvatarMenuOpen(false)}
                          >
                            {t('nav:admin')}
                          </Link>
                        ) : null}
                      </div>
                      <div className="border-t border-[var(--glass-stroke-soft)] px-3 py-2">
                        <LanguageSwitcher />
                      </div>
                      <div className="border-t border-[var(--glass-stroke-soft)]">
                        <button
                          className="w-full px-3 py-2 text-left text-sm text-[var(--glass-tone-danger-fg)] hover:bg-[var(--glass-bg-hover)]"
                          type="button"
                          disabled={loggingOut}
                          onClick={handleLogout}
                        >
                          {t('nav:logout')}
                        </button>
                      </div>
                    </div>
                  ) : null}
                </div>
              </>
            ) : isBootstrapping ? (
              <LanguageSwitcher />
            ) : (
              <>
                <Link className="glass-btn-base glass-btn-secondary h-8 px-3" to="/auth/signin">
                  {t('nav:signin')}
                </Link>
                <Link className="glass-btn-base glass-btn-ghost h-8 px-3" to="/auth/signup">
                  {t('nav:signup')}
                </Link>
                <LanguageSwitcher />
              </>
            )}
          </nav>
        </div>
      </div>

      {/* Mobile menu */}
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
                {username ? (
                  <div className="px-4 py-2 text-sm font-medium text-[var(--glass-text-primary)]">{username}</div>
                ) : null}
                <Link
                  className="glass-btn-base glass-btn-ghost h-11 min-h-[44px] w-full justify-start px-4"
                  to="/workspace"
                  onClick={closeMobileMenu}
                >
                  {t('nav:workspace')}
                </Link>
                <Link
                  className="glass-btn-base glass-btn-ghost h-11 min-h-[44px] w-full justify-start px-4"
                  to="/profile/api-config"
                  onClick={closeMobileMenu}
                >
                  {t('nav:modelSettings')}
                </Link>
                <Link
                  className="glass-btn-base glass-btn-ghost h-11 min-h-[44px] w-full justify-start px-4"
                  to="/profile/account"
                  onClick={closeMobileMenu}
                >
                  {t('nav:account')}
                </Link>
                {isAdmin ? (
                  <Link
                    className="glass-btn-base glass-btn-ghost h-11 min-h-[44px] w-full justify-start px-4"
                    to="/admin"
                    onClick={closeMobileMenu}
                  >
                    {t('nav:admin')}
                  </Link>
                ) : null}
                <button
                  className="glass-btn-base glass-btn-secondary h-11 min-h-[44px] w-full justify-start px-4"
                  type="button"
                  disabled={loggingOut}
                  onClick={handleLogout}
                >
                  {t('nav:logout')}
                </button>
              </>
            ) : isBootstrapping ? null : (
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
