// Hide the console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod clipboard;
mod crypto;
mod expand;
mod hook;
mod input;
mod store;
mod updater;
mod usage;
mod winutil;

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::image::Image;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WindowEvent, Wry};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_notification::NotificationExt;

use store::{normalize_app, shared, Settings, Snippet};

/// The window that was focused before the popup opened; the snippet gets pasted back into it.
static TARGET_WINDOW: AtomicIsize = AtomicIsize::new(0);
static HOTKEY_ERROR: Mutex<Option<String>> = Mutex::new(None);
/// Lets code without an AppHandle (like expand::deliver) tell the windows that data changed.
static APP: OnceLock<AppHandle> = OnceLock::new();
/// Bumped on every tray left click, so a pending single click can tell a double click followed it.
static TRAY_CLICKS: AtomicU64 = AtomicU64::new(0);
/// Windows reports a double click as press, release, double click, release: the last release isn't a new click.
static SKIP_NEXT_RELEASE: AtomicBool = AtomicBool::new(false);

/// A snippet waiting for its {input:...} values in the fill-in window.
struct FillRequest {
    snippet: Snippet,
    /// Trigger characters still in the target app, deleted once the values are entered.
    backspaces: usize,
    /// The window the snippet goes into.
    target: isize,
    /// The trigger's swallowed last key, typed back if the user cancels.
    swallowed: Option<char>,
}

static FILL: Mutex<Option<FillRequest>> = Mutex::new(None);

#[derive(Serialize, Clone)]
struct FillPrompt {
    name: String,
    labels: Vec<String>,
}

const PAUSED_ICON: &[u8] = include_bytes!("../icons/tray-paused.png");
const UPDATE_ICON: &[u8] = include_bytes!("../icons/tray-update.png");
const TOOLTIP: &str = "Zap — type, zap, done";
const PAUSED_TOOLTIP: &str = "Zap — paused";
const UPDATE_TOOLTIP: &str = "Zap — restart to update";

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
    update_ready: Option<String>,
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        // Closing only hides the window, so the page keeps its last tab. Reopening starts on Snippets.
        if !window.is_visible().unwrap_or(true) {
            let _ = app.emit_to("main", "main-reopened", ());
        }
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
    refresh_tray(app);
    data_changed(app);
}

/// Sets the tray icon and tooltip for Zap's state; being paused matters most, then a ready update.
pub(crate) fn refresh_tray(app: &AppHandle) {
    let Some(tray) = app.tray_by_id("zap") else { return };
    let (icon, tooltip) = if shared().paused.load(Ordering::Relaxed) {
        (Image::from_bytes(PAUSED_ICON).ok(), PAUSED_TOOLTIP)
    } else if updater::ready_version().is_some() {
        (Image::from_bytes(UPDATE_ICON).ok(), UPDATE_TOOLTIP)
    } else {
        (app.default_window_icon().cloned(), TOOLTIP)
    };
    let _ = tray.set_icon(icon);
    let _ = tray.set_tooltip(Some(tooltip));
}

/// Left click opens Zap, but only once the double-click time has passed without a second click,
/// so a double click (pause) doesn't also open the window.
fn tray_left_click(app: &AppHandle) {
    if SKIP_NEXT_RELEASE.swap(false, Ordering::SeqCst) {
        return;
    }
    let click = TRAY_CLICKS.fetch_add(1, Ordering::SeqCst) + 1;
    let wait = Duration::from_millis(u64::from(unsafe {
        windows_sys::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime()
    }));
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(wait);
        if TRAY_CLICKS.load(Ordering::SeqCst) == click {
            show_main(&app);
        }
    });
}

fn tray_double_click(app: &AppHandle) {
    TRAY_CLICKS.fetch_add(1, Ordering::SeqCst);
    SKIP_NEXT_RELEASE.store(true, Ordering::SeqCst);
    set_paused_inner(app, !shared().paused.load(Ordering::Relaxed));
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
        update_ready: updater::ready_version(),
    }
}

#[tauri::command]
fn save_snippet(app: AppHandle, mut snippet: Snippet) -> Result<Snippet, String> {
    snippet.trigger = snippet.trigger.trim().to_string();
    snippet.name = snippet.name.trim().to_string();
    if snippet.name.is_empty() {
        snippet.name = snippet.trigger.clone();
    }
    snippet.group = snippet.group.trim().to_string();
    snippet.html = snippet.html.take().filter(|h| !h.trim().is_empty());
    if snippet.sensitive && snippet.html.is_some() {
        return Err("Formatted snippets are pasted, so they can't be Sensitive".into());
    }
    {
        let mut data = shared().data.write().unwrap();
        store::validate(&data.snippets, &snippet, &data.settings.secondary_prefix)?;
        if snippet.id.is_empty() {
            snippet.id = store::new_id();
        }
        match data.snippets.iter_mut().find(|s| s.id == snippet.id) {
            Some(existing) => {
                // The editor doesn't send stats, so keep the ones already counted.
                keep_stats(existing, &mut snippet);
                *existing = snippet.clone();
            }
            None => data.snippets.push(snippet.clone()),
        }
    }
    save_and_notify(&app)?;
    Ok(snippet)
}

