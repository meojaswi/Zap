//! Raw Win32 clipboard access, so we can keep Zap's pastes out of clipboard history and cloud sync.

use std::ptr::{copy_nonoverlapping, null_mut, write_bytes};
use std::thread::sleep;
use std::time::Duration;

use windows_sys::Win32::Foundation::GlobalFree;
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, CountClipboardFormats, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable,
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

/// Puts text on the clipboard, marked so it stays out of clipboard history and cloud sync.
pub fn set_text_private(text: &str) -> bool {
    let utf16 = wide(text);
    let bytes: Vec<u8> = utf16.iter().flat_map(|u| u.to_le_bytes()).collect();
    unsafe {
        if !open() {
            return false;
        }
        EmptyClipboard();
        let ok = set_global(TEXT, &bytes);
        if ok {
            for name in PRIVATE_FORMATS {
                let format = RegisterClipboardFormatW(wide(name).as_ptr());
                if format != 0 {
                    set_global(format, &0u32.to_le_bytes());
                }
            }
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
