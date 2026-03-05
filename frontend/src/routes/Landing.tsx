import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';

export function Landing() {
  const { t } = useTranslation(['landing', 'nav']);

  return (
    <main className="page-shell py-12 md:py-16">
      <section className="glass-surface relative overflow-hidden p-8 md:p-12">
        <div className="absolute -top-28 -right-16 h-72 w-72 rounded-full bg-cyan-300/30 blur-3xl" />
        <div className="absolute -bottom-24 -left-16 h-64 w-64 rounded-full bg-blue-300/25 blur-3xl" />
        <div className="relative z-10 max-w-3xl">
          <p className="text-xs uppercase tracking-[0.24em] text-[var(--glass-text-tertiary)]">React + Vite + Axum</p>
          <h1 className="mt-4 text-4xl font-semibold leading-tight text-[var(--glass-text-primary)] md:text-5xl">
            {t('landing:title')}
          </h1>
          <p className="mt-4 text-base text-[var(--glass-text-secondary)] md:text-lg">{t('landing:subtitle')}</p>
          <div className="mt-8 flex flex-col gap-3 sm:flex-row sm:flex-wrap">
            <Link className="glass-btn-base glass-btn-primary h-11 w-full justify-center px-6 sm:w-auto" to="/auth/signin">
              {t('nav:signin')}
            </Link>
            <Link className="glass-btn-base glass-btn-secondary h-11 w-full justify-center px-6 sm:w-auto" to="/auth/signup">
              {t('nav:signup')}
            </Link>
            <Link className="glass-btn-base glass-btn-soft h-11 w-full justify-center px-6 sm:w-auto" to="/workspace">
              {t('landing:enterWorkspace')}
            </Link>
          </div>
          <div className="mt-8 grid gap-3 md:grid-cols-3">
            <article className="glass-kpi p-4">
              <h3 className="text-sm font-semibold">{t('landing:features.character.title')}</h3>
              <p className="mt-1 text-xs text-[var(--glass-text-secondary)]">{t('landing:features.character.description')}</p>
            </article>
            <article className="glass-kpi p-4">
              <h3 className="text-sm font-semibold">{t('landing:features.storyboard.title')}</h3>
              <p className="mt-1 text-xs text-[var(--glass-text-secondary)]">{t('landing:features.storyboard.description')}</p>
            </article>
            <article className="glass-kpi p-4">
              <h3 className="text-sm font-semibold">{t('landing:features.world.title')}</h3>
              <p className="mt-1 text-xs text-[var(--glass-text-secondary)]">{t('landing:features.world.description')}</p>
            </article>
          </div>
        </div>
      </section>
    </main>
  );
}