fn keep_stats(from: &Snippet, to: &mut Snippet) {
    to.uses = from.uses;
    to.chars_saved = from.chars_saved;
    to.last_used = from.last_used.clone();
}

/// Counts one use of a snippet for the stats, then saves and refreshes the windows.
pub(crate) fn record_use(id: &str, chars_saved: usize) {
    {
        let mut data = shared().data.write().unwrap();
        if !data.settings.keep_stats {
            return;
        }
        let Some(snippet) = data.snippets.iter_mut().find(|s| s.id == id) else { return };
        snippet.uses += 1;
        snippet.chars_saved += chars_saved as u64;
        snippet.last_used = Some(chrono::Local::now().format("%Y-%m-%d").to_string());
    }
    if let Err(e) = shared().save() {
        eprintln!("Zap: couldn't save stats: {e}");
    }
    if let Some(app) = APP.get() {
        data_changed(app);
    }
}

#[tauri::command]
fn reset_stats(app: AppHandle) -> Result<(), String> {
    for snippet in shared().data.write().unwrap().snippets.iter_mut() {
        snippet.uses = 0;
        snippet.chars_saved = 0;
        snippet.last_used = None;
    }
    save_and_notify(&app)
}

#[tauri::command]
fn delete_snippet(app: AppHandle, id: String) -> Result<(), String> {
    shared().data.write().unwrap().snippets.retain(|s| s.id != id);
    save_and_notify(&app)
}

#[tauri::command]
fn update_settings(app: AppHandle, mut settings: Settings) -> Result<(), String> {
    settings.hotkey = settings.hotkey.trim().to_string();
    settings.typing_wpm = settings.typing_wpm.clamp(5, 300);
    settings.secondary_prefix = settings.secondary_prefix.trim().to_string();
    store::validate_prefix(&shared().data.read().unwrap().snippets, &settings.secondary_prefix)?;
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
    {
        let mut data = shared().data.write().unwrap();
        // Turning the check-in off forgets the ID, so turning it back on starts as a new install.
        if !settings.usage_ping {
            data.install_id.clear();
            data.last_ping = None;
        }
        data.settings = settings;
    }
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
    if !expand::input_labels(&snippet.content).is_empty() {
        request_fill(snippet, 0, target, None);
        return Ok(());
    }
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(60));
        winutil::focus_window(target);
        thread::sleep(Duration::from_millis(120));
        expand::deliver(&snippet, 0);
    });
    Ok(())
}

/// Shows the fill-in window for a snippet with {input:...} placeholders.
pub(crate) fn request_fill(snippet: Snippet, backspaces: usize, target: isize, swallowed: Option<char>) {
    let Some(app) = APP.get() else { return };
    let prompt = FillPrompt { name: snippet.name.clone(), labels: expand::input_labels(&snippet.content) };
    *FILL.lock().unwrap() = Some(FillRequest { snippet, backspaces, target, swallowed });
    if let Some(window) = app.get_webview_window("fill") {
        let _ = window.center();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit_to("fill", "fill-request", prompt);
    }
}

/// Hides the fill-in window and returns focus to the app the snippet goes into.
fn finish_fill(app: &AppHandle, then: impl FnOnce(FillRequest) + Send + 'static) {
    let Some(request) = FILL.lock().unwrap().take() else { return };
    if let Some(window) = app.get_webview_window("fill") {
        let _ = window.hide();
    }
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(60));
        winutil::focus_window(request.target);
        thread::sleep(Duration::from_millis(120));
        then(request);
    });
}

#[tauri::command]
fn fill_submit(app: AppHandle, values: HashMap<String, String>) {
    finish_fill(&app, move |request| expand::deliver_filled(&request.snippet, request.backspaces, &values));
}

#[tauri::command]
fn fill_cancel(app: AppHandle) {
    cancel_fill(&app);
}

fn cancel_fill(app: &AppHandle) {
    finish_fill(app, |request| {
        if let Some(key) = request.swallowed {
            input::type_text(&key.to_string());
        }
    });
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
    // Fill-ins show as [Label], since their values are only known when the snippet is used.
    let shown = expand::input_labels(&content).into_iter().map(|l| (l.clone(), format!("[{l}]"))).collect();
    expand::fill_inputs(&expand::render(&content), &shown)
}

