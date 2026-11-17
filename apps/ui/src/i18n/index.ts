import { createI18n } from 'vue-i18n';
import it from './locales/it.json';
import en from './locales/en.json';

export type Locale = 'it' | 'en';
export type MessageSchema = typeof it;

export const i18n = createI18n({
  legacy: false,
  locale: 'it',
  fallbackLocale: 'it',
  messages: { it, en },
});

export function setLocale(locale: Locale): void {
  i18n.global.locale.value = locale;
  document.documentElement.lang = locale;
}
