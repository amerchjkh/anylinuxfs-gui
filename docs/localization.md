# Interface localization

The app supports English and Simplified Chinese. Open **Preferences → Language** to choose **Follow system**, **English**, or **简体中文**. Changes take effect immediately and the selection is saved in webview local storage under `anylinuxfs.language`.

In system mode, the first supported language in `navigator.languages` is used. Chinese variants use Simplified Chinese; English variants use English. Other languages fall back to English. The app listens for system `languagechange` events while running.

## Frontend messages

`src/lib/i18n/zh-CN.ts` maps English source messages to Chinese translations. English source messages also provide the fallback, so missing translations remain readable. No localization dependency is required.

In Svelte components, import the translation store and use it in markup:

```svelte
<script lang="ts">
    import { t } from '#lib/i18n';
</script>

<button>{$t('Refresh')}</button>
<span>{$t('{count} lines', { count: lines.length })}</span>
```

Use named placeholders for dynamic values. Values are substituted once as plain text; Svelte continues to escape the result. Do not use translated HTML. Reactive values such as navigation labels must use `$derived` so they update when the language changes.

Keep stored errors, status messages, validation results, and operation outcomes in their original form. Translate them where they are displayed, not before string matching or protocol handling. Notifications are translated when sent. Existing terminal output and log history are not rewritten when the language changes.

Known application error messages and prefixes have translations. Unknown CLI diagnostics, device paths, mount options, image names, user-created action descriptions, and configuration values are preserved. Built-in action descriptions can have catalog entries.

## Native menus and dialogs

`src-tauri/src/localization.rs` accepts only the `en` and `zh-CN` locale values. The frontend sends the resolved locale on mount and when it changes. Native menu item text is updated in place, preserving menu IDs, shortcuts, and enabled state. English is the native startup fallback until the webview initializes its saved or system preference.

Application-owned quit dialogs, key-file picker titles, administrator password prompts, and Terminal completion notices use the current locale when opened or generated. Already-open dialogs retain their original text. macOS-owned controls follow the operating system's language.

Adding a language requires updating both the frontend locale resolver/catalog and the native locale enum/labels.

## Validation

```sh
npm test
npm run build
npx svelte-check
cd src-tauri
cargo check --locked
cargo test --locked
```

The localization tests cover language resolution, storage failures, live switching, system-language changes, placeholder parity, catalog coverage for static UI calls, and preservation of machine-facing errors and CLI output. Rust tests cover accepted locale values and the hidden-input/cancellation behavior of both administrator prompts.
