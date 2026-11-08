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

/// Works out a date relative to now from what follows the name, like "+7", "-2w:DD/MM" or "+1m".
/// A plain number is days; w, m and y are weeks, months and years. Anything else returns None.
fn date_maths(spec: &str, name: &str, now: &chrono::DateTime<chrono::Local>) -> Option<String> {
    use chrono::{Days, Months};
    let (offset, format) = spec.split_once(':').map_or((spec, None), |(o, f)| (o, Some(f)));
    let forward = offset.starts_with('+');
    let rest = offset.get(1..)?;
    let digits = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    let amount: u32 = rest[..digits].parse().ok()?;
    let by_days = |days: u64| if forward { now.checked_add_days(Days::new(days)) } else { now.checked_sub_days(Days::new(days)) };
    let by_months = |months: u32| if forward { now.checked_add_months(Months::new(months)) } else { now.checked_sub_months(Months::new(months)) };
    let shifted = match &rest[digits..] {
        "" | "d" => by_days(u64::from(amount)),
        "w" => by_days(u64::from(amount) * 7),
        "m" => by_months(amount),
        "y" => by_months(amount.checked_mul(12)?),
        _ => None,
    }?;
    let strftime = match (format, name) {
        (Some(format), _) => to_strftime(format),
        (None, "datetime") => "%Y-%m-%d %H:%M".to_string(),
        (None, "day") => "%A".to_string(),
        (None, _) => "%Y-%m-%d".to_string(),
    };
    Some(shifted.format(&strftime).to_string())
}

/// Replaces {name+N} and {name-N} placeholders. Ones that can't be read are left as written.
fn fill_date_maths(text: &str, name: &str, now: &chrono::DateTime<chrono::Local>) -> String {
    let opening = format!("{{{name}");
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find(&opening) {
        let name_end = start + opening.len();
        let after = &rest[name_end..];
        if !after.starts_with(['+', '-']) {
            out.push_str(&rest[..name_end]);
            rest = after;
            continue;
        }
        let Some(end) = after.find('}') else { break };
        match date_maths(&after[..end], name, now) {
            Some(value) => {
                out.push_str(&rest[..start]);
                out.push_str(&value);
            }
            None => out.push_str(&rest[..name_end + end + 1]),
        }
        rest = &after[end + 1..];
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

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

fn unescape_html(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// Like `fill_inputs`, for a formatted snippet: values are escaped so typed text can't add HTML,
/// and labels are matched as written (the HTML may store "A&B" as "A&amp;B").
pub fn fill_inputs_html(html: &str, values: &HashMap<String, String>) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(start) = rest.find("{input:") {
        let after = &rest[start + "{input:".len()..];
        let Some(end) = after.find('}') else { break };
        out.push_str(&rest[..start]);
        let label = unescape_html(after[..end].trim());
        out.push_str(&escape_html(values.get(label.trim()).map_or("", String::as_str)));
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

const MAX_NESTING: usize = 8;
/// Stops a few snippets that each include the others many times from growing without end.
const MAX_NESTED_BYTES: usize = 100_000;

/// Finds the snippet a {snippet:...} names: by name first, then by trigger, with or without its symbol.
fn find_nested<'a>(reference: &str, all: &'a [Snippet]) -> Option<&'a Snippet> {
    let bare = |s: &'a str| s.trim_start_matches(|c: char| !c.is_alphanumeric());
    all.iter()
        .find(|s| s.name.eq_ignore_ascii_case(reference))
        .or_else(|| all.iter().find(|s| s.trigger.eq_ignore_ascii_case(reference)))
        .or_else(|| all.iter().find(|s| bare(&s.trigger).eq_ignore_ascii_case(reference)))
}

struct Nesting<'a> {
    all: &'a [Snippet],
    /// Show a Sensitive snippet as [Name] instead of its text (for previews).
    mask_sensitive: bool,
    used_sensitive: bool,
    bytes: usize,
}

impl Nesting<'_> {
    /// Replaces each {snippet:...}. Unknown names and loops are left as written, so they're visible.
    fn expand(&mut self, text: &str, html: bool, stack: &mut Vec<String>) -> String {
        let mut out = String::new();
        let mut rest = text;
        while let Some(start) = rest.find("{snippet:") {
            let after = &rest[start + "{snippet:".len()..];
            let Some(end) = after.find('}') else { break };
            out.push_str(&rest[..start]);
            let placeholder = &rest[start..start + "{snippet:".len() + end + 1];
            let reference = if html { unescape_html(&after[..end]) } else { after[..end].to_string() };
            rest = &after[end + 1..];
            let found = find_nested(reference.trim(), self.all)
                .filter(|s| !stack.contains(&s.id) && stack.len() < MAX_NESTING && self.bytes < MAX_NESTED_BYTES);
            let Some(nested) = found else {
                out.push_str(placeholder);
                continue;
            };
            if nested.sensitive {
                self.used_sensitive = true;
                if self.mask_sensitive {
                    out.push_str(&format!("[{}]", nested.name));
                    continue;
                }
            }
            self.bytes += nested.content.len();
            let inner = match (html, nested.html.as_deref().filter(|_| !nested.sensitive)) {
                (false, _) => nested.content.clone(),
                (true, Some(formatted)) => formatted.to_string(),
                (true, None) => escape_html(&nested.content).replace('\n', "<br>"),
            };
            stack.push(nested.id.clone());
            out.push_str(&self.expand(&inner, html, stack));
            stack.pop();
        }
        out.push_str(rest);
        out
    }
}

