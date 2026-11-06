//! Turns a snippet into text and puts it where the user is typing.

use std::thread::sleep;
use std::time::Duration;

use crate::store::Snippet;
use crate::{clipboard, input};

/// Format codes for {date:...}, {time:...} and {datetime:...}, longest first so "MMMM" wins over "MM".
const FORMAT_CODES: [(&str, &str); 18] = [
    ("YYYY", "%Y"),
    ("YY", "%y"),
    ("MMMM", "%B"),
    ("MMM", "%b"),
    ("MM", "%m"),
    ("M", "%-m"),
    ("dddd", "%A"),
    ("ddd", "%a"),
    ("DD", "%d"),
    ("D", "%-d"),
    ("HH", "%H"),
    ("H", "%-H"),
    ("hh", "%I"),
    ("h", "%-I"),
    ("mm", "%M"),
    ("ss", "%S"),
    ("A", "%p"),
    ("a", "%P"),
];

/// Turns a user format like "DD/MM/YYYY" into a chrono format string. Text in [brackets] and any
/// other characters are kept as they are.
fn to_strftime(format: &str) -> String {
    let mut out = String::new();
    let mut rest = format;
    'outer: while let Some(ch) = rest.chars().next() {
        if ch == '[' {
            if let Some(end) = rest.find(']') {
                out.push_str(&rest[1..end].replace('%', "%%"));
                rest = &rest[end + 1..];
                continue;
            }
        }
        for (code, spec) in FORMAT_CODES {
            if let Some(after) = rest.strip_prefix(code) {
                out.push_str(spec);
                rest = after;
                continue 'outer;
            }
        }
        if ch == '%' {
            out.push_str("%%");
        } else {
            out.push(ch);
        }
        rest = &rest[ch.len_utf8()..];
    }
    out
}

/// Replaces {name:FORMAT} placeholders, such as {date:DD/MM/YYYY}, with the formatted time.
fn fill_custom_formats(text: &str, name: &str, now: &chrono::DateTime<chrono::Local>) -> String {
    let opening = format!("{{{name}:");
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find(&opening) {
        let format_start = start + opening.len();
        let Some(len) = rest[format_start..].find('}') else { break };
        out.push_str(&rest[..start]);
        out.push_str(&now.format(&to_strftime(&rest[format_start..format_start + len])).to_string());
        rest = &rest[format_start + len + 1..];
    }
    out.push_str(rest);
    out
}

/// Fills in placeholders like {date} and {clipboard}.
pub fn render(content: &str) -> String {
    render_at(content, chrono::Local::now())
}

fn render_at(content: &str, now: chrono::DateTime<chrono::Local>) -> String {
    let mut text = content.to_string();
    for name in ["date", "time", "datetime"] {
        text = fill_custom_formats(&text, name, &now);
    }
    text = text
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

/// Deletes `backspaces` typed characters, inserts the snippet, then counts the use.
pub fn deliver(snippet: &Snippet, backspaces: usize) {
    let text = render(&snippet.content);
    insert(&text, snippet.sensitive, backspaces);
    crate::record_use(&snippet.id, chars_saved(&snippet.content, &snippet.trigger));
}

/// Characters the user didn't type. Text from {clipboard} doesn't count: they'd have pasted it anyway.
fn chars_saved(content: &str, trigger: &str) -> usize {
    let typed_for_you = render(&content.replace("{clipboard}", "")).chars().count();
    typed_for_you.saturating_sub(trigger.chars().count())
}

fn insert(text: &str, sensitive: bool, backspaces: usize) {
    sleep(Duration::from_millis(30));
    if backspaces > 0 {
        input::backspaces(backspaces);
        // Let the app finish deleting before the snippet arrives.
        sleep(Duration::from_millis(50));
    }

    // Typing avoids the clipboard entirely: used for sensitive snippets, and when the clipboard
    // holds an image/files that we couldn't put back afterwards.
    if sensitive || clipboard::holds_non_text() {
        input::type_text(text);
        return;
    }

    let previous = clipboard::get_text();
    if !clipboard::set_text_private(text) {
        input::type_text(text);
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
    use chrono::TimeZone;

    use super::{render, render_at};

    #[test]
    fn fills_every_placeholder() {
        let out = render("d={date} t={time} dt={datetime} day={day} u={uuid}");
        println!("{out}");
        assert!(!out.contains('{'), "unfilled placeholder in: {out}");
    }

    /// Friday 6 November 2026, 17:05:09.
    fn at(content: &str) -> String {
        render_at(content, chrono::Local.with_ymd_and_hms(2026, 11, 6, 17, 5, 9).unwrap())
    }

    #[test]
    fn default_formats_are_unchanged() {
        assert_eq!(at("{date} {time} {datetime} {day}"), "2026-11-06 17:05 2026-11-06 17:05 Friday");
    }

    #[test]
    fn custom_date_formats() {
        assert_eq!(at("{date:DD/MM/YYYY}"), "06/11/2026");
        assert_eq!(at("{date:D MMM YYYY}"), "6 Nov 2026");
        assert_eq!(at("{date:dddd, D MMMM}"), "Friday, 6 November");
        assert_eq!(at("{date:ddd DD-MM-YY}"), "Fri 06-11-26");
        assert_eq!(at("{date:M/D}"), "11/6");
    }

    #[test]
    fn custom_time_formats() {
        assert_eq!(at("{time:hh:mm A}"), "05:05 PM");
        assert_eq!(at("{time:h:mm a}"), "5:05 pm");
        assert_eq!(at("{time:HH:mm:ss}"), "17:05:09");
        assert_eq!(at("{datetime:DD/MM/YYYY hh:mm A}"), "06/11/2026 05:05 PM");
    }

    #[test]
    fn brackets_and_other_text_are_kept() {
        assert_eq!(at("{date:[Today is] dddd}"), "Today is Friday");
        assert_eq!(at("{date:100% YYYY}"), "100% 2026");
        assert_eq!(at("Due {date:DD/MM} and {date:DD/MM}"), "Due 06/11 and 06/11");
    }

    #[test]
    fn clipboard_text_is_not_counted_as_saved() {
        assert_eq!(super::chars_saved("ojaswi3236@gmail.com", "`pemail"), 13);
        assert_eq!(super::chars_saved("{clipboard}", "`clipboard"), 0);
        assert_eq!(super::chars_saved("Quote: {clipboard}", "`q"), 5);
        assert_eq!(super::chars_saved("hi", "`longtrigger"), 0);
    }

    #[test]
    fn unclosed_placeholder_is_left_alone() {
        assert_eq!(at("{date:DD/MM"), "{date:DD/MM");
    }
}
