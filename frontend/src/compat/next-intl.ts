import { useTranslation } from 'react-i18next';

type TranslationValues = Record<string, string | number | boolean | Date>;

export function useLocale(): string {
  const { i18n } = useTranslation();
  return i18n.resolvedLanguage ?? i18n.language ?? 'en';
}

export function useTranslations(namespace?: string) {
  const { t } = useTranslation(namespace);

  return (key: string, values?: TranslationValues): string => {
    const translated = t(key, values as Record<string, unknown> | undefined);
    return typeof translated === 'string' ? translated : String(translated);
  };
}
