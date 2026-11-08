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
    /// Group the snippet is listed under, like "Work"; empty means ungrouped.
    #[serde(default)]
    pub group: String,
    /// Formatted version (sanitized HTML) for apps that accept it. `content` stays the plain text,
    /// used for search, previews, stats and apps that only take text.
    #[serde(default)]
    pub html: Option<String>,
    /// Usage stats: only counts and the date are kept, never what was typed around the snippet.
    #[serde(default)]
    pub uses: u64,
    /// Characters the user didn't have to type, summed over every use.
    #[serde(default)]
    pub chars_saved: u64,
    /// Local date of the last use, as YYYY-MM-DD.
    #[serde(default)]
    pub last_used: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub hotkey: String,
    /// Expand triggers as you type them anywhere (needs a keyboard hook).
    pub inline_enabled: bool,
    /// Executable names (e.g. "valorant.exe") where inline expansion is disabled.
    pub excluded_apps: Vec<String>,
    /// Look for new versions on GitHub at startup and about once an hour.
    pub check_updates: bool,
    /// Typing speed in words per minute, used to turn characters saved into time saved.
    pub typing_wpm: u32,
    /// Count how often each snippet is used. Off means nothing is recorded.
    pub keep_stats: bool,
    /// Letters that stand in for a trigger's leading symbol, so `email also works as zzemail on
    /// sites that block `. Empty means off.
    pub secondary_prefix: String,
    /// Send an anonymous daily check-in (random ID + version) so the makers can count active users.
    pub usage_ping: bool,
    /// Download updates in the background and install them when the user is away.
    pub auto_update: bool,
    /// Copies the selection into a new snippet from any app; empty turns it off.
    pub save_hotkey: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self { hotkey: "Ctrl+Shift+Space".into(), inline_enabled: true, excluded_apps: Vec::new(), check_updates: true, typing_wpm: 40, keep_stats: true, secondary_prefix: String::new(), usage_ping: true, auto_update: true, save_hotkey: "Ctrl+Alt+Shift+S".into() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Data {
    pub snippets: Vec<Snippet>,
    pub settings: Settings,
    /// Random ID for the anonymous daily check-in; empty until the first one, cleared when it's turned off.
    pub install_id: String,
    /// Local date of the last successful check-in, as YYYY-MM-DD.
    pub last_ping: Option<String>,
    /// Version being installed automatically; on the next start Zap stays in the tray and says so.
    pub quiet_restart: Option<String>,
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
        group: String::new(),
        html: None,
        uses: 0,
        chars_saved: 0,
        last_used: None,
    };
    Data {
        snippets: vec![
            snippet("Today's date", "`date", "{date}"),
            // `now rather than `datetime: `date is part of `datetime, so they couldn't coexist.
            snippet("Date and time", "`now", "{datetime}"),
            snippet("Current time", "`time", "{time}"),
            snippet("Day of the week", "`day", "{day}"),
            snippet("Paste clipboard", "`clipboard", "{clipboard}"),
            snippet("Random ID", "`uuid", "{uuid}"),
        ],
        settings: Settings::default(),
        install_id: String::new(),
        last_ping: None,
        quiet_restart: None,
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

/// The trigger with the secondary prefix in place of its leading symbol: `email with "zz" is zzemail.
/// None when no prefix is set, or when the trigger starts with a letter or digit, since those work
/// everywhere already (and prefix + trigger would contain the trigger, so it could never fire).
pub fn secondary_trigger(trigger: &str, prefix: &str) -> Option<String> {
    let first = trigger.chars().next()?;
    if prefix.is_empty() || first.is_alphanumeric() {
        return None;
    }
    Some(format!("{prefix}{}", &trigger[first.len_utf8()..]))
}

/// Every text that fires a snippet: its trigger, plus its secondary trigger when there is one.
pub fn trigger_forms(trigger: &str, prefix: &str) -> Vec<String> {
    let mut forms = vec![trigger.to_string()];
    forms.extend(secondary_trigger(trigger, prefix));
    forms
}

/// Checks a secondary prefix: letters or digits only, and no clashes among the triggers it creates.
pub fn validate_prefix(snippets: &[Snippet], prefix: &str) -> Result<(), String> {
    if prefix.is_empty() {
        return Ok(());
    }
    if prefix.chars().count() < 2 || !prefix.chars().all(char::is_alphanumeric) {
        return Err("The secondary prefix needs at least 2 letters or digits, like zz".into());
    }
    for snippet in snippets {
        validate(snippets, snippet, prefix).map_err(|e| format!("With the prefix {prefix}: {e}"))?;
    }
    Ok(())
}

/// Checks a snippet against the others before saving it, including their secondary triggers.
pub fn validate(snippets: &[Snippet], candidate: &Snippet, prefix: &str) -> Result<(), String> {
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
    let mine = trigger_forms(trigger, prefix);
    for other in snippets.iter().filter(|s| s.id != candidate.id) {
        for theirs in trigger_forms(&other.trigger, prefix) {
            for form in &mine {
                if *form == theirs {
                    return Err(format!("{form} is already used by \"{}\"", other.name));
                }
                // While typing the longer trigger, the shorter one would fire first.
                if theirs.contains(form.as_str()) || form.contains(theirs.as_str()) {
                    return Err(format!(
                        "{form} overlaps with {theirs} (one contains the other), so only the shorter one would ever fire"
                    ));
                }
            }
        }
    }
    Ok(())
}

pub fn normalize_app(name: &str) -> String {
    let name = name.trim().to_lowercase();
    if name.is_empty() || name.ends_with(".exe") { name } else { format!("{name}.exe") }
}

#[cfg(test)]
mod tests {
    use super::{seed, validate, validate_prefix};

    #[test]
    fn snippets_saved_before_stats_still_load() {
        let snippet: super::Snippet = serde_json::from_str(r#"{"id":"1","trigger":"`a","content":"x"}"#).unwrap();
        assert_eq!((snippet.uses, snippet.chars_saved, snippet.last_used), (0, 0, None));
        assert_eq!(snippet.group, "");
        assert_eq!(snippet.html, None);
        let settings: super::Settings = serde_json::from_str(r#"{"hotkey":"F13"}"#).unwrap();
        assert_eq!(settings.typing_wpm, 40);
        assert!(settings.keep_stats);
        assert!(settings.auto_update);
    }

    #[test]
    fn seed_snippets_are_valid_together() {
        let snippets = seed().snippets;
        for snippet in &snippets {
            assert_eq!(validate(&snippets, snippet, ""), Ok(()), "{}", snippet.trigger);
        }
        assert_eq!(validate_prefix(&snippets, "zz"), Ok(()));
    }

    fn snippet(id: &str, trigger: &str) -> super::Snippet {
        serde_json::from_str(&format!(r#"{{"id":"{id}","trigger":"{trigger}","content":"x"}}"#)).unwrap()
    }

    #[test]
    fn secondary_trigger_replaces_a_leading_symbol_only() {
        use super::secondary_trigger;
        assert_eq!(secondary_trigger("`email", "zz").as_deref(), Some("zzemail"));
        assert_eq!(secondary_trigger("$addr", "qq").as_deref(), Some("qqaddr"));
        assert_eq!(secondary_trigger("email", "zz"), None);
        assert_eq!(secondary_trigger("`email", ""), None);
    }

    #[test]
    fn prefix_must_be_letters_and_cannot_create_clashes() {
        let snippets = vec![snippet("1", "`email"), snippet("2", "zzemailwork")];
        assert!(validate_prefix(&snippets, "z").is_err());
        assert!(validate_prefix(&snippets, "z z").is_err());
        assert!(validate_prefix(&snippets, "`;").is_err());
        // zzemail would sit inside zzemailwork, so `email's secondary trigger could never fire.
        assert!(validate_prefix(&snippets, "zz").is_err());
        assert_eq!(validate_prefix(&snippets, "qq"), Ok(()));
        // A new snippet is checked against secondary triggers too.
        assert!(validate(&[snippet("1", "`email")], &snippet("3", "zzemail"), "zz").is_err());
    }
}
