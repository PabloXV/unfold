import { readText, writeText } from '../storage';
import { en, type Messages } from './en';
import { zhTW } from './zh-TW';

export const LOCALES = { en, 'zh-TW': zhTW } satisfies Record<string, Messages>;
export type Locale = keyof typeof LOCALES;

const KEY = 'unfold:locale';

/** The saved choice, else Traditional Chinese for Taiwan, Hong Kong and zh-Hant browsers, else English. */
function initial(): Locale {
	const saved = readText(KEY);
	if (saved && saved in LOCALES) return saved as Locale;
	const traditional = navigator.languages.some((l) => /^zh-(TW|HK|MO|Hant)/i.test(l));
	return traditional ? 'zh-TW' : 'en';
}

let current = $state<Locale>(initial());
document.documentElement.lang = current;

export function locale(): Locale {
	return current;
}

export function setLocale(next: Locale): void {
	current = next;
	document.documentElement.lang = next;
	writeText(KEY, next);
}

/** The messages in the current language; read inside a template or effect to follow changes. */
export function t(): Messages {
	return LOCALES[current];
}
