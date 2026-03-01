import { FormEvent, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';

import { register } from '../../api/auth';
import { GlassButton, GlassField, GlassInput, GlassSurface } from '../../components/ui/primitives';

export function SignUp() {
  const { t } = useTranslation('auth');
  const navigate = useNavigate();
  const [name, setName] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setLoading(true);
    setError(null);
    try {
      await register(name, password);
      navigate('/workspace');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Sign up failed');
    } finally {
      setLoading(false);
    }
  }

  return (
    <main className="page-shell py-12">
      <GlassSurface className="mx-auto max-w-md" variant="modal">
        <form className="space-y-4" onSubmit={handleSubmit}>
          <div>
            <h1 className="text-2xl font-semibold text-[var(--glass-text-primary)]">{t('createAccount')}</h1>
            <p className="mt-1 text-sm text-[var(--glass-text-secondary)]">{t('joinPlatform')}</p>
          </div>
          <GlassField id="signup-name" label={t('phoneNumber')} required>
            <GlassInput
              id="signup-name"
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder={t('phoneNumberPlaceholder')}
              autoComplete="username"
            />
          </GlassField>
          <GlassField id="signup-password" label={t('password')} required>
            <GlassInput
              id="signup-password"
              type="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              placeholder={t('passwordMinPlaceholder')}
              autoComplete="new-password"
            />
          </GlassField>
          {error ? <p className="text-sm text-[var(--glass-tone-danger-fg)]">{error}</p> : null}
          <GlassButton type="submit" variant="primary" className="w-full" loading={loading}>
            {loading ? t('signupButtonLoading') : t('signupButton')}
          </GlassButton>
        </form>
      </GlassSurface>
    </main>
  );
}
