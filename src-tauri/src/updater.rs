//! Checks GitHub Releases for a newer signed version, tells the user, and installs it on request.
//!
//! The only network request Zap makes. It fetches latest.json and sends no snippets or usage data.

use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use tauri::async_runtime::block_on;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::store::shared;
use crate::{data_changed, winutil, TrayState};

const CHECK_EVERY: Duration = Duration::from_secs(60 * 60);
/// Automatic installs wait for this long without keyboard or mouse input.
const IDLE_BEFORE_INSTALL: Duration = Duration::from_secs(3 * 60);
const CALM_POLL: Duration = Duration::from_secs(30);

/// The version an automatic install is already under way for, so hourly checks don't repeat it.
static AUTO_INSTALLING: Mutex<Option<String>> = Mutex::new(None);

/// A downloaded update waiting to be installed: its version and installer.
static READY: Mutex<Option<(String, Vec<u8>)>> = Mutex::new(None);

/// The newest version found so far, ready to install.
static PENDING: Mutex<Option<Update>> = Mutex::new(None);

/// The version that's downloaded and only needs a restart, if any.
pub fn ready_version() -> Option<String> {
    READY.lock().unwrap().as_ref().map(|(version, _)| version.clone())
}

fn update_tray_label(app: &AppHandle) {
    let Some(tray) = app.try_state::<TrayState>() else { return };
    let label = match (ready_version(), pending_version()) {
        (Some(version), _) => format!("Restart to update to Zap {version}"),
        (None, Some(version)) => format!("Install Zap {version}"),
        (None, None) => "Check for updates".to_string(),
    };
    let _ = tray.update.set_text(label);
}

pub fn pending_version() -> Option<String> {
    PENDING.lock().unwrap().as_ref().map(|update| update.version.clone())
}

/// Checks at startup and then hourly, for as long as the setting is on.
pub fn start(app: AppHandle) {
    // Dev builds aren't real releases, so "Check now" is the only way they look for updates.
    if cfg!(debug_assertions) {
        return;
    }
    thread::spawn(move || {
        let mut at_startup = true;
        loop {
            let (enabled, auto) = {
                let settings = &shared().data.read().unwrap().settings;
                (settings.check_updates, settings.auto_update)
            };
            if enabled {
                // With automatic installs there's nothing to ask, so no "update available" notification.
                match block_on(check(&app, !auto)) {
                    Ok(_) if auto => auto_install(&app, at_startup),
                    Ok(_) => {}
                    Err(e) => eprintln!("Zap: update check failed: {e}"),
                }
            }
            at_startup = false;
            thread::sleep(CHECK_EVERY);
        }
    });
}

/// Downloads the pending update in the background, then installs it at a calm moment.
fn auto_install(app: &AppHandle, at_startup: bool) {
    let Some(update) = PENDING.lock().unwrap().clone() else { return };
    {
        let mut installing = AUTO_INSTALLING.lock().unwrap();
        if installing.as_deref() == Some(update.version.as_str()) {
            return;
        }
        *installing = Some(update.version.clone());
    }
    let app = app.clone();
    thread::spawn(move || {
        let result = download_and_wait(&app, &update, at_startup).and_then(|bytes| install_quietly(&update, &bytes));
        // Only reached if it didn't install; the next hourly check tries again.
        if let Err(e) = result {
            eprintln!("Zap: automatic update stopped: {e}");
        }
        *AUTO_INSTALLING.lock().unwrap() = None;
    });
}

fn download_and_wait(app: &AppHandle, update: &Update, at_startup: bool) -> Result<Vec<u8>, String> {
    let bytes = block_on(update.download(|_, _| {}, || {})).map_err(|e| e.to_string())?;
    // Show "restart to update" right away; the install below still happens by itself if nobody clicks.
    *READY.lock().unwrap() = Some((update.version.clone(), bytes.clone()));
    update_tray_label(app);
    crate::refresh_tray(app);
    data_changed(app);
    let mut just_started = at_startup;
    loop {
        if !shared().data.read().unwrap().settings.auto_update {
            return Err("turned off in Settings".into());
        }
        if pending_version().as_deref() != Some(update.version.as_str()) {
            return Err("a different version is now pending".into());
        }
        if is_calm(app, just_started) {
            return Ok(bytes);
        }
        just_started = false;
        thread::sleep(CALM_POLL);
    }
}

/// Safe to restart Zap: none of its windows are open (so no half-edited snippet is lost), and the
/// user is away from the PC or Zap has only just started.
fn is_calm(app: &AppHandle, just_started: bool) -> bool {
    let window_open = ["main", "popup", "fill"]
        .iter()
        .any(|label| app.get_webview_window(label).is_some_and(|w| w.is_visible().unwrap_or(true)));
    !window_open && (just_started || winutil::idle_time() >= IDLE_BEFORE_INSTALL)
}

fn install_quietly(update: &Update, bytes: &[u8]) -> Result<(), String> {
    set_quiet_restart(Some(update.version.clone()))?;
    // On Windows this runs the installer and exits Zap; the installer restarts it.
    update.install(bytes).map_err(|e| {
        let _ = set_quiet_restart(None);
        e.to_string()
    })
}

fn set_quiet_restart(version: Option<String>) -> Result<(), String> {
    shared().data.write().unwrap().quiet_restart = version;
    shared().save()
}

/// After an automatic update: returns true if Zap should stay in the tray, and tells the user.
pub fn finish_quiet_restart(app: &AppHandle) -> bool {
    let Some(version) = shared().data.write().unwrap().quiet_restart.take() else { return false };
    if let Err(e) = shared().save() {
        eprintln!("Zap: {e}");
    }
    let _ = app.notification().builder().title(format!("Zap updated to {version}")).body("Updated automatically in the background.").show();
    true
}

/// Returns the available version, if any. With `notify`, tells the user once per new version.
pub async fn check(app: &AppHandle, notify: bool) -> Result<Option<String>, String> {
    let update = app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())?;
    let version = update.as_ref().map(|update| update.version.clone());
    let is_new = version.is_some() && version != pending_version();
    *PENDING.lock().unwrap() = update;
    // A download for an older version is no use once a newer one is out.
    {
        let mut ready = READY.lock().unwrap();
        if ready.as_ref().is_some_and(|(v, _)| Some(v) != version.as_ref()) {
            *ready = None;
        }
    }
    update_tray_label(app);
    crate::refresh_tray(app);
    if is_new && notify {
        let _ = app
            .notification()
            .builder()
            .title(format!("Zap {} is available", version.as_deref().unwrap_or_default()))
            .body("Open Zap from the tray to install the update.")
            .show();
    }
    data_changed(app);
    Ok(version)
}

/// Downloads and installs the pending update, then restarts Zap. The signature is verified
/// before the installer runs, so a tampered download is rejected.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let update = PENDING.lock().unwrap().clone().ok_or_else(|| "No update available".to_string())?;
    // Already downloaded: just install it.
    if let Some((version, bytes)) = READY.lock().unwrap().clone() {
        if version == update.version {
            update.install(&bytes).map_err(|e| e.to_string())?;
            app.restart();
        }
    }
    update.download_and_install(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
    app.restart()
}
