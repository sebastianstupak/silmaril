// Localization system for Silmaril Editor
// Simple key-based translation with interpolation

import en from './locales/en';

export type TranslationKey = keyof typeof en;
type Locale = Record<string, string>;

const locales: Record<string, Locale> = { en };
let currentLocale = 'en';

/** Register a locale. Call before setLocale. */
export function registerLocale(code: string, translations: Locale) {
  locales[code] = translations;
}

/** Set the active locale. */
export function setLocale(code: string) {
  if (!locales[code]) {
    console.warn(`[i18n] locale '${code}' not registered, falling back to 'en'`);
    currentLocale = 'en';
    return;
  }
  currentLocale = code;
}

/** Get the active locale code. */
export function getLocale(): string {
  return currentLocale;
}

/** Get available locale codes. */
export function getAvailableLocales(): string[] {
  return Object.keys(locales);
}

/**
 * Translate a key with optional interpolation.
 *
 * Usage:
 *   t('menu.file')              → "File"
 *   t('welcome', { name: 'X' }) → "Welcome, X"
 */
export function t(key: string, params?: Record<string, string | number>): string {
  const locale = locales[currentLocale] ?? locales.en;
  let text = locale[key] ?? locales.en[key] ?? key;

  if (params) {
    for (const [k, v] of Object.entries(params)) {
      text = text.replace(`{${k}}`, String(v));
    }
  }

  return text;
}
