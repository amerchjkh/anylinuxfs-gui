mod cache;
mod cli;
mod commands;
mod elevation;
mod error;
mod paths;
mod localization;

use localization::{current_locale, set_locale, NativeMenus};

pub use error::{AppError, AppResult};
pub use paths::{get_socket_path, get_log_path, get_log_paths, get_log_dir, COMMAND_TIMEOUT_SECS, MOUNT_TIMEOUT_SECS};

use std::sync::{Arc, Mutex};
use tauri::Manager;
use tauri::tray::TrayIconBuilder;
use tauri::menu::{MenuBuilder, MenuItemBuilder, AboutMetadataBuilder, SubmenuBuilder, PredefinedMenuItem};
use tauri_plugin_log::{Target, TargetKind};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use elevation::{
    cancel_elevation_operation, get_elevation_policy, set_elevation_mode, ElevationState,
};

struct UnmountMenuItem(tauri::menu::MenuItem<tauri::Wry>);

fn cancel_pending_elevation(app: &tauri::AppHandle) {
    let state = app.state::<Arc<ElevationState>>();
    let cancelled = state.cancel_all_pending();
    if cancelled > 0 {
        log::info!("Requested cleanup for {} pending elevation operation(s)", cancelled);
    }
}

#[cfg(target_os = "macos")]
fn set_dock_visible(visible: bool) {
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
    use objc2_foundation::MainThreadMarker;
    // Safe: tray/window event handlers always run on the main thread in Tauri
    let mtm = unsafe { MainThreadMarker::new_unchecked() };
    let app = NSApplication::sharedApplication(mtm);
    let policy = if visible {
        NSApplicationActivationPolicy::Regular
    } else {
        NSApplicationActivationPolicy::Accessory
    };
    app.setActivationPolicy(policy);
}

use commands::{
    list_disks, mount_disk, select_key_file, list_btrfs_subvolumes, unmount_disk, eject_disk, force_cleanup,
    get_mount_status, check_cli,
    get_log_content, list_log_files, start_log_stream, start_disk_watcher, stop_watchers,
    get_config, update_config,
    start_shell, write_shell, resize_shell, stop_shell,
    list_images, install_image, uninstall_image,
    list_packages, add_packages, remove_packages,
    list_custom_actions, create_custom_action, update_custom_action, delete_custom_action,
    WatcherState, PtyState,
};

fn confirm_quit(app: &tauri::AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let has_mounts = tokio::task::spawn_blocking(|| {
            commands::get_mount_status_sync()
                .map(|mounts| !mounts.is_empty())
                .unwrap_or(false)
        })
        .await
        .unwrap_or(false);

        if has_mounts {
            let locale = current_locale();
            let app_clone = app.clone();
            app.dialog()
                .message(locale.text("Filesystems are currently mounted. Quitting will NOT unmount them automatically.\n\nAre you sure you want to quit?", "当前仍有文件系统处于挂载状态，退出程序不会自动卸载它们。\n\n确定要退出吗？"))
                .title(locale.text("Confirm Quit", "确认退出"))
                .kind(MessageDialogKind::Warning)
                .buttons(MessageDialogButtons::OkCancelCustom(locale.text("Quit", "退出").into(), locale.text("Cancel", "取消").into()))
                .show(move |confirmed| {
                    if confirmed {
                        cancel_pending_elevation(&app_clone);
                        app_clone.exit(0);
                    }
                });
        } else {
            cancel_pending_elevation(&app);
            app.exit(0);
        }
    });
}