/// The snippet with its {snippet:...} parts filled in, ready for its fill-ins and placeholders.
/// Including a Sensitive snippet makes the whole result Sensitive, so it never touches the clipboard.
pub fn with_nested(snippet: &Snippet, all: &[Snippet]) -> Snippet {
    let mut nesting = Nesting { all, mask_sensitive: false, used_sensitive: false, bytes: 0 };
    let mut result = snippet.clone();
    result.content = nesting.expand(&snippet.content, false, &mut vec![snippet.id.clone()]);
    nesting.bytes = 0;
    result.html = snippet.html.as_deref().map(|html| nesting.expand(html, true, &mut vec![snippet.id.clone()]));
    result.sensitive |= nesting.used_sensitive;
    result
}

/// Text for the editor's preview: nested snippets filled in, Sensitive ones shown only by name.
pub fn nested_for_preview(content: &str, all: &[Snippet]) -> String {
    Nesting { all, mask_sensitive: true, used_sensitive: false, bytes: 0 }.expand(content, false, &mut Vec::new())
}

/// Fills in placeholders like {date} and {clipboard}.
pub fn render(content: &str) -> String {
    render_at(content, chrono::Local::now(), false)
}

/// `render` for a formatted snippet's HTML: clipboard text is escaped so it stays text.
pub fn render_html(html: &str) -> String {
    render_at(html, chrono::Local::now(), true)
}

fn render_at(content: &str, now: chrono::DateTime<chrono::Local>, html: bool) -> String {
    let mut text = content.to_string();
    for name in ["date", "datetime", "day"] {
        text = fill_date_maths(&text, name, &now);
    }
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
        let copied = clipboard::get_text().unwrap_or_default();
        text = text.replace("{clipboard}", &if html { escape_html(&copied) } else { copied });
    }
    text
}

/// What was inserted, for undoing it.
pub struct Delivered {
    /// Backspaces that remove the text again, when that can be trusted.
    pub undo_chars: Option<usize>,
    pub saved: usize,
}

const MAX_UNDO_CHARS: usize = 500;

/// How many backspaces remove the inserted text, or None if that can't be known. Too many would
/// eat the user's own text, so block-level formatting and emoji (one or several presses, depending
/// on the app) aren't undoable, and very long text would take too long to remove.
fn undo_length(text: &str, html: Option<&str>) -> Option<usize> {
    if html.is_some_and(|h| ["<ul", "<ol", "<li", "<p", "<div"].iter().any(|tag| h.contains(tag))) {
        return None;
    }
    if text.chars().any(|c| c as u32 > 0xFFFF || c == '\u{200D}' || c == '\u{FE0F}') {
        return None;
    }
    let count = text.chars().filter(|c| *c != '\r').count();
    (count > 0 && count <= MAX_UNDO_CHARS).then_some(count)
}

/// Removes an expansion and types its trigger back. The hook ignores these keys, so it won't re-fire.
pub fn undo(chars: usize, trigger: &str) {
    sleep(Duration::from_millis(30));
    input::backspaces(chars);
    sleep(Duration::from_millis(50));
    input::type_text(trigger);
}

/// Copies what's selected in the focused app and puts the user's clipboard back.
pub fn copy_selection() -> Result<String, String> {
    const NOTHING: &str = "Nothing was selected. Select some text, then press the hotkey again.";
    if !input::wait_for_modifiers_up(Duration::from_secs(2)) {
        return Err("Let go of the hotkey quickly so Zap can copy the selection.".to_string());
    }
    // An image or files couldn't be put back afterwards, so they're left alone.
    if clipboard::holds_non_text() {
        return Err("Zap couldn't copy the selection because an image or files are on the clipboard. Paste the text here instead.".to_string());
    }
    let previous = clipboard::get_text();
    let before = clipboard::sequence();
    input::ctrl_c();
    let mut waited = 0;
    while clipboard::sequence() == before {
        if waited >= 500 {
            return Err(NOTHING.to_string());
        }
        sleep(Duration::from_millis(10));
        waited += 10;
    }
    // Let the app finish writing to the clipboard.
    sleep(Duration::from_millis(40));
    let copied = clipboard::get_text();
    match previous {
        Some(previous) => {
            clipboard::set_text_private(&previous);
        }
        None => clipboard::clear(),
    }
    let text = copied.unwrap_or_default().replace("\r\n", "\n");
    let text = text.trim_end_matches('\n');
    if text.is_empty() {
        return Err(NOTHING.to_string());
    }
    Ok(text.to_string())
}

