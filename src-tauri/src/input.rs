//! Synthetic keyboard input via SendInput.

use std::mem::size_of;
use std::thread::sleep;
use std::time::Duration;

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, GetKeyboardLayout, MapVirtualKeyExW, SendInput, VkKeyScanExW, HKL, INPUT, INPUT_0, INPUT_KEYBOARD,
    KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MAPVK_VK_TO_CHAR, MAPVK_VK_TO_VSC, VK_BACK, VK_CAPITAL,
    VK_CONTROL, VK_RETURN, VK_SHIFT, VK_TAB,
};

use crate::winutil;

const VK_V: u16 = 0x56;

fn key(vk: u16, scan: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
    }
}

fn send(inputs: &[INPUT]) {
    if inputs.is_empty() {
        return;
    }
    unsafe {
        SendInput(inputs.len() as u32, inputs.as_ptr(), size_of::<INPUT>() as i32);
    }
}

fn tap(vk: u16) -> [INPUT; 2] {
    [key(vk, 0, 0), key(vk, 0, KEYEVENTF_KEYUP)]
}

/// One at a time: apps that fall behind on a burst can apply a late key to the wrong spot.
pub fn backspaces(count: usize) {
    for _ in 0..count {
        send(&tap(VK_BACK));
        sleep(CHAR_DELAY);
    }
}

pub fn ctrl_v() {
    send(&[key(VK_CONTROL, 0, 0), key(VK_V, 0, 0), key(VK_V, 0, KEYEVENTF_KEYUP), key(VK_CONTROL, 0, KEYEVENTF_KEYUP)]);
}

/// Pause after each synthetic key. Apps like Windows 11 Notepad work out a key's character from the
/// keyboard state when they get round to it, so keys sent faster than they keep up come out wrong.
const CHAR_DELAY: Duration = Duration::from_millis(20);

/// Types text character by character. Slower than pasting but never touches the clipboard.
pub fn type_text(text: &str) {
    type_text_paced(text, CHAR_DELAY);
}

fn type_text_paced(text: &str, delay: Duration) {
    let layout = unsafe { GetKeyboardLayout(winutil::foreground_thread()) };
    for ch in text.chars() {
        let inputs = match ch {
            '\r' => continue,
            '\n' => tap(VK_RETURN).to_vec(),
            '\t' => tap(VK_TAB).to_vec(),
            _ => layout_keys(ch, layout).unwrap_or_else(|| unicode_keys(ch)),
        };
        send(&inputs);
        sleep(delay);
    }
}

/// The real key presses that type `ch` on this keyboard layout, with Shift if needed.
///
/// Preferred over Unicode packets: apps like Windows 11 Notepad read a packet's character only when
/// they get round to it, so when they fall behind, earlier characters come out as later ones.
fn layout_keys(ch: char, layout: HKL) -> Option<Vec<INPUT>> {
    let unit = u16::try_from(u32::from(ch)).ok()?;
    let scan = unsafe { VkKeyScanExW(unit, layout) };
    if scan == -1 {
        return None;
    }
    let vk = (scan & 0xff) as u16;
    let modifiers = (scan >> 8) & 0xff;
    // Ctrl or Alt (AltGr) combos could fire shortcuts, and dead keys would combine with the next key.
    let dead = unsafe { MapVirtualKeyExW(vk as u32, MAPVK_VK_TO_CHAR, layout) } & 0x8000_0000 != 0;
    if modifiers & !1 != 0 || dead {
        return None;
    }
    let mut shift = modifiers & 1 != 0;
    // VkKeyScanEx assumes Caps Lock is off, which flips Shift for letters.
    if ch.is_alphabetic() && unsafe { GetKeyState(VK_CAPITAL as i32) } & 1 != 0 {
        shift = !shift;
    }
    let vsc = unsafe { MapVirtualKeyExW(vk as u32, MAPVK_VK_TO_VSC, layout) } as u16;
    let mut inputs = Vec::with_capacity(4);
    if shift {
        inputs.push(key(VK_SHIFT, 0, 0));
    }
    inputs.push(key(vk, vsc, 0));
    inputs.push(key(vk, vsc, KEYEVENTF_KEYUP));
    if shift {
        inputs.push(key(VK_SHIFT, 0, KEYEVENTF_KEYUP));
    }
    Some(inputs)
}

/// Characters the layout can't type directly, sent as Unicode packets.
fn unicode_keys(ch: char) -> Vec<INPUT> {
    let mut units = [0u16; 2];
    ch.encode_utf16(&mut units)
        .iter()
        .flat_map(|unit| [key(0, *unit, KEYEVENTF_UNICODE), key(0, *unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP)])
        .collect()
}
