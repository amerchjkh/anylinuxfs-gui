//! Presentation language, independent of CLI output and administrator policy.
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{menu::{MenuItem, PredefinedMenuItem, Submenu}, Manager};

#[derive(Clone, Copy, Debug, serde::Deserialize, PartialEq, Eq)]
pub enum Locale {
    #[serde(rename = "en")]
    English,
    #[serde(rename = "zh-CN")]
    SimplifiedChinese,
}

impl Locale {
    pub fn text<'a>(self, english: &'a str, chinese: &'a str) -> &'a str {
        match self {
            Self::English => english,
            Self::SimplifiedChinese => chinese,
        }
    }
}

// English is the startup fallback. The webview synchronizes the saved preference
// (or system language) on mount, including when launching directly to the tray.
static CHINESE: AtomicBool = AtomicBool::new(false);

pub fn current_locale() -> Locale {
    if CHINESE.load(Ordering::Relaxed) { Locale::SimplifiedChinese } else { Locale::English }
}

pub struct NativeMenus {
    pub items: Vec<(MenuItem<tauri::Wry>, &'static str, &'static str)>,
    pub predefined: Vec<(PredefinedMenuItem<tauri::Wry>, &'static str, &'static str)>,
    pub submenus: Vec<(Submenu<tauri::Wry>, &'static str, &'static str)>,
}

#[tauri::command]
pub fn set_locale(app: tauri::AppHandle, locale: Locale) -> Result<(), String> {
    let menus = app.try_state::<NativeMenus>().ok_or("Native menus are not initialized")?;
    for (item, en, zh) in &menus.items {
        item.set_text(locale.text(en, zh)).map_err(|error| error.to_string())?;
    }
    for (item, en, zh) in &menus.predefined {
        item.set_text(locale.text(en, zh)).map_err(|error| error.to_string())?;
    }
    for (item, en, zh) in &menus.submenus {
        item.set_text(locale.text(en, zh)).map_err(|error| error.to_string())?;
    }
    CHINESE.store(locale == Locale::SimplifiedChinese, Ordering::Relaxed);
    Ok(())
}

pub fn authentication_script(locale: Locale) -> &'static str {
    locale.text(
        r#"#!/bin/bash
osascript -e 'Tell application "System Events" to display dialog "anylinuxfs requires administrator privileges." & return & return & "Enter your password:" with hidden answer default answer "" buttons {"Cancel", "OK"} default button "OK" with title "Authentication Required" with icon caution' -e 'text returned of result' 2>/dev/null
"#,
        r#"#!/bin/bash
osascript -e 'Tell application "System Events" to display dialog "anylinuxfs 需要管理员权限。" & return & return & "请输入你的登录密码：" with hidden answer default answer "" buttons {"取消", "确定"} default button "确定" cancel button "取消" with title "需要身份验证" with icon caution' -e 'text returned of result' 2>/dev/null
"#,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_supported_locale_values_are_accepted() {
        assert_eq!(serde_json::from_str::<Locale>("\"en\"").unwrap(), Locale::English);
        assert_eq!(serde_json::from_str::<Locale>("\"zh-CN\"").unwrap(), Locale::SimplifiedChinese);
        for value in ["\"system\"", "\"zh\"", "\"fr\"", "\"$(id)\"", "null"] {
            assert!(serde_json::from_str::<Locale>(value).is_err());
        }
    }

    #[test]
    fn authentication_prompts_preserve_hidden_input_and_cancellation() {
        for locale in [Locale::English, Locale::SimplifiedChinese] {
            let script = authentication_script(locale);
            assert!(script.starts_with("#!/bin/bash\n"));
            assert!(script.contains("with hidden answer default answer \"\""));
            assert!(script.contains("'text returned of result' 2>/dev/null"));
            assert!(!script.contains("${"));
        }
        assert!(authentication_script(Locale::English).contains("buttons {\"Cancel\", \"OK\"}"));
        assert!(authentication_script(Locale::SimplifiedChinese).contains("cancel button \"取消\""));
    }
}
