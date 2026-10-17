import { derived, writable } from 'svelte/store';
import { readLanguagePreference, resolveLocale, saveLanguagePreference, translate } from './i18n/core.ts';
import type { LanguageSetting, Parameters } from './i18n/core.ts';

export const language = writable<LanguageSetting>('system');
const systemLanguages = writable<readonly string[]>([]);
export const locale = derived([language, systemLanguages], ([$language, $languages]) =>
	resolveLocale($language, $languages)
);
export const t = derived(locale, ($locale) =>
	(message: string, parameters?: Parameters) => translate($locale, message, parameters)
);

// Called on mount, never during SSR or module initialization.
export function initializeLanguage(): () => void {
	const updateSystemLanguage = () => systemLanguages.set(navigator.languages.length ? navigator.languages : [navigator.language]);
	updateSystemLanguage();
	try {
		language.set(readLanguagePreference(window.localStorage));
	} catch {
		language.set('system');
	}
	const unsubscribe = language.subscribe((setting) => {
		try {
			saveLanguagePreference(window.localStorage, setting);
		} catch {
			// Access to localStorage itself can throw in a restricted webview.
		}
	});
	window.addEventListener('languagechange', updateSystemLanguage);
	return () => {
		unsubscribe();
		window.removeEventListener('languagechange', updateSystemLanguage);
	};
}
