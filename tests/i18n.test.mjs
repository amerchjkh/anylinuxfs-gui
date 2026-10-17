import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { parse } from 'svelte/compiler';
import { get } from 'svelte/store';
import { zhCN } from '../src/lib/i18n/zh-CN.ts';
import { resolveLocale, readLanguagePreference, saveLanguagePreference, translate, LANGUAGE_STORAGE_KEY } from '../src/lib/i18n/core.ts';
import { initializeLanguage, language, locale, t } from '../src/lib/i18n.ts';
import { mountOptionError } from '../src/lib/filesystems.ts';
import { parseError } from '../src/lib/errors.ts';

test('explicit settings override system preferences; system mode falls back to English', () => {
	assert.equal(resolveLocale('en', ['zh-CN']), 'en');
	assert.equal(resolveLocale('zh-CN', ['en-US']), 'zh-CN');
	for (const code of ['zh-CN', 'zh-Hans', 'zh-TW', 'ZH-hk', 'zh']) assert.equal(resolveLocale('system', [code]), 'zh-CN');
	assert.equal(resolveLocale('system', ['en-GB', 'zh-CN']), 'en');
	assert.equal(resolveLocale('system', ['fr-FR', 'zh-CN']), 'zh-CN');
	assert.equal(resolveLocale('system', ['fr-FR']), 'en');
	assert.equal(resolveLocale('system', []), 'en');
});

test('preferences survive reload and tolerate invalid or unavailable storage', () => {
	const data = new Map();
	const storage = { getItem: (key) => data.get(key) ?? null, setItem: (key, value) => data.set(key, value) };
	assert.equal(readLanguagePreference(storage), 'system');
	for (const setting of ['en', 'zh-CN', 'system']) {
		saveLanguagePreference(storage, setting);
		assert.equal(readLanguagePreference(storage), setting);
	}
	data.set(LANGUAGE_STORAGE_KEY, '<script>');
	assert.equal(readLanguagePreference(storage), 'system');
	assert.equal(readLanguagePreference({ getItem() { throw new Error('disabled'); } }), 'system');
	assert.doesNotThrow(() => saveLanguagePreference({ setItem() { throw new Error('disabled'); } }, 'en'));
});

test('interpolation preserves paths and inserted values; unknown CLI output remains intact', () => {
	const device = '/dev/disk6s3';
	assert.equal(translate('en', '{device} mounted successfully.', { device }), `${device} mounted successfully.`);
	assert.equal(translate('zh-CN', '{device} mounted successfully.', { device }), `${device} 已成功挂载。`);
	assert.equal(translate('zh-CN', '{count} lines', { count: 0 }), '0 行');
	assert.equal(translate('zh-CN', 'Mount failed: Device is busy - close any applications using it and try again'), '挂载失败：设备正忙，请关闭正在使用它的应用后重试');
	for (const text of ['Linux: custom error {count}', 'compress=zstd,subvolid=256', '/Volumes/Disks', '__proto__']) {
		assert.equal(translate('zh-CN', text, { count: 42 }), text);
	}
	assert.equal(translate('zh-CN', '{device} mounted successfully.', { device: '{count}' }), '{count} 已成功挂载。');
});

test('every translation preserves the English placeholder set', () => {
	const placeholders = (text) => [...text.matchAll(/\{([a-zA-Z][a-zA-Z0-9_]*)\}/g)].map((match) => match[1]).sort();
	for (const [english, chinese] of Object.entries(zhCN)) {
		assert.ok(chinese.trim(), english);
		assert.deepEqual(placeholders(chinese), placeholders(english), english);
		assert.equal(translate('en', english), english);
	}
});

test('every static UI translation call has a catalog entry', () => {
	function files(directory) { return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => entry.isDirectory() ? files(join(directory, entry.name)) : [join(directory, entry.name)]); }
	let count = 0;
	for (const file of files(new URL('../src', import.meta.url).pathname).filter((file) => file.endsWith('.svelte'))) {
		const ast = parse(readFileSync(file, 'utf8'), { modern: true });
		function visit(node) {
			if (!node || typeof node !== 'object') return;
			if (node.type === 'CallExpression' && node.callee?.name === '$t' && node.arguments[0]?.type === 'Literal') {
				assert.ok(Object.hasOwn(zhCN, node.arguments[0].value), `${file}: ${node.arguments[0].value}`);
				count++;
			}
			for (const [key, value] of Object.entries(node)) {
				if (key === 'metadata' || key === 'parent') continue;
				if (Array.isArray(value)) value.forEach(visit); else if (value && typeof value === 'object') visit(value);
			}
		}
		visit(ast.fragment); visit(ast.instance);
	}
	assert.ok(count > 200);
});

test('locale changes never alter validation strings or machine error classification', () => {
	const message = mountOptionError('xfs', false, 'compress=zstd');
	assert.match(message, /Btrfs/);
	assert.match(translate('zh-CN', message), /仅适用于 Btrfs/);
	const error = parseError('Mount failed: wrong fs type');
	assert.equal(error.code, 'MOUNT_FAILED');
	assert.equal(error.details, 'Mount failed: wrong fs type');
	assert.equal(translate('zh-CN', error.message), '挂载文件系统失败');
	assert.equal(translate('en', error.message), 'Failed to mount filesystem');
});

test('live switching updates subscribers, follows system changes, persists, and cleans up', (context) => {
	const originalWindow = Object.getOwnPropertyDescriptor(globalThis, 'window');
	const originalNavigator = Object.getOwnPropertyDescriptor(globalThis, 'navigator');
	const data = new Map([[LANGUAGE_STORAGE_KEY, 'zh-CN']]);
	const listeners = new Map();
	const navigation = { languages: ['en-US'], language: 'en-US' };
	Object.defineProperty(globalThis, 'navigator', { configurable: true, value: navigation });
	Object.defineProperty(globalThis, 'window', { configurable: true, value: {
		localStorage: { getItem: (key) => data.get(key) ?? null, setItem: (key, value) => data.set(key, value) },
		addEventListener: (name, listener) => listeners.set(name, listener),
		removeEventListener: (name, listener) => { assert.equal(listeners.get(name), listener); listeners.delete(name); }
	} });
	context.after(() => {
		if (originalWindow) Object.defineProperty(globalThis, 'window', originalWindow); else delete globalThis.window;
		if (originalNavigator) Object.defineProperty(globalThis, 'navigator', originalNavigator); else delete globalThis.navigator;
		language.set('system');
	});
	const stop = initializeLanguage();
	const seen = [];
	const unsubscribe = locale.subscribe((value) => seen.push(value));
	assert.equal(get(t)('Disks'), '磁盘');
	language.set('en');
	assert.equal(get(t)('Disks'), 'Disks');
	assert.equal(data.get(LANGUAGE_STORAGE_KEY), 'en');
	language.set('system');
	navigation.languages = ['zh-CN'];
	listeners.get('languagechange')();
	assert.equal(get(t)('Disks'), '磁盘');
	assert.deepEqual(seen, ['zh-CN', 'en', 'zh-CN']);
	stop(); unsubscribe();
	assert.equal(listeners.size, 0);
});
