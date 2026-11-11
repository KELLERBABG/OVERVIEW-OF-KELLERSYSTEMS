/**
 * Dynamic Locale Manager for AmtFlow
 * Loads and caches translations on demand per language code.
 */

export interface LocaleDictionary {
  [key: string]: any;
}

const localeCache: Record<string, LocaleDictionary> = {};

export async function loadLocale(lang: string): Promise<LocaleDictionary> {
  const code = lang || 'de';
  if (localeCache[code]) {
    return localeCache[code];
  }

  try {
    const response = await fetch(`./locales/${code}.json`);
    if (!response.ok) {
      throw new Error(`Failed to load locale ${code}: ${response.statusText}`);
    }
    const data = await response.json();
    localeCache[code] = data;
    return data;
  } catch (err) {
    console.warn(`Fallback to German locale due to error:`, err);
    if (code !== 'de') {
      return loadLocale('de');
    }
    return {};
  }
}

export function getCurrentLocale(): string {
  return localStorage.getItem('amtflow_lang') || 'de';
}

export function setCurrentLocale(lang: string): void {
  localStorage.setItem('amtflow_lang', lang);
}
