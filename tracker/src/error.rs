//! A failed command: the message for the user and the exit code (§3).

use std::io::ErrorKind;
use std::path::Path;

use serde_json::Value;

#[derive(Debug)]
pub struct Error {
    pub code: u8,
    pub message: String,
    /// For a move the diagram doesn't draw (§2.2), the moves that are
    /// allowed, so an agent can read them under `--json` rather than parse
    /// the message.
    pub allowed: Option<Value>,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Exit code 1: bad input, or something that doesn't exist.
    pub fn input(message: String) -> Error {
        Error {
            code: 1,
            message,
            allowed: None,
        }
    }

    /// Exit code 3: the store, or one of its files, can't be read or written.
    pub fn store(message: String) -> Error {
        Error {
            code: 3,
            message,
            allowed: None,
        }
    }
}

/// Exit code 3 for a file or folder in the store that can't be used, such
/// as `Can't read …/counters.json: Permission denied (os error 13).`, with
/// how to fix it where the error says. `doing` is the verb, such as `read`.
pub fn io(doing: &str, path: &Path, e: &std::io::Error) -> Error {
    let display = path.display();
    let hint = match e.kind() {
        ErrorKind::PermissionDenied => format!(
            " Check its permissions with `ls -ld {display}`: the store and everything in it \
             should be yours, readable and writable by you, as `tracker init` made them."
        ),
        _ => String::new(),
    };
    Error::store(format!("Can't {doing} {display}: {e}.{hint}"))
}

/// The same as `io`, for one of the parts of the store that `tracker init`
/// creates: its folder, `config.json`, `counters.json`, `items/` and the
/// lock, or a file written into one of those folders. Only for these does
/// "not found" mean that `init` would put back what's missing. An item
/// file that's missing isn't one: `init` doesn't create items.
pub fn io_in_store(doing: &str, path: &Path, e: &std::io::Error) -> Error {
    let mut err = io(doing, path, e);
    if e.kind() == ErrorKind::NotFound {
        err.message
            .push_str(" Run `tracker init` to put back what's missing.");
    }
    err
}

/// Where the parser stopped and why, from a serde_json error: the line and
/// column, if it gave them, and its reason without them, such as
/// ``expected `,` or `}` ``.
pub fn json_problem(e: &serde_json::Error) -> (Option<usize>, Option<usize>, String) {
    let text = e.to_string();
    if e.line() == 0 {
        return (None, None, text);
    }
    let at = format!(" at line {} column {}", e.line(), e.column());
    let reason = text.strip_suffix(&at).unwrap_or(&text).to_string();
    (Some(e.line()), Some(e.column()), reason)
}

/// `path`, then where in it, such as `…/XX-2.json, line 3, column 5`.
pub fn at(path: &Path, line: Option<usize>, column: Option<usize>) -> String {
    match (line, column) {
        (Some(line), Some(column)) => format!("{}, line {line}, column {column}", path.display()),
        (Some(line), None) => format!("{}, line {line}", path.display()),
        _ => path.display().to_string(),
    }
}

/// Exit code 3 for a JSON file in the store that can't be parsed (§4.4):
/// its name, the parser's line and reason, then `fix`, which says what to
/// do about it.
pub fn parse(path: &Path, e: &serde_json::Error, fix: &str) -> Error {
    let (line, column, reason) = json_problem(e);
    Error::store(format!(
        "Can't parse {}: {reason}. {fix}",
        at(path, line, column)
    ))
}
