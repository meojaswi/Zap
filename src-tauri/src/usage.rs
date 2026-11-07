//! Optional anonymous check-in, at most once a day, so the makers can count active users.
//!
//! Sends only a random install ID (made on this PC, not tied to the hardware or the person) and
//! the app version. Turning it off in Settings deletes the ID.

use std::thread;
use std::time::Duration;

use tauri::async_runtime::block_on;
use tauri::AppHandle;

use crate::store::shared;

const ENDPOINT: &str = "https://zap-rho-five.vercel.app/api/ping";
const CHECK_EVERY: Duration = Duration::from_secs(60 * 60);

/// Checks hourly whether today's check-in is still due, for as long as the setting is on.
pub fn start(app: AppHandle) {
    // Dev builds aren't real installs, so they'd only inflate the count.
    if cfg!(debug_assertions) {
        return;
    }
    let version = app.package_info().version.to_string();
    thread::spawn(move || loop {
        if let Err(e) = check_in(&version) {
            eprintln!("Zap: usage check-in failed: {e}");
        }
        thread::sleep(CHECK_EVERY);
    });
}

fn check_in(version: &str) -> Result<(), String> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let id = {
        let mut data = shared().data.write().unwrap();
        if !data.settings.usage_ping || data.last_ping.as_deref() == Some(today.as_str()) {
            return Ok(());
        }
        if data.install_id.is_empty() {
            data.install_id = uuid::Uuid::new_v4().to_string();
        }
        data.install_id.clone()
    };

    block_on(send(&id, version))?;

    {
        let mut data = shared().data.write().unwrap();
        // Skip recording if the setting was switched off (and the ID deleted) while sending.
        if data.install_id != id {
            return Ok(());
        }
        data.last_ping = Some(today);
    }
    shared().save()
}

async fn send(id: &str, version: &str) -> Result<(), String> {
    // reqwest is built without a default TLS crypto provider (to share the updater's setup), and
    // panics if none is installed. The updater installs ring the same way, on first use.
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    let client = reqwest::Client::builder().timeout(Duration::from_secs(15)).build().map_err(|e| e.to_string())?;
    let response = client
        .post(ENDPOINT)
        .json(&serde_json::json!({ "id": id, "version": version }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("server responded {}", response.status()))
    }
}