#[tauri::command]
fn set_tray_unmount_enabled(state: tauri::State<'_, UnmountMenuItem>, enabled: bool) -> Result<(), String> {
    state.0.set_enabled(enabled).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir { file_name: None }),
            Target::new(TargetKind::Webview),
        ]).build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(Arc::new(WatcherState::default()))
        .manage(Arc::new(Mutex::new(PtyState::default())))
        .setup(|app| {
            let elevation_config = app.path().app_config_dir()?.join("preferences.toml");
            app.manage(Arc::new(ElevationState::load(elevation_config)));

            let show_item = MenuItemBuilder::with_id("show", "Show").build(app)?;
            let unmount_item = MenuItemBuilder::with_id("unmount", "Unmount")
                .enabled(false)
                .build(app)?;
            app.manage(UnmountMenuItem(unmount_item.clone()));
            let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&show_item)
                .item(&unmount_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../../static/tray.png"))
                .expect("failed to load tray.png");
            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .icon_as_template(true)
                .menu(&menu)
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                                #[cfg(target_os = "macos")]
                                set_dock_visible(false);
                            } else {
                                #[cfg(target_os = "macos")]
                                set_dock_visible(true);
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                #[cfg(target_os = "macos")]
                                set_dock_visible(true);
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "unmount" => {
                            let app = app.clone();
                            tauri::async_runtime::spawn(async move {
                                let _ = unmount_disk(app, None).await;
                            });
                        }
                        "quit" => {
                            confirm_quit(app);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // App menu with keyboard shortcuts
            let app_quit_item = MenuItemBuilder::with_id("app_quit", "Quit anylinuxfs")
                .accelerator("CmdOrCtrl+Q")
                .build(app)?;
            let about_item = PredefinedMenuItem::about(app, Some("About anylinuxfs"), Some(AboutMetadataBuilder::new().build()))?;
            let hide_item = PredefinedMenuItem::hide(app, Some("Hide anylinuxfs"))?;
            let hide_others_item = PredefinedMenuItem::hide_others(app, Some("Hide Others"))?;
            let show_all_item = PredefinedMenuItem::show_all(app, Some("Show All"))?;
            let close_window_item = PredefinedMenuItem::close_window(app, Some("Close Window"))?;
            let undo_item = PredefinedMenuItem::undo(app, Some("Undo"))?;
            let redo_item = PredefinedMenuItem::redo(app, Some("Redo"))?;
            let cut_item = PredefinedMenuItem::cut(app, Some("Cut"))?;
            let copy_item = PredefinedMenuItem::copy(app, Some("Copy"))?;
            let paste_item = PredefinedMenuItem::paste(app, Some("Paste"))?;
            let select_all_item = PredefinedMenuItem::select_all(app, Some("Select All"))?;
            let app_menu = SubmenuBuilder::new(app, "anylinuxfs")
                .item(&about_item)
                .separator()
                .item(&hide_item)
                .item(&hide_others_item)
                .item(&show_all_item)
                .separator()
                .item(&app_quit_item)
                .build()?;
            let file_menu = SubmenuBuilder::new(app, "File")
                .item(&close_window_item)
                .build()?;
            // WKWebView uses native editing actions for shortcuts such as Cmd+V.
            // Keep these standard actions available, including in password inputs.
            let edit_menu = SubmenuBuilder::new(app, "Edit")
                .item(&undo_item)
                .item(&redo_item)
                .separator()
                .item(&cut_item)
                .item(&copy_item)
                .item(&paste_item)
                .item(&select_all_item)
                .build()?;
            let menu = MenuBuilder::new(app)
                .item(&app_menu)
                .item(&file_menu)
                .item(&edit_menu)
                .build()?;
            app.set_menu(menu)?;
            app.manage(NativeMenus {
                items: vec![
                    (show_item, "Show", "显示主窗口"),
                    (unmount_item, "Unmount", "卸载"),
                    (quit_item, "Quit", "退出"),
                    (app_quit_item, "Quit anylinuxfs", "退出 anylinuxfs"),
                ],
                predefined: vec![
                    (about_item, "About anylinuxfs", "关于 anylinuxfs"),
                    (hide_item, "Hide anylinuxfs", "隐藏 anylinuxfs"),
                    (hide_others_item, "Hide Others", "隐藏其他应用"),
                    (show_all_item, "Show All", "显示全部"),
                    (close_window_item, "Close Window", "关闭窗口"),
                    (undo_item, "Undo", "撤销"),
                    (redo_item, "Redo", "重做"),
                    (cut_item, "Cut", "剪切"),
                    (copy_item, "Copy", "复制"),
                    (paste_item, "Paste", "粘贴"),
                    (select_all_item, "Select All", "全选"),
                ],
                submenus: vec![(file_menu, "File", "文件"), (edit_menu, "Edit", "编辑")],
            });

            app.on_menu_event(move |app, event| {
                match event.id().as_ref() {
                    "close" => {
                        // Cmd+W: hide to tray instead of closing
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.hide();
                            #[cfg(target_os = "macos")]
                            set_dock_visible(false);
                        }
                    }
                    "app_quit" => {
                        confirm_quit(app);
                    }
                    _ => {}
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = window.hide();
                    #[cfg(target_os = "macos")]
                    set_dock_visible(false);
                }
                tauri::WindowEvent::Destroyed => {
                    let app = window.app_handle();
                    cancel_pending_elevation(app);
                    // Stop watchers
                    let watcher_state = app.state::<Arc<WatcherState>>();
                    watcher_state.shutdown();
                    // Stop PTY
                    let pty_arc = app.state::<Arc<Mutex<PtyState>>>().inner().clone();
                    let lock_result = pty_arc.lock();
                    if let Ok(mut state) = lock_result {
                        state.shutdown();
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_disks,
            mount_disk,
            select_key_file,
            list_btrfs_subvolumes,
            unmount_disk,
            eject_disk,
            force_cleanup,
            get_mount_status,
            check_cli,
            get_log_content,
            list_log_files,
            start_log_stream,
            start_disk_watcher,
            stop_watchers,
            get_config,
            update_config,
            start_shell,
            write_shell,
            resize_shell,
            stop_shell,
            list_images,
            install_image,
            uninstall_image,
            list_packages,
            add_packages,
            remove_packages,
            list_custom_actions,
            create_custom_action,
            update_custom_action,
            delete_custom_action,
            set_tray_unmount_enabled,
            set_locale,
            get_elevation_policy,
            set_elevation_mode,
            cancel_elevation_operation,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
