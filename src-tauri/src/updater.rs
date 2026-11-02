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
use crate::{data_changed, TrayState};

const CHECK_EVERY: Duration = Duration::from_secs(60 * 60);

/// The newest version found so far, ready to install.
static PENDING: Mutex<Option<Update>> = Mutex::new(None);

pub fn pending_version() -> Option<String> {
    PENDING.lock().unwrap().as_ref().map(|update| update.version.clone())
}

/// Checks at startup and then hourly, for as long as the setting is on.
pub fn start(app: AppHandle) {
    // Dev builds aren't real releases, so "Check now" is the only way they look for updates.
    if cfg!(debug_assertions) {
        return;
    }
    thread::spawn(move || loop {
        if shared().data.read().unwrap().settings.check_updates {
            if let Err(e) = block_on(check(&app)) {
                eprintln!("Zap: update check failed: {e}");
            }
        }
        thread::sleep(CHECK_EVERY);
    });
}

/// Returns the available version, if any. Notifies the user once per new version.
pub async fn check(app: &AppHandle) -> Result<Option<String>, String> {
    let update = app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())?;
    let version = update.as_ref().map(|update| update.version.clone());
    let is_new = version.is_some() && version != pending_version();
    *PENDING.lock().unwrap() = update;

    if let Some(tray) = app.try_state::<TrayState>() {
        let label = version.as_ref().map_or("Check for updates".to_string(), |v| format!("Install Zap {v}"));
        let _ = tray.update.set_text(label);
    }
    if is_new {
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
    update.download_and_install(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
    app.restart()
}
