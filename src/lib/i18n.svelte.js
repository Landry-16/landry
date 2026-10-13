// Interface strings. The French text is the key (gettext style); en.json maps it to English.
// Reading `i18n.locale` inside t() makes every call reactive to a language change.
import en from '../i18n/en.json';

export const i18n = $state({ locale: 'fr' });

const dicts = { en };

/** t('Aucun résultat pour « {q} »', { q }) */
export function t(key, vars) {
  const s = dicts[i18n.locale]?.[key] ?? key;
  return vars ? s.replace(/\{(\w+)\}/g, (m, k) => (k in vars ? String(vars[k]) : m)) : s;
}