/// Deletes `backspaces` typed characters, inserts the snippet, then counts the use.
pub fn deliver(snippet: &Snippet, backspaces: usize) -> Delivered {
    deliver_filled(snippet, backspaces, &HashMap::new())
}

/// Like `deliver`, with the values the user entered for the snippet's {input:...} fill-ins.
/// Placeholders are filled first, so a typed value like "{date}" is inserted as written.
pub fn deliver_filled(snippet: &Snippet, backspaces: usize, values: &HashMap<String, String>) -> Delivered {
    let text = fill_inputs(&render(&snippet.content), values);
    // Formatted snippets are pasted; Sensitive ones are always typed as plain text.
    let formatted = snippet.html.as_deref().filter(|_| !snippet.sensitive);
    match formatted {
        Some(html) => insert_formatted(&fill_inputs_html(&render_html(html), values), &text, backspaces),
        None => insert(&text, snippet.sensitive, backspaces),
    }
    // Inline, the trigger typed was `backspaces` + 1 characters (it may be the secondary one).
    let typed = if backspaces > 0 { backspaces + 1 } else { snippet.trigger.chars().count() };
    let saved = chars_saved(&snippet.content, typed);
    crate::record_use(&snippet.id, saved);
    Delivered { undo_chars: undo_length(&text, formatted), saved }
}

/// Characters the user didn't type. Text from {clipboard} doesn't count (they'd have pasted it
/// anyway), and neither do fill-in values, which the user typed themselves.
fn chars_saved(content: &str, trigger_len: usize) -> usize {
    let typed_for_you = fill_inputs(&render(&content.replace("{clipboard}", "")), &HashMap::new()).chars().count();
    typed_for_you.saturating_sub(trigger_len)
}

fn delete_trigger(backspaces: usize) {
    sleep(Duration::from_millis(30));
    if backspaces > 0 {
        input::backspaces(backspaces);
        // Let the app finish deleting before the snippet arrives.
        sleep(Duration::from_millis(50));
    }
}