/// A snippet as written to and read from an export file. Ids and usage stats stay out of the file,
/// and fields are optional so hand-written files import too.
#[derive(Serialize, Deserialize)]
struct FileSnippet {
    #[serde(default)]
    name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    group: String,
    trigger: String,
    content: String,
    #[serde(default)]
    sensitive: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    html: Option<String>,
}

#[derive(Serialize)]
struct ImportResult {
    imported: usize,
    skipped: Vec<String>,
}

// Async so the blocking file dialog runs off the main thread.
fn export_json(snippets: &[Snippet]) -> Result<String, String> {
    let file: Vec<FileSnippet> = snippets
        .iter()
        .map(|s| FileSnippet {
            name: s.name.clone(),
            group: s.group.clone(),
            trigger: s.trigger.clone(),
            content: s.content.clone(),
            sensitive: s.sensitive,
            html: s.html.clone(),
        })
        .collect();
    serde_json::to_string_pretty(&file).map_err(|e| e.to_string())
}

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
    let json = export_json(&snippets)?;
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
    let incoming: Vec<FileSnippet> =
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
                group: item.group.trim().to_string(),
                // Sensitive snippets are always typed as plain text.
                html: item.html.filter(|h| !h.trim().is_empty() && !item.sensitive),
                uses: 0,
                chars_saved: 0,
                last_used: None,
            };
            if snippet.name.trim().is_empty() {
                snippet.name = snippet.trigger.clone();
            }
            if let Err(e) = store::validate(&data.snippets, &snippet, &data.settings.secondary_prefix) {
                result.skipped.push(format!("{}: {e}", snippet.trigger));
                continue;
            }
            if snippet.id.is_empty() {
                snippet.id = store::new_id();
                data.snippets.push(snippet);
            } else if let Some(existing) = data.snippets.iter_mut().find(|s| s.id == snippet.id) {
                keep_stats(existing, &mut snippet);
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
    updater::check(&app, true).await
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
        updater::check(&app, true).await
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
        .tooltip(TOOLTIP)
        .menu(&menu)
        // The menu is for right click; left click opens Zap and double left click pauses it.
        .show_menu_on_left_click(false)
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
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } => {
                tray_left_click(tray.app_handle())
            }
            TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } => tray_double_click(tray.app_handle()),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn main() {
    let app = tauri::Builder::default()
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
            let handle = app.handle().clone();
            let settings = shared().data.read().unwrap().settings.clone();
            if let Err(e) = register_hotkey(&handle, &settings.hotkey) {
                *HOTKEY_ERROR.lock().unwrap() = Some(e);
            }
            hook::set_enabled(settings.inline_enabled);
            build_tray(app)?;
            updater::start(handle.clone());
            usage::start(handle.clone());
            let _ = APP.set(handle.clone());
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title(&format!("Zap {}", app.package_info().version));
            }
            let updated_quietly = updater::finish_quiet_restart(&handle);
            if !updated_quietly && !std::env::args().any(|a| a == "--minimized") {
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
            // Clicking away from the fill-in window cancels it, like Esc.
            WindowEvent::Focused(false) if window.label() == "fill" => cancel_fill(window.app_handle()),
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
            install_update,
            reset_stats,
            fill_submit,
            fill_cancel
        ])
        .build(tauri::generate_context!())
        .expect("error while building Zap");

    // Windows are created before setup() runs and can call get_data as soon as they load,
    // so the store has to be ready before the app starts running.
    store::init(app.path().app_data_dir().expect("no app data folder"));
    app.run(|_, _| {});
}

#[cfg(test)]
mod tests {
    use super::{export_json, store::Snippet, PAUSED_ICON, UPDATE_ICON};

    #[test]
    fn update_tray_icon_decodes() {
        let icon = tauri::image::Image::from_bytes(UPDATE_ICON).unwrap();
        assert_eq!((icon.width(), icon.height()), (64, 64));
    }

    #[test]
    fn paused_tray_icon_decodes() {
        let icon = tauri::image::Image::from_bytes(PAUSED_ICON).unwrap();
        assert_eq!((icon.width(), icon.height()), (64, 64));
    }

    #[test]
    fn export_leaves_out_ids_and_usage_stats() {
        let snippet = Snippet {
            id: "secret-id".into(),
            name: "Aadhar".into(),
            trigger: "`aadhar".into(),
            content: "123456789012".into(),
            sensitive: true,
            group: "Personal".into(),
            html: None,
            uses: 12,
            chars_saved: 60,
            last_used: Some("2026-11-06".into()),
        };
        let json = export_json(&[snippet]).unwrap();
        let fields: serde_json::Value = serde_json::from_str(&json).unwrap();
        let keys: Vec<&str> = fields[0].as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["content", "group", "name", "sensitive", "trigger"]);
    }
}
