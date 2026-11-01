//! Synthetic keyboard input via SendInput.

use std::mem::size_of;
use std::thread::sleep;
use std::time::Duration;

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VK_BACK,
    VK_CONTROL, VK_RETURN, VK_TAB,
};

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

pub fn backspaces(count: usize) {
    let inputs: Vec<INPUT> = (0..count).flat_map(|_| tap(VK_BACK)).collect();
    send(&inputs);
}

pub fn ctrl_v() {
    send(&[key(VK_CONTROL, 0, 0), key(VK_V, 0, 0), key(VK_V, 0, KEYEVENTF_KEYUP), key(VK_CONTROL, 0, KEYEVENTF_KEYUP)]);
}

/// Types text character by character. Slower than pasting but never touches the clipboard.
pub fn type_text(text: &str) {
    let mut batch = Vec::new();
    for ch in text.chars() {
        match ch {
            '\r' => continue,
            '\n' => batch.extend(tap(VK_RETURN)),
            '\t' => batch.extend(tap(VK_TAB)),
            _ => {
                let mut units = [0u16; 2];
                for unit in ch.encode_utf16(&mut units) {
                    batch.push(key(0, *unit, KEYEVENTF_UNICODE));
                    batch.push(key(0, *unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
                }
            }
        }
        // Small batches: some apps drop keys when flooded with one huge SendInput call.
        if batch.len() >= 64 {
            send(&batch);
            batch.clear();
            sleep(Duration::from_millis(5));
        }
    }
    send(&batch);
}
