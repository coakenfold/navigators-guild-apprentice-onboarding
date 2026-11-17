//! Text with colour, for what a command prints (§8: anstream and anstyle).
//!
//! A command builds its text as `Styled`: pieces of text, each with an
//! `anstyle::Style`. `render` escapes each piece with `text::for_terminal`
//! and wraps it in its colour codes, and `main` writes the result through
//! `anstream`, which drops the codes when the output isn't a terminal, as
//! in `tracker list | cat`, or when `NO_COLOR` is set. Since the pieces are
//! escaped before the codes go round them, colour never stops a title
//! edited by hand from being escaped, and a title can't add colour of its
//! own.

use anstyle::{AnsiColor, Color, Style};

use crate::item::{Priority, Status};
use crate::text;

/// Pieces of text, each with its style. A plain piece has `Style::new()`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Styled {
    pub parts: Vec<(String, Style)>,
}

impl Styled {
    pub fn new() -> Styled {
        Styled::default()
    }

    /// Adds plain text.
    pub fn push(&mut self, text: &str) {
        self.push_styled(text, Style::new());
    }

    /// Adds text in `style`.
    pub fn push_styled(&mut self, text: &str, style: Style) {
        if !text.is_empty() {
            self.parts.push((text.to_string(), style));
        }
    }

    /// Adds every piece of `other`.
    pub fn append(&mut self, other: Styled) {
        self.parts.extend(other.parts);
    }

    /// The text without its colour.
    pub fn plain(&self) -> String {
        self.parts.iter().map(|(text, _)| text.as_str()).collect()
    }

    /// The text with each piece escaped, so nothing in it can change what
    /// the terminal shows, and then wrapped in its colour codes. Line
    /// breaks are kept.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for (piece, style) in &self.parts {
            let piece = text::for_terminal(piece, true);
            if style.is_plain() {
                out.push_str(&piece);
            } else {
                out.push_str(&format!(
                    "{}{piece}{}",
                    style.render(),
                    style.render_reset()
                ));
            }
        }
        out
    }
}

impl From<String> for Styled {
    fn from(text: String) -> Styled {
        let mut styled = Styled::new();
        styled.push(&text);
        styled
    }
}

impl From<&str> for Styled {
    fn from(text: &str) -> Styled {
        let mut styled = Styled::new();
        styled.push(text);
        styled
    }
}

const fn color(color: AnsiColor) -> Style {
    Style::new().fg_color(Some(Color::Ansi(color)))
}

/// A heading, such as `list`'s column names or a project's name.
pub const HEADING: Style = Style::new().bold();
/// The `error:` before a message on stderr.
pub const ERROR: Style = color(AnsiColor::Red).bold();
/// The `warning:` before a message on stderr, such as a skipped file.
pub const WARNING: Style = color(AnsiColor::Yellow).bold();

/// A state's colour: the ones that need something from someone stand out,
/// and shelved items, which nobody is working on, fade.
pub fn status(status: Status) -> Style {
    match status {
        Status::Open => Style::new(),
        Status::InProgress => color(AnsiColor::Cyan),
        Status::Review => color(AnsiColor::Yellow),
        Status::Done => color(AnsiColor::Green),
        Status::Shelved => Style::new().dimmed(),
    }
}

/// A priority's colour: high stands out, and low fades.
pub fn priority(priority: Priority) -> Style {
    match priority {
        Priority::High => color(AnsiColor::Red),
        Priority::Medium => Style::new(),
        Priority::Low => Style::new().dimmed(),
    }
}
