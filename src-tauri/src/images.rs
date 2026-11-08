//! Pictures inside formatted snippets. Each one is its own DPAPI-encrypted file, so the main data
//! file stays small; a snippet's HTML only holds `data-zap-id`, filled in when the image is used.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::crypto;
use crate::store::shared;

const MAX_BYTES: usize = 1024 * 1024;
const ID_ATTRIBUTE: &str = "data-zap-id=\"";

/// Ids are UUIDs and become file names, so nothing else may get through.
fn valid_id(id: &str) -> bool {
    id.len() == 36 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

fn media_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else {
        None
    }
}

fn check(bytes: &[u8]) -> Result<(), String> {
    if media_type(bytes).is_none() {
        return Err("Only PNG and JPG images are supported.".to_string());
    }
    if bytes.len() > MAX_BYTES {
        return Err("That image is over 1 MB.".to_string());
    }
    Ok(())
}

fn path_of(id: &str) -> PathBuf {
    shared().images_dir().join(format!("{id}.dat"))
}

fn write(id: &str, bytes: &[u8]) -> Result<(), String> {
    let encrypted = crypto::protect(bytes)?;
    fs::create_dir_all(shared().images_dir()).map_err(|e| e.to_string())?;
    fs::write(path_of(id), encrypted).map_err(|e| e.to_string())
}

fn read(id: &str) -> Result<Vec<u8>, String> {
    if !valid_id(id) {
        return Err("Invalid image".to_string());
    }
    let encrypted = fs::read(path_of(id)).map_err(|_| "That image is missing.".to_string())?;
    crypto::unprotect(&encrypted)
}

fn data_url(bytes: &[u8]) -> Option<String> {
    Some(format!("data:{};base64,{}", media_type(bytes)?, STANDARD.encode(bytes)))
}

#[tauri::command]
pub fn save_image(data: String) -> Result<String, String> {
    let bytes = STANDARD.decode(data).map_err(|_| "Couldn't read that image.".to_string())?;
    check(&bytes)?;
    let id = uuid::Uuid::new_v4().to_string();
    write(&id, &bytes)?;
    Ok(id)
}

#[tauri::command]
pub fn get_image(id: String) -> Result<String, String> {
    data_url(&read(&id)?).ok_or_else(|| "That image is damaged.".to_string())
}

/// Calls `source` for each image in the HTML: its answer becomes the `src`, and with no answer the
/// image is left out.
fn rewrite(html: &str, mut source: impl FnMut(&str) -> Option<String>) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(start) = rest.find("<img") {
        let Some(len) = rest[start..].find('>') else { break };
        let tag = &rest[start..=start + len];
        out.push_str(&rest[..start]);
        if let Some(at) = tag.find(ID_ATTRIBUTE) {
            let id_start = at + ID_ATTRIBUTE.len();
            if let Some(id_len) = tag[id_start..].find('"') {
                if let Some(src) = source(&tag[id_start..id_start + id_len]) {
                    out.push_str(&tag[..at]);
                    out.push_str(&format!("src=\"{src}\""));
                    out.push_str(&tag[id_start + id_len + 1..]);
                }
            }
        }
        rest = &rest[start + len + 1..];
    }
    out.push_str(rest);
    out
}

fn referenced(html: &str) -> Vec<String> {
    let mut ids = Vec::new();
    rewrite(html, |id| {
        ids.push(id.to_string());
        None
    });
    ids
}

/// HTML ready for the clipboard. Browsers take the picture inside the HTML; desktop apps like Word
/// load it from a file, so those get short-lived decrypted copies, returned for deleting afterwards.
pub fn for_paste(html: &str) -> (String, Vec<PathBuf>) {
    if !html.contains(ID_ATTRIBUTE) {
        return (html.to_string(), Vec::new());
    }
    const BROWSERS: [&str; 7] = ["chrome.exe", "msedge.exe", "firefox.exe", "brave.exe", "opera.exe", "vivaldi.exe", "arc.exe"];
    let browser = crate::winutil::foreground_exe().is_some_and(|exe| BROWSERS.contains(&exe.as_str()));
    let mut temporary = Vec::new();
    let result = rewrite(html, |id| {
        let bytes = read(id).ok()?;
        if browser {
            return data_url(&bytes);
        }
        let extension = if media_type(&bytes)? == "image/png" { "png" } else { "jpg" };
        let path = std::env::temp_dir().join(format!("zap-{}.{extension}", uuid::Uuid::new_v4()));
        fs::write(&path, bytes).ok()?;
        let url = format!("file:///{}", path.display().to_string().replace('\\', "/").replace(' ', "%20"));
        temporary.push(path);
        Some(url)
    });
    (result, temporary)
}

/// The images a snippet uses, as base64, so an export is a complete backup.
pub fn for_export(html: Option<&str>) -> Option<HashMap<String, String>> {
    let images: HashMap<String, String> = referenced(html?)
        .into_iter()
        .filter_map(|id| Some((id.clone(), STANDARD.encode(read(&id).ok()?))))
        .collect();
    (!images.is_empty()).then_some(images)
}

/// Writes an import's images. Anything that isn't a valid PNG or JPG under the size limit is skipped.
pub fn restore(images: &HashMap<String, String>) {
    for (id, encoded) in images {
        if let (true, Ok(bytes)) = (valid_id(id), STANDARD.decode(encoded)) {
            if check(&bytes).is_ok() {
                let _ = write(id, &bytes);
            }
        }
    }
}

/// Deletes images no snippet uses any more. Runs at startup, when no editor can hold an unsaved one.
pub fn remove_unused() {
    let used: Vec<String> = shared().data.read().unwrap().snippets.iter().filter_map(|s| s.html.as_deref()).flat_map(referenced).collect();
    let Ok(entries) = fs::read_dir(shared().images_dir()) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(id) = name.strip_suffix(".dat") {
            if valid_id(id) && !used.iter().any(|u| u == id) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{check, referenced, rewrite, valid_id};

    const ID: &str = "3f9a1234-1234-1234-1234-123456789abc";

    #[test]
    fn only_uuids_can_become_file_names() {
        assert!(valid_id(ID));
        assert!(!valid_id("..\\..\\..\\..\\..\\windows\\win.ini-aaaaaaa"));
        assert!(!valid_id("short"));
    }

    #[test]
    fn only_small_png_and_jpg_files_are_accepted() {
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(check(&png).is_ok());
        assert!(check(&[0xFF, 0xD8, 0xFF, 0xE0]).is_ok());
        assert!(check(b"GIF89a").is_err());
        assert!(check(b"<svg></svg>").is_err());
        let mut big = png.to_vec();
        big.resize(1024 * 1024 + 1, 0);
        assert!(check(&big).is_err());
    }

    #[test]
    fn images_get_their_source_and_the_rest_stays() {
        let html = format!("<b>Hi</b><img data-zap-id=\"{ID}\" width=\"120\"> bye");
        assert_eq!(rewrite(&html, |_| Some("x.png".to_string())), "<b>Hi</b><img src=\"x.png\" width=\"120\"> bye");
        assert_eq!(rewrite(&html, |_| None), "<b>Hi</b> bye");
        assert_eq!(referenced(&html), [ID]);
        assert_eq!(rewrite("<b>none</b>", |_| None), "<b>none</b>");
    }
}