/// Pastes what's on the clipboard, then puts the user's previous clipboard text back.
fn paste_then_restore(previous: Option<String>) {
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

/// Pastes formatted text; apps that don't take HTML get `plain`.
fn insert_formatted(html: &str, plain: &str, backspaces: usize) {
    delete_trigger(backspaces);
    // An image or files on the clipboard couldn't be put back afterwards, so type the plain text.
    if clipboard::holds_non_text() {
        input::type_text(plain);
        return;
    }
    let previous = clipboard::get_text();
    if !clipboard::set_html_private(html, plain) {
        input::type_text(plain);
        return;
    }
    paste_then_restore(previous);
}

fn insert(text: &str, sensitive: bool, backspaces: usize) {
    delete_trigger(backspaces);

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
    paste_then_restore(previous);
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
        render_at(content, chrono::Local.with_ymd_and_hms(2026, 11, 6, 17, 5, 9).unwrap(), false)
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
    fn formatted_fill_ins_escape_values_and_match_escaped_labels() {
        use std::collections::HashMap;
        let values = HashMap::from([("A&B".to_string(), "<script>x</script> & \"hi\"".to_string())]);
        assert_eq!(
            super::fill_inputs_html("<b>{input:A&amp;B}</b>", &values),
            "<b>&lt;script&gt;x&lt;/script&gt; &amp; &quot;hi&quot;</b>"
        );
    }

    fn snip(id: &str, name: &str, trigger: &str, content: &str) -> crate::store::Snippet {
        serde_json::from_value(serde_json::json!({ "id": id, "name": name, "trigger": trigger, "content": content })).unwrap()
    }

    #[test]
    fn nested_snippets_fill_in_by_name_or_trigger() {
        let all = [snip("1", "Signature", "`sig", "Regards, {snippet:Team}"), snip("2", "Team", "`team", "Zap {date:YYYY}")];
        let outer = snip("3", "Mail", "`m", "Hi\n{snippet:signature} / {snippet:`team} / {snippet:team}");
        assert_eq!(super::with_nested(&outer, &all).content, "Hi\nRegards, Zap {date:YYYY} / Zap {date:YYYY} / Zap {date:YYYY}");
    }

    #[test]
    fn nested_fill_ins_are_asked_for() {
        let all = [snip("1", "Greeting", "`g", "Hi {input:Name}")];
        let outer = snip("2", "Mail", "`m", "{snippet:Greeting}, {input:Topic}");
        assert_eq!(super::input_labels(&super::with_nested(&outer, &all).content), ["Name", "Topic"]);
    }

    #[test]
    fn unknown_names_and_loops_are_left_as_written() {
        let a = snip("1", "A", "`a", "a{snippet:B}");
        let b = snip("2", "B", "`b", "b{snippet:A}");
        let all = [a.clone(), b];
        assert_eq!(super::with_nested(&a, &all).content, "ab{snippet:A}");
        let missing = snip("3", "C", "`c", "x{snippet:Nope}y");
        assert_eq!(super::with_nested(&missing, &all).content, "x{snippet:Nope}y");
    }

    #[test]
    fn repeated_nesting_stays_bounded() {
        let body = |next: &str| format!("{{snippet:{next}}}").repeat(20);
        let all: Vec<_> = ["A", "B", "C", "D", "E", "F", "G"].windows(2).map(|w| snip(w[0], w[0], "", &body(w[1]))).collect();
        let result = super::with_nested(&all[0], &all);
        assert!(result.content.len() < 2_000_000, "{}", result.content.len());
    }

    #[test]
    fn a_sensitive_part_makes_the_whole_snippet_sensitive() {
        let mut secret = snip("1", "Pin", "`pin", "1234");
        secret.sensitive = true;
        let all = [secret];
        let mut outer = snip("2", "Mail", "`m", "Code {snippet:Pin}");
        outer.html = Some("<b>Code</b> {snippet:Pin}".to_string());
        let result = super::with_nested(&outer, &all);
        assert!(result.sensitive);
        assert_eq!(result.content, "Code 1234");
        assert_eq!(super::nested_for_preview(&outer.content, &all), "Code [Pin]");
    }

    #[test]
    fn nested_text_is_escaped_inside_formatted_snippets() {
        let all = [snip("1", "A&B", "`ab", "x < y\nz")];
        let mut outer = snip("2", "Mail", "`m", "{snippet:A&B}");
        outer.html = Some("<b>{snippet:A&amp;B}</b>".to_string());
        assert_eq!(super::with_nested(&outer, &all).html.unwrap(), "<b>x &lt; y<br>z</b>");
    }

    #[test]
    fn undo_length_only_trusts_what_backspace_removes_exactly() {
        use super::undo_length;
        assert_eq!(undo_length("hello", None), Some(5));
        assert_eq!(undo_length("a\nb", None), Some(3));
        assert_eq!(undo_length("a\r\nb", None), Some(3));
        assert_eq!(undo_length("नमस्ते", None), Some(6));
        assert_eq!(undo_length("<b>", Some("&lt;b&gt;")), Some(3));
        assert_eq!(undo_length("hi", Some("<b>hi</b>")), Some(2));
        assert_eq!(undo_length("one\ntwo", Some("<ul><li>one</li><li>two</li></ul>")), None);
        assert_eq!(undo_length("ok 😀", None), None);
        assert_eq!(undo_length("", None), None);
        assert_eq!(undo_length(&"x".repeat(501), None), None);
    }

    #[test]
    fn date_maths_adds_and_subtracts() {
        assert_eq!(at("{date+7}"), "2026-11-13");
        assert_eq!(at("{date-6:DD/MM/YYYY}"), "31/10/2026");
        assert_eq!(at("{date+2w}"), "2026-11-20");
        assert_eq!(at("{date+1m:D MMM}"), "6 Dec");
        assert_eq!(at("{date-1y:YYYY}"), "2025");
        assert_eq!(at("{day+1}"), "Saturday");
        assert_eq!(at("{datetime+1:DD/MM hh:mm A}"), "07/11 05:05 PM");
        assert_eq!(at("{datetime-1}"), "2026-11-05 17:05");
        assert_eq!(at("Due {date+1} after {date}"), "Due 2026-11-07 after 2026-11-06");
    }

    #[test]
    fn date_maths_uses_the_last_day_of_a_shorter_month() {
        let jan_31 = chrono::Local.with_ymd_and_hms(2026, 1, 31, 12, 0, 0).unwrap();
        assert_eq!(render_at("{date+1m}", jan_31, false), "2026-02-28");
    }

    #[test]
    fn unreadable_date_maths_is_left_as_written() {
        for text in ["{date+}", "{date+x}", "{date+1q}", "{date-+1}", "{date+99999999999}", "{date+4000000000y}", "{date+1"] {
            assert_eq!(at(text), text);
        }
    }

    #[test]
    fn unclosed_placeholder_is_left_alone() {
        assert_eq!(at("{date:DD/MM"), "{date:DD/MM");
    }
}
