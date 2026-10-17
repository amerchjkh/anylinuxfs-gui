import { zhCN } from './zh-CN.ts';

export type Locale = 'en' | 'zh-CN';
export type LanguageSetting = 'system' | Locale;
export type Parameters = Readonly<Record<string, string | number>>;
export const LANGUAGE_STORAGE_KEY = 'anylinuxfs.language';

export function isLanguageSetting(value: unknown): value is LanguageSetting {
	return value === 'system' || value === 'en' || value === 'zh-CN';
}

export function resolveLocale(setting: LanguageSetting, languages: readonly string[]): Locale {
	if (setting !== 'system') return setting;
	for (const language of languages) {
		if (/^zh(?:-|$)/i.test(language)) return 'zh-CN';
		if (/^en(?:-|$)/i.test(language)) return 'en';
	}
	return 'en';
}

export function readLanguagePreference(storage: Pick<Storage, 'getItem'>): LanguageSetting {
	try {
		const value = storage.getItem(LANGUAGE_STORAGE_KEY);
		return isLanguageSetting(value) ? value : 'system';
	} catch {
		return 'system';
	}
}

export function saveLanguagePreference(storage: Pick<Storage, 'setItem'>, value: LanguageSetting): void {
	try {
		storage.setItem(LANGUAGE_STORAGE_KEY, value);
	} catch {
		// Language switching still works when webview storage is unavailable.
	}
}

// Only known application prefixes are translated. Raw CLI diagnostics are preserved.
const prefixes = [
	'CLI not found: ', 'CLI execution failed: ', 'Mount failed: ', 'Unmount failed: ',
	'Configuration error: ', 'IO error: ', 'JSON error: ', 'TOML parse error: ',
	'TOML serialize error: ', 'Task error: ', 'Shell error: ', 'Permission denied: ', 'Unverified: '
];

export function translate(locale: Locale, message: string, parameters: Parameters = {}): string {
	if (Object.hasOwn(zhCN, message)) {
		const text = locale === 'zh-CN' ? zhCN[message] : message;
		return text.replace(/\{([a-zA-Z][a-zA-Z0-9_]*)\}/g, (placeholder, name: string) =>
			Object.hasOwn(parameters, name) ? String(parameters[name]) : placeholder
		);
	}
	if (locale === 'zh-CN') {
		for (const prefix of prefixes) {
			if (message.startsWith(prefix)) {
				return zhCN[prefix] + translate(locale, message.slice(prefix.length));
			}
		}
	}
	return message;
}
