import { en } from './en';
import type { SiteCopy } from './types';

/**
 * Locale resolution.
 *
 * One locale today. The indirection exists so adding a second one is a two-line
 * change here rather than a hunt through fourteen components, and so nothing
 * outside this folder ever imports a specific language.
 *
 * Adding a language:
 *   1. `cp en.ts fr.ts`, translate the values, keep every key.
 *   2. Add it to LOCALES below.
 *   3. `npm run check` fails until the file satisfies SiteCopy, so a half-done
 *      translation cannot ship silently.
 *
 * Routing comes later: Astro's i18n gives `/fr/` prefixes, and Starlight has its
 * own locale config for the wiki. Neither changes the shape of this module.
 */

export const DEFAULT_LOCALE = 'en' as const;

export const LOCALES = {
  en,
} satisfies Record<string, SiteCopy>;

export type Locale = keyof typeof LOCALES;

export function copyFor(locale: Locale = DEFAULT_LOCALE): SiteCopy {
  return LOCALES[locale];
}

/**
 * The active copy.
 *
 * Components import this directly while the site is single-locale. When routing
 * arrives they switch to `copyFor(Astro.currentLocale)`, which is a mechanical
 * change because the returned shape is identical.
 */
export const copy = copyFor();

export type { SiteCopy } from './types';
