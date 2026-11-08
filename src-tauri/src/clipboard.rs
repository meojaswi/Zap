//! Raw Win32 clipboard access, so we can keep Zap's pastes out of clipboard history and cloud sync.

use std::ptr::{copy_nonoverlapping, null_mut, write_bytes};
use std::thread::sleep;
use std::time::Duration;

use windows_sys::Win32::Foundation::GlobalFree;
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, CountClipboardFormats, EmptyClipboard, GetClipboardData, GetClipboardSequenceNumber,
    IsClipboardFormatAvailable,
    OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows_sys::Win32::System::Ole::CF_UNICODETEXT;

const TEXT: u32 = CF_UNICODETEXT as u32;

// Formats Windows checks to decide whether an entry goes into Win+V history / cloud clipboard.
const PRIVATE_FORMATS: [&str; 3] =
    ["ExcludeClipboardContentFromMonitorProcessing", "CanIncludeInClipboardHistory", "CanUploadToCloudClipboard"];

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Another app may briefly hold the clipboard open, so retry a few times.
fn open() -> bool {
    for _ in 0..10 {
        if unsafe { OpenClipboard(null_mut()) } != 0 {
            return true;
        }
        sleep(Duration::from_millis(20));
    }
    false
}

pub fn get_text() -> Option<String> {
    unsafe {
        if IsClipboardFormatAvailable(TEXT) == 0 || !open() {
            return None;
        }
        let mut result = None;
        let handle = GetClipboardData(TEXT);
        if !handle.is_null() {
            let ptr = GlobalLock(handle) as *const u16;
            if !ptr.is_null() {
                let mut len = 0;
                while *ptr.add(len) != 0 {
                    len += 1;
                }
                result = Some(String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len)));
                GlobalUnlock(handle);
            }
        }
        CloseClipboard();
        result
    }
}

/// Changes every time the clipboard's contents change.
pub fn sequence() -> u32 {
    unsafe { GetClipboardSequenceNumber() }
}

/// True when the clipboard holds something we can't restore as text (an image, copied files, ...).
pub fn holds_non_text() -> bool {
    unsafe { CountClipboardFormats() > 0 && IsClipboardFormatAvailable(TEXT) == 0 }
}

unsafe fn set_global(format: u32, bytes: &[u8]) -> bool {
    let handle = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1));
    if handle.is_null() {
        return false;
    }
    let ptr = GlobalLock(handle) as *mut u8;
    if ptr.is_null() {
        GlobalFree(handle);
        return false;
    }
    write_bytes(ptr, 0, bytes.len().max(1));
    copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
    GlobalUnlock(handle);
    if SetClipboardData(format, handle).is_null() {
        GlobalFree(handle);
        return false;
    }
    true
}

fn text_bytes(text: &str) -> Vec<u8> {
    wide(text).iter().flat_map(|u| u.to_le_bytes()).collect()
}

/// Adds the formats that keep the current clipboard entry out of history and cloud sync.
/// The clipboard must be open.
unsafe fn mark_private() {
    for name in PRIVATE_FORMATS {
        let format = RegisterClipboardFormatW(wide(name).as_ptr());
        if format != 0 {
            set_global(format, &0u32.to_le_bytes());
        }
    }
}

/// Puts text on the clipboard, marked so it stays out of clipboard history and cloud sync.
pub fn set_text_private(text: &str) -> bool {
    unsafe {
        if !open() {
            return false;
        }
        EmptyClipboard();
        let ok = set_global(TEXT, &text_bytes(text));
        if ok {
            mark_private();
        }
        CloseClipboard();
        ok
    }
}

/// Windows' "HTML Format" payload: a header of byte offsets, then the fragment in a minimal document.
pub fn cf_html(fragment: &str) -> String {
    const PREFIX: &str = "<html><body>\r\n<!--StartFragment-->";
    const SUFFIX: &str = "<!--EndFragment-->\r\n</body></html>";
    let header = |a: usize, b: usize, c: usize, d: usize| {
        format!("Version:0.9\r\nStartHTML:{a:010}\r\nEndHTML:{b:010}\r\nStartFragment:{c:010}\r\nEndFragment:{d:010}\r\n")
    };
    let start_html = header(0, 0, 0, 0).len();
    let start_fragment = start_html + PREFIX.len();
    let end_fragment = start_fragment + fragment.len();
    let end_html = end_fragment + SUFFIX.len();
    format!("{}{PREFIX}{fragment}{SUFFIX}", header(start_html, end_html, start_fragment, end_fragment))
}

/// Puts formatted text on the clipboard with a plain-text version for apps that don't take HTML,
/// marked so it stays out of clipboard history and cloud sync.
pub fn set_html_private(html: &str, plain: &str) -> bool {
    let mut payload = cf_html(html).into_bytes();
    payload.push(0);
    unsafe {
        if !open() {
            return false;
        }
        EmptyClipboard();
        let format = RegisterClipboardFormatW(wide("HTML Format").as_ptr());
        let ok = format != 0 && set_global(TEXT, &text_bytes(plain)) && set_global(format, &payload);
        if ok {
            mark_private();
        }
        CloseClipboard();
        ok
    }
}

pub fn clear() {
    unsafe {
        if open() {
            EmptyClipboard();
            CloseClipboard();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::cf_html;

    fn offset(payload: &str, key: &str) -> usize {
        let start = payload.find(key).unwrap() + key.len();
        payload[start..start + 10].parse().unwrap()
    }

    #[test]
    fn html_format_offsets_point_at_the_fragment_and_document() {
        let fragment = "<b>Thanks</b> — Zap ⚡";
        let payload = cf_html(fragment);
        let (start_html, end_html) = (offset(&payload, "StartHTML:"), offset(&payload, "EndHTML:"));
        let (start, end) = (offset(&payload, "StartFragment:"), offset(&payload, "EndFragment:"));
        // Offsets are byte positions in the UTF-8 payload.
        assert_eq!(&payload.as_bytes()[start..end], fragment.as_bytes());
        assert!(payload[start_html..].starts_with("<html>"));
        assert_eq!(end_html, payload.len());
    }
}
