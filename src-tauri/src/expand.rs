//! Turns a snippet into text and puts it where the user is typing.

use std::collections::HashMap;
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

const MAX_FILL_INS: usize = 10;

/// The labels of the snippet's {input:Label} fill-ins, in order, each once.
pub fn input_labels(content: &str) -> Vec<String> {
    let mut labels = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("{input:") {
        let after = &rest[start + "{input:".len()..];
        let Some(end) = after.find('}') else { break };
        let label = after[..end].trim();
        if !label.is_empty() && !labels.iter().any(|l| l == label) && labels.len() < MAX_FILL_INS {
            labels.push(label.to_string());
        }
        rest = &after[end + 1..];
    }
    labels
}

/// Replaces each {input:Label} with the value given for it, or nothing if there isn't one.
pub fn fill_inputs(text: &str, values: &HashMap<String, String>) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("{input:") {
        let after = &rest[start + "{input:".len()..];
        let Some(end) = after.find('}') else { break };
        out.push_str(&rest[..start]);
        out.push_str(values.get(after[..end].trim()).map_or("", String::as_str));
        rest = &after[end + 1..];
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
    deliver_filled(snippet, backspaces, &HashMap::new());
}

/// Like `deliver`, with the values the user entered for the snippet's {input:...} fill-ins.
/// Placeholders are filled first, so a typed value like "{date}" is inserted as written.
pub fn deliver_filled(snippet: &Snippet, backspaces: usize, values: &HashMap<String, String>) {
    let text = fill_inputs(&render(&snippet.content), values);
    insert(&text, snippet.sensitive, backspaces);
    // Inline, the trigger typed was `backspaces` + 1 characters (it may be the secondary one).
    let typed = if backspaces > 0 { backspaces + 1 } else { snippet.trigger.chars().count() };
    crate::record_use(&snippet.id, chars_saved(&snippet.content, typed));
}

/// Characters the user didn't type. Text from {clipboard} doesn't count (they'd have pasted it
/// anyway), and neither do fill-in values, which the user typed themselves.
fn chars_saved(content: &str, trigger_len: usize) -> usize {
    let typed_for_you = fill_inputs(&render(&content.replace("{clipboard}", "")), &HashMap::new()).chars().count();
    typed_for_you.saturating_sub(trigger_len)
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
        assert_eq!(super::chars_saved("someone@example.com", 6), 13);
        assert_eq!(super::chars_saved("{clipboard}", 10), 0);
        assert_eq!(super::chars_saved("Quote: {clipboard}", 2), 5);
        assert_eq!(super::chars_saved("hi", 12), 0);
    }

    #[test]
    fn input_labels_are_found_once_each_in_order() {
        use super::input_labels;
        assert_eq!(input_labels("Dear {input:Name}, order {input: Order no } by {input:Date}. Bye {input:Name}"),
                   ["Name", "Order no", "Date"]);
        assert_eq!(input_labels("no fill-ins {date}"), Vec::<String>::new());
        assert_eq!(input_labels("{input:} {input:   } {input:Open"), Vec::<String>::new());
    }

    #[test]
    fn fill_inputs_puts_each_value_everywhere_its_label_appears() {
        use std::collections::HashMap;
        let values = HashMap::from([("Name".to_string(), "Rahul".to_string()), ("Order no".to_string(), "A12".to_string())]);
        assert_eq!(super::fill_inputs("Hi {input:Name}, #{input: Order no }. Bye {input:Name}", &values), "Hi Rahul, #A12. Bye Rahul");
        assert_eq!(super::fill_inputs("Missing {input:Date}.", &values), "Missing .");
    }

    #[test]
    fn typed_values_are_not_treated_as_placeholders() {
        use std::collections::HashMap;
        let values = HashMap::from([("Note".to_string(), "{date}".to_string())]);
        let text = super::fill_inputs(&at("On {date}: {input:Note}"), &values);
        assert_eq!(text, "On 2026-11-06: {date}");
    }

    #[test]
    fn fill_in_values_are_not_counted_as_saved() {
        assert_eq!(super::chars_saved("Hi {input:Name}!", 3), 1); // "Hi !" is 4, minus a 3-character trigger
        assert_eq!(super::chars_saved("Hello there {input:Name}", 4), 8);
    }

    #[test]
    fn unclosed_placeholder_is_left_alone() {
        assert_eq!(at("{date:DD/MM"), "{date:DD/MM");
    }
}
