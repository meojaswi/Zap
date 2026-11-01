//! Snippets + settings, kept in memory and saved encrypted to disk.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{OnceLock, RwLock};

use serde::{Deserialize, Serialize};

use crate::crypto;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub trigger: String,
    pub content: String,
    /// Sensitive snippets are typed out instead of pasted, so they never touch the clipboard.
    #[serde(default)]
    pub sensitive: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub hotkey: String,
    /// Expand triggers as you type them anywhere (needs a keyboard hook).
    pub inline_enabled: bool,
    /// Executable names (e.g. "valorant.exe") where inline expansion is disabled.
    pub excluded_apps: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self { hotkey: "Ctrl+Shift+Space".into(), inline_enabled: true, excluded_apps: Vec::new() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Data {
    pub snippets: Vec<Snippet>,
    pub settings: Settings,
}

pub struct Shared {
    pub data: RwLock<Data>,
    pub paused: AtomicBool,
    path: PathBuf,
}

static SHARED: OnceLock<Shared> = OnceLock::new();

pub fn shared() -> &'static Shared {
    SHARED.get().expect("store not initialised")
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn init(dir: PathBuf) {
    let path = dir.join("zap.dat");
    let (data, fresh) = load(&path);
    let _ = SHARED.set(Shared { data: RwLock::new(data), paused: AtomicBool::new(false), path });
    if fresh {
        if let Err(e) = shared().save() {
            eprintln!("Zap: {e}");
        }
    }
}

fn load(path: &PathBuf) -> (Data, bool) {
    let Ok(bytes) = fs::read(path) else { return (seed(), true) };
    let parsed = crypto::unprotect(&bytes)
        .and_then(|plain| serde_json::from_slice::<Data>(&plain).map_err(|e| e.to_string()));
    match parsed {
        Ok(data) => (data, false),
        Err(e) => {
            // Keep the unreadable file around instead of overwriting it.
            eprintln!("Zap: {e}");
            let _ = fs::rename(path, path.with_extension("dat.bad"));
            (seed(), true)
        }
    }
}

fn seed() -> Data {
    let snippet = |name: &str, trigger: &str, content: &str| Snippet {
        id: new_id(),
        name: name.into(),
        trigger: trigger.into(),
        content: content.into(),
        sensitive: false,
    };
    Data {
        snippets: vec![
            snippet("Personal email", "`pemail", "your.email@example.com"),
            snippet("Today's date", "`date", "{date}"),
            snippet("Go error check", "`iferr", "if err != nil {\n\treturn err\n}"),
        ],
        settings: Settings::default(),
    }
}

impl Shared {
    pub fn save(&self) -> Result<(), String> {
        let json = serde_json::to_vec(&*self.data.read().unwrap()).map_err(|e| e.to_string())?;
        let encrypted = crypto::protect(&json)?;
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        // Write to a temp file first so a crash mid-write can't corrupt the real file.
        let tmp = self.path.with_extension("dat.tmp");
        fs::write(&tmp, encrypted).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &self.path).map_err(|e| e.to_string())
    }
}

/// Checks a snippet against the others before saving it.
pub fn validate(snippets: &[Snippet], candidate: &Snippet) -> Result<(), String> {
    let trigger = candidate.trigger.as_str();
    if trigger.chars().count() < 2 {
        return Err("Trigger must be at least 2 characters".into());
    }
    if trigger.chars().any(char::is_whitespace) {
        return Err("Trigger can't contain spaces".into());
    }
    if candidate.content.is_empty() {
        return Err("Text can't be empty".into());
    }
    for other in snippets.iter().filter(|s| s.id != candidate.id) {
        if other.trigger == trigger {
            return Err(format!("{trigger} is already used by \"{}\"", other.name));
        }
        // While typing the longer trigger, the shorter one would fire first.
        if other.trigger.contains(trigger) || trigger.contains(other.trigger.as_str()) {
            return Err(format!(
                "{trigger} overlaps with {} (one contains the other), so only the shorter one would ever fire",
                other.trigger
            ));
        }
    }
    Ok(())
}

pub fn normalize_app(name: &str) -> String {
    let name = name.trim().to_lowercase();
    if name.is_empty() || name.ends_with(".exe") { name } else { format!("{name}.exe") }
}
