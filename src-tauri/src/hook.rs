//! Optional inline mode: a low-level keyboard hook that expands triggers as you type them.
//!
//! Privacy: only the last few characters (up to the longest trigger) are ever kept, in memory,
//! and nothing is logged or written to disk. A mouse hook on the same thread only watches for
//! clicks, which reset that buffer.

use std::mem::zeroed;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicIsize, AtomicU64, Ordering};
use std::sync::{mpsc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, GetKeyboardLayout, ToUnicodeEx, VK_BACK, VK_CAPITAL, VK_CONTROL, VK_DELETE,
    VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_LEFT, VK_LWIN, VK_MENU, VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT,
    VK_RWIN, VK_SHIFT, VK_TAB, VK_UP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN,
    WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_NCLBUTTONDOWN, WM_NCMBUTTONDOWN, WM_NCRBUTTONDOWN, WM_NCXBUTTONDOWN, WM_QUIT,
    WM_RBUTTONDOWN, WM_SYSKEYDOWN, WM_XBUTTONDOWN,
};

use crate::store::{self, normalize_app};
use crate::{expand, winutil};

static HOOK_THREAD: Mutex<Option<u32>> = Mutex::new(None);
static BUFFER: Mutex<String> = Mutex::new(String::new());
/// Window the buffered keys were typed into; typing into a different window starts over.
static BUFFER_WINDOW: AtomicIsize = AtomicIsize::new(0);

/// The expansion Backspace would undo. Only the trigger is kept, never the text around it.
struct Undo {
    window: isize,
    snippet_id: String,
    trigger: String,
    chars: usize,
    saved: usize,
    at: Instant,
}

static UNDO: Mutex<Option<Undo>> = Mutex::new(None);
/// Bumped by every real key or click, so an expansion can tell the user did something while it ran.
static ACTIVITY: AtomicU64 = AtomicU64::new(0);
/// Backspace much later is an ordinary correction, not second thoughts about the expansion.
const UNDO_WINDOW: Duration = Duration::from_secs(5);

/// Only the very next key can undo, so any key or click ends the chance.
fn take_undo() -> Option<Undo> {
    let mut undo = UNDO.lock().unwrap();
    ACTIVITY.fetch_add(1, Ordering::Relaxed);
    undo.take()
}

fn offer_undo(activity: u64, undo: Undo) {
    let mut slot = UNDO.lock().unwrap();
    if ACTIVITY.load(Ordering::Relaxed) == activity {
        *slot = Some(undo);
    }
}

pub fn set_enabled(enabled: bool) {
    let mut thread_id = HOOK_THREAD.lock().unwrap();
    match (enabled, *thread_id) {
        (true, None) => *thread_id = start(),
        (false, Some(id)) => {
            unsafe {
                PostThreadMessageW(id, WM_QUIT, 0, 0);
            }
            *thread_id = None;
            clear_buffer();
            take_undo();
        }
        _ => {}
    }
}

pub fn clear_buffer() {
    BUFFER.lock().unwrap().clear();
}

