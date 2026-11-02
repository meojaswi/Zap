//! Turns a snippet into text and puts it where the user is typing.

use std::thread::sleep;
use std::time::Duration;

use crate::store::Snippet;
use crate::{clipboard, input};

/// Fills in placeholders like {date} and {clipboard}.
pub fn render(content: &str) -> String {
    let now = chrono::Local::now();
    let mut text = content
        .replace("{date}", &now.format("%Y-%m-%d").to_string())
        .replace("{time}", &now.format("%H:%M").to_string())
        .replace("{datetime}", &now.format("%Y-%m-%d %H:%M").to_string())
        .replace("{day}", &now.format("%A").to_string());
    while text.contains("{uuid}") {
        text = text.replacen("{uuid}", &uuid::Uuid::new_v4().to_string(), 1);
    }
    if text.contains("{clipboard}") {
        text = text.replace("{clipboard}", &clipboard::get_text().unwrap_or_default());
    }
    text
}

/// Deletes `backspaces` typed characters, then inserts the snippet.
pub fn deliver(snippet: &Snippet, backspaces: usize) {
    let text = render(&snippet.content);
    sleep(Duration::from_millis(30));
    if backspaces > 0 {
        input::backspaces(backspaces);
        sleep(Duration::from_millis(20));
    }

    // Typing avoids the clipboard entirely: used for sensitive snippets, and when the clipboard
    // holds an image/files that we couldn't put back afterwards.
    if snippet.sensitive || clipboard::holds_non_text() {
        input::type_text(&text);
        return;
    }

    let previous = clipboard::get_text();
    if !clipboard::set_text_private(&text) {
        input::type_text(&text);
        return;
    }
    sleep(Duration::from_millis(40));
    input::ctrl_v();
    // Give the target app time to read the clipboard before we restore it.
    sleep(Duration::from_millis(300));
    match previous {
        Some(previous) => {
            clipboard::set_text_private(&previous);
        }
        None => clipboard::clear(),
    }
}

#[cfg(test)]
mod tests {
    use super::render;

    #[test]
    fn fills_every_placeholder() {
        let out = render("d={date} t={time} dt={datetime} day={day} u={uuid}");
        println!("{out}");
        assert!(!out.contains('{'), "unfilled placeholder in: {out}");
    }
}
