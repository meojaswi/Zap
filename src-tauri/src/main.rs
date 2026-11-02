// Hide the console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod clipboard;
mod crypto;
mod expand;
mod hook;
mod input;
mod store;
mod updater;
mod winutil;

use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WindowEvent, Wry};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_notification::NotificationExt;

use store::{normalize_app, shared, Settings, Snippet};

/// The window that was focused before the popup opened; the snippet gets pasted back into it.
static TARGET_WINDOW: AtomicIsize = AtomicIsize::new(0);
static HOTKEY_ERROR: Mutex<Option<String>> = Mutex::new(None);

struct TrayState {
    pause: CheckMenuItem<Wry>,
    update: MenuItem<Wry>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppData {
    snippets: Vec<Snippet>,
    settings: Settings,
    autostart: bool,
    paused: bool,
    hotkey_error: Option<String>,
    app_version: String,
    update_version: Option<String>,
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn toggle_popup(app: &AppHandle) {
    let Some(popup) = app.get_webview_window("popup") else { return };
    if popup.is_visible().unwrap_or(false) {
        let _ = popup.hide();
        return;
    }
    if shared().paused.load(Ordering::Relaxed) {
        return;
    }
    TARGET_WINDOW.store(winutil::foreground_window(), Ordering::SeqCst);
    let _ = popup.center();
    let _ = popup.show();
    let _ = popup.set_focus();
    let _ = popup.emit("popup-shown", ());
}

fn data_changed(app: &AppHandle) {
    let _ = app.emit("data-changed", ());
}

fn register_hotkey(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    let shortcuts = app.global_shortcut();
    let _ = shortcuts.unregister_all();
    shortcuts.register(hotkey).map_err(|e| format!("Couldn't use {hotkey} as the hotkey: {e}"))
}

fn set_paused_inner(app: &AppHandle, paused: bool) {
    shared().paused.store(paused, Ordering::Relaxed);
    hook::clear_buffer();
    if let Some(tray) = app.try_state::<TrayState>() {
        let _ = tray.pause.set_checked(paused);
    }
    data_changed(app);
}

fn save_and_notify(app: &AppHandle) -> Result<(), String> {
    shared().save()?;
    data_changed(app);
    Ok(())
}

#[tauri::command]
fn get_data(app: AppHandle) -> AppData {
    let data = shared().data.read().unwrap().clone();
    AppData {
        snippets: data.snippets,
        settings: data.settings,
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
        paused: shared().paused.load(Ordering::Relaxed),
        hotkey_error: HOTKEY_ERROR.lock().unwrap().clone(),
        app_version: app.package_info().version.to_string(),
        update_version: updater::pending_version(),
    }
}

#[tauri::command]
fn save_snippet(app: AppHandle, mut snippet: Snippet) -> Result<Snippet, String> {
    snippet.trigger = snippet.trigger.trim().to_string();
    snippet.name = snippet.name.trim().to_string();
    if snippet.name.is_empty() {
        snippet.name = snippet.trigger.clone();
    }
    {
        let mut data = shared().data.write().unwrap();
        store::validate(&data.snippets, &snippet)?;
        if snippet.id.is_empty() {
            snippet.id = store::new_id();
        }
        match data.snippets.iter_mut().find(|s| s.id == snippet.id) {
            Some(existing) => *existing = snippet.clone(),
            None => data.snippets.push(snippet.clone()),
        }
    }
    save_and_notify(&app)?;
    Ok(snippet)
}

#[tauri::command]
fn delete_snippet(app: AppHandle, id: String) -> Result<(), String> {
    shared().data.write().unwrap().snippets.retain(|s| s.id != id);
    save_and_notify(&app)
}

#[tauri::command]
fn update_settings(app: AppHandle, mut settings: Settings) -> Result<(), String> {
    settings.hotkey = settings.hotkey.trim().to_string();
    settings.excluded_apps = settings
        .excluded_apps
        .iter()
        .map(|a| normalize_app(a))
        .filter(|a| !a.is_empty())
        .collect();

    let old = shared().data.read().unwrap().settings.clone();
    if settings.hotkey != old.hotkey || HOTKEY_ERROR.lock().unwrap().is_some() {
        if let Err(e) = register_hotkey(&app, &settings.hotkey) {
            let _ = register_hotkey(&app, &old.hotkey);
            return Err(e);
        }
        *HOTKEY_ERROR.lock().unwrap() = None;
    }
    hook::set_enabled(settings.inline_enabled);
    shared().data.write().unwrap().settings = settings;
    save_and_notify(&app)
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let launcher = app.autolaunch();
    let result = if enabled { launcher.enable() } else { launcher.disable() };
    result.map_err(|e| e.to_string())
}

#[tauri::command]
fn set_paused(app: AppHandle, paused: bool) {
    set_paused_inner(&app, paused);
}

#[tauri::command]
fn insert_snippet(app: AppHandle, id: String) -> Result<(), String> {
    let snippet = shared()
        .data
        .read()
        .unwrap()
        .snippets
        .iter()
        .find(|s| s.id == id)
        .cloned()
        .ok_or_else(|| "Snippet not found".to_string())?;
    if let Some(popup) = app.get_webview_window("popup") {
        let _ = popup.hide();
    }
    let target = TARGET_WINDOW.load(Ordering::SeqCst);
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(60));
        winutil::focus_window(target);
        thread::sleep(Duration::from_millis(120));
        expand::deliver(&snippet, 0);
    });
    Ok(())
}

