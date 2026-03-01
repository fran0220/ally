import { FormEvent, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';

import { login } from '../../api/auth';
import { GlassButton, GlassField, GlassInput, GlassSurface } from '../../components/ui/primitives';

export function SignIn() {
  const { t } = useTranslation('auth');
  const navigate = useNavigate();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setLoading(true);
    setError(null);
    try {
      await login(username, password);
      navigate('/workspace');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Login failed');
    } finally {
      setLoading(false);
    }
  }

  return (
    <main className="page-shell py-12">
      <GlassSurface className="mx-auto max-w-md" variant="modal">
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <h1 className="text-2xl font-semibold text-[var(--glass-text-primary)]">{t('welcomeBack')}</h1>
            <p className="mt-1 text-sm text-[var(--glass-text-secondary)]">{t('loginTo')}</p>
          </div>
          <GlassField id="signin-username" label={t('phoneNumber')} required>
            <GlassInput
              id="signin-username"
              value={username}
              onChange={(event) => setUsername(event.target.value)}
              placeholder={t('phoneNumberPlaceholder')}
              autoComplete="username"
            />
          </GlassField>
          <GlassField id="signin-password" label={t('password')} required>
            <GlassInput
              id="signin-password"
              type="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              placeholder={t('passwordPlaceholder')}
              autoComplete="current-password"
            />
          </GlassField>
          {error ? <p className="text-sm text-[var(--glass-tone-danger-fg)]">{error}</p> : null}
          <GlassButton type="submit" variant="primary" className="w-full" loading={loading}>
            {loading ? t('loginButtonLoading') : t('loginButton')}
          </GlassButton>
        </form>
      </GlassSurface>
    </main>
  );
}
