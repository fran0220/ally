import { useTranslation } from 'react-i18next';

import ApiConfigTab from '../../components/profile/ApiConfigTab';

export function Profile() {
  const { t } = useTranslation('profile');

  return (
    <main className="page-shell py-10">
      <header className="mb-6">
        <h1 className="glass-page-title">{t('personalAccount')}</h1>
        <p className="glass-page-subtitle">{t('apiConfig')}</p>
      </header>

      <section className="glass-surface-elevated overflow-hidden">
        <ApiConfigTab />
      </section>
    </main>
  );
}
