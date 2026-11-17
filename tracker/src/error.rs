//! A failed command: the message for the user and the exit code (§3).

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
