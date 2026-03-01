import { useTranslation } from 'react-i18next';

export function LanguageSwitcher() {
  const { i18n, t } = useTranslation('common');

  return (
    <select
      value={i18n.language}
      aria-label={t('language.select')}
      className="glass-select-base h-9 min-w-28 px-3 text-xs"
      onChange={(event) => {
        const next = event.target.value;
        void i18n.changeLanguage(next);
      }}
    >
      <option value="zh">{t('language.zh')}</option>
      <option value="en">{t('language.en')}</option>
    </select>
  );
}