fn start() -> Option<u32> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || unsafe {
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), GetModuleHandleW(null()), 0);
        if hook.is_null() {
            let _ = tx.send(None);
            return;
        }
        // A click can move the caret or focus without any key press, so it resets the buffer too.
        // If this hook can't be installed, the foreground-window check in handle_key still applies.
        let mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), GetModuleHandleW(null()), 0);
        let _ = tx.send(Some(GetCurrentThreadId()));
        // Low-level hooks are delivered through this thread's message loop.
        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        UnhookWindowsHookEx(hook);
        if !mouse_hook.is_null() {
            UnhookWindowsHookEx(mouse_hook);
        }
    });
    rx.recv().ok().flatten()
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && (wparam as u32 == WM_KEYDOWN || wparam as u32 == WM_SYSKEYDOWN) {
        let event = &*(lparam as *const KBDLLHOOKSTRUCT);
        // Ignore our own SendInput keystrokes.
        if event.flags & LLKHF_INJECTED == 0 && handle_key(event.vkCode, event.scanCode) {
            return 1; // swallow the last key of the trigger
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        match wparam as u32 {
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN | WM_NCLBUTTONDOWN
            | WM_NCRBUTTONDOWN | WM_NCMBUTTONDOWN | WM_NCXBUTTONDOWN => {
                clear_buffer();
                take_undo();
            }
            _ => {}
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

fn is_down(vk: u16) -> bool {
    unsafe { GetAsyncKeyState(vk as i32) < 0 }
}

/// Returns true when the key completed a trigger and should be swallowed.
fn handle_key(vk: u32, scan: u32) -> bool {
    let undo = take_undo();
    let shared = store::shared();
    if shared.paused.load(Ordering::Relaxed) {
        return false;
    }
    let mut buffer = BUFFER.lock().unwrap();
    let window = winutil::foreground_window();
    if BUFFER_WINDOW.swap(window, Ordering::Relaxed) != window {
        buffer.clear();
    }
    if is_down(VK_CONTROL) || is_down(VK_MENU) || is_down(VK_LWIN) || is_down(VK_RWIN) {
        buffer.clear();
        return false;
    }
    match vk as u16 {
        VK_BACK => {
            if let Some(undo) = undo.filter(|u| u.window == window && u.at.elapsed() < UNDO_WINDOW) {
                buffer.clear();
                thread::spawn(move || {
                    expand::undo(undo.chars, &undo.trigger);
                    crate::record_undo(&undo.snippet_id, undo.saved);
                });
                return true;
            }
            buffer.pop();
            return false;
        }
        VK_RETURN | VK_TAB | VK_ESCAPE | VK_LEFT | VK_RIGHT | VK_UP | VK_DOWN | VK_HOME | VK_END | VK_PRIOR
        | VK_NEXT | VK_DELETE => {
            buffer.clear();
            return false;
        }
        _ => {}
    }
    let Some(ch) = vk_to_char(vk, scan) else { return false };
    buffer.push(ch);

    let data = shared.data.read().unwrap();
    let prefix = data.settings.secondary_prefix.as_str();
    let forms: Vec<(usize, String)> = data
        .snippets
        .iter()
        .enumerate()
        .flat_map(|(i, s)| store::trigger_forms(&s.trigger, prefix).into_iter().map(move |f| (i, f)))
        .collect();
    let longest = forms.iter().map(|(_, f)| f.chars().count()).max().unwrap_or(0);
    let len = buffer.chars().count();
    if len > longest {
        *buffer = buffer.chars().skip(len - longest).collect();
    }

    let Some((index, typed_trigger)) = forms.into_iter().find(|(_, f)| buffer.ends_with(f.as_str())) else {
        return false;
    };
    let snippet = &data.snippets[index];
    buffer.clear();
    if let Some(exe) = winutil::foreground_exe() {
        if data.settings.excluded_apps.iter().any(|app| normalize_app(app) == exe) {
            return false;
        }
    }
    let snippet = expand::with_nested(snippet, &data.snippets);
    // The final key is swallowed, so only the characters already typed need deleting.
    let typed = typed_trigger.chars().count() - 1;
    if expand::input_labels(&snippet.content).is_empty() {
        let activity = ACTIVITY.load(Ordering::Relaxed);
        thread::spawn(move || {
            let delivered = expand::deliver(&snippet, typed);
            if let Some(chars) = delivered.undo_chars {
                offer_undo(activity, Undo { window, snippet_id: snippet.id, trigger: typed_trigger, chars, saved: delivered.saved, at: Instant::now() });
            }
        });
    } else {
        // Ask for the fill-ins first. If the user cancels, the swallowed key is typed back.
        let target = winutil::foreground_window();
        let swallowed = typed_trigger.chars().last();
        thread::spawn(move || crate::request_fill(snippet, typed, target, swallowed));
    }
    true
}

fn vk_to_char(vk: u32, scan: u32) -> Option<char> {
    let mut state = [0u8; 256];
    if is_down(VK_SHIFT) {
        state[VK_SHIFT as usize] = 0x80;
    }
    unsafe {
        if GetKeyState(VK_CAPITAL as i32) & 1 != 0 {
            state[VK_CAPITAL as usize] = 0x01;
        }
        let layout = GetKeyboardLayout(winutil::foreground_thread());
        let mut out = [0u16; 4];
        // Flag 0x4: don't change the keyboard's dead-key state (Windows 10 1607+).
        let n = ToUnicodeEx(vk, scan, state.as_ptr(), out.as_mut_ptr(), out.len() as i32, 0x4, layout);
        if n == 1 {
            char::from_u32(out[0] as u32).filter(|c| !c.is_control())
        } else {
            None
        }
    }
}
