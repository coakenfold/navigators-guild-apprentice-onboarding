//! Keeping text from changing what the terminal shows.
//!
//! Titles and names are checked when they come in (`check_printable`), but a
//! file edited by hand, or a folder's name, can still hold an escape that
//! clears a line or a mark that reorders text. So everything printed goes
//! through `for_terminal` or `for_json` as well.

use crate::error::{Error, Result};

/// Whether `c` would change what the terminal shows rather than appear as
/// itself: a control character (including tabs, line breaks and the C1
/// controls), a Unicode line or paragraph separator, or a mark that
/// reorders text.
pub fn changes_display(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{2028}'
                | '\u{2029}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
}

/// Refuses text that's printed on one line, such as a title, if it holds a
/// line break or a character that changes what the terminal shows.
/// Otherwise an agent's title could rewrite the lines I read before
/// deciding on its work. `what` names the text, and `fix` says what to do
/// instead.
pub fn check_printable(text: &str, what: &str, fix: &str) -> Result<()> {
    for c in text.chars() {
        let line_break = matches!(
            c,
            '\n' | '\r' | '\u{0b}' | '\u{0c}' | '\u{85}' | '\u{2028}' | '\u{2029}'
        );
        if line_break {
            return Err(Error::input(format!("{what} must fit on one line. {fix}")));
        }
        if changes_display(c) {
            return Err(Error::input(format!(
                "{what} can't contain the control character U+{:04X}, which would change \
                 what the terminal shows. {fix}",
                c as u32
            )));
        }
    }
    Ok(())
}

/// `text` with each character that would change the display written out
/// instead, as `\u{1b}`. With `keep_newlines`, `\n` is kept, for output
/// that's several lines long.
pub fn for_terminal(text: &str, keep_newlines: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if changes_display(c) && !(keep_newlines && c == '\n') {
            out.push_str(&format!("\\u{{{:x}}}", c as u32));
        } else {
            out.push(c);
        }
    }
    out
}

/// JSON text with each character that would change the display written as
/// a JSON escape, such as `\u202e`. serde_json already escapes the controls
/// below U+0020, but writes the others as they are. Every such character
/// is inside a string, since JSON's own syntax is plain ASCII, so the JSON
/// still parses to the same value. The newlines of pretty-printing are kept.
pub fn for_json(json: &str) -> String {
    let mut out = String::with_capacity(json.len());
    for c in json.chars() {
        if changes_display(c) && c != '\n' {
            // Every such character is in the Basic Multilingual Plane, so
            // one four-digit escape is enough.
            out.push_str(&format!("\\u{:04x}", c as u32));
        } else {
            out.push(c);
        }
    }
    out
}