#[tauri::command]
fn hide_popup(app: AppHandle) {
    if let Some(popup) = app.get_webview_window("popup") {
        let _ = popup.hide();
    }
    winutil::focus_window(TARGET_WINDOW.load(Ordering::SeqCst));
}

#[tauri::command]
fn preview(content: String) -> String {
    expand::render(&content)
}

/// Snippets in an export file; ids are optional so hand-written files work too.
#[derive(Deserialize)]
struct ImportedSnippet {
    #[serde(default)]
    name: String,
    trigger: String,
    content: String,
    #[serde(default)]
    sensitive: bool,
}

#[derive(Serialize)]
struct ImportResult {
    imported: usize,
    skipped: Vec<String>,
}

// Async so the blocking file dialog runs off the main thread.
#[tauri::command]
async fn export_snippets(app: AppHandle) -> Result<Option<usize>, String> {
    let Some(path) = app
        .dialog()
        .file()
        .set_file_name("zap-snippets.json")
        .add_filter("JSON", &["json"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let snippets = shared().data.read().unwrap().snippets.clone();
    let json = serde_json::to_string_pretty(&snippets).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(Some(snippets.len()))
}

#[tauri::command]
async fn import_snippets(app: AppHandle) -> Result<Option<ImportResult>, String> {
    let Some(path) = app.dialog().file().add_filter("JSON", &["json"]).blocking_pick_file() else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let incoming: Vec<ImportedSnippet> =
        serde_json::from_str(&text).map_err(|e| format!("Not a Zap snippets file: {e}"))?;

    let mut result = ImportResult { imported: 0, skipped: Vec::new() };
    {
        let mut data = shared().data.write().unwrap();
        for item in incoming {
            // Same trigger as an existing snippet: update it in place.
            let id = data
                .snippets
                .iter()
                .find(|s| s.trigger == item.trigger.trim())
                .map(|s| s.id.clone())
                .unwrap_or_default();
            let mut snippet = Snippet {
                id,
                name: item.name,
                trigger: item.trigger.trim().to_string(),
                content: item.content,
                sensitive: item.sensitive,
            };
            if snippet.name.trim().is_empty() {
                snippet.name = snippet.trigger.clone();
            }
            if let Err(e) = store::validate(&data.snippets, &snippet) {
                result.skipped.push(format!("{}: {e}", snippet.trigger));
                continue;
            }
            if snippet.id.is_empty() {
                snippet.id = store::new_id();
                data.snippets.push(snippet);
            } else if let Some(existing) = data.snippets.iter_mut().find(|s| s.id == snippet.id) {
                *existing = snippet;
            }
            result.imported += 1;
        }
    }
    save_and_notify(&app)?;
    Ok(Some(result))
}

#[tauri::command]
async fn check_for_updates(app: AppHandle) -> Result<Option<String>, String> {
    updater::check(&app).await
}

#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    updater::install(&app).await
}

/// Tray item: installs a pending update, otherwise checks for one and reports the result.
async fn tray_update(app: AppHandle) {
    let result = if updater::pending_version().is_some() {
        updater::install(&app).await.map(|_| None)
    } else {
        updater::check(&app).await
    };
    let message = match result {
        Ok(None) => "You're on the latest version.".to_string(),
        Ok(Some(_)) => return, // check() has already notified about the new version
        Err(e) => format!("Couldn't update: {e}"),
    };
    let _ = app.notification().builder().title("Zap").body(message).show();
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Zap", true, None::<&str>)?;
    let search = MenuItem::with_id(app, "search", "Search snippets", true, None::<&str>)?;
    let pause = CheckMenuItem::with_id(app, "pause", "Pause Zap", true, false, None::<&str>)?;
    let update = MenuItem::with_id(app, "update", "Check for updates", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Zap", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &search, &pause, &update, &separator, &quit])?;
    app.manage(TrayState { pause, update });

    TrayIconBuilder::with_id("zap")
        .icon(app.default_window_icon().cloned().expect("app icon missing"))
        .tooltip("Zap — type, zap, done")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main(app),
            "search" => toggle_popup(app),
            "pause" => {
                let paused = app.state::<TrayState>().pause.is_checked().unwrap_or(false);
                set_paused_inner(app, paused);
            }
            "update" => {
                tauri::async_runtime::spawn(tray_update(app.clone()));
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick { .. } = event {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        // Must be first: a second launch just focuses the running Zap.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_popup(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            store::init(app.path().app_data_dir()?);
            let handle = app.handle().clone();
            let settings = shared().data.read().unwrap().settings.clone();
            if let Err(e) = register_hotkey(&handle, &settings.hotkey) {
                *HOTKEY_ERROR.lock().unwrap() = Some(e);
            }
            hook::set_enabled(settings.inline_enabled);
            build_tray(app)?;
            updater::start(handle.clone());
            if !std::env::args().any(|a| a == "--minimized") {
                show_main(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // Closing the manager keeps Zap running in the tray.
            WindowEvent::CloseRequested { api, .. } if window.label() == "main" => {
                api.prevent_close();
                let _ = window.hide();
            }
            WindowEvent::Focused(false) if window.label() == "popup" => {
                let _ = window.hide();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            get_data,
            save_snippet,
            delete_snippet,
            update_settings,
            set_autostart,
            set_paused,
            insert_snippet,
            hide_popup,
            preview,
            export_snippets,
            import_snippets,
            check_for_updates,
            install_update
        ])
        .run(tauri::generate_context!())
        .expect("error while running Zap");
}
