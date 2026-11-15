//! A failed command: the message for the user and the exit code (§3).

#[derive(Debug)]
pub struct Error {
    pub code: u8,
    pub message: String,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Exit code 1: bad input, or something that doesn't exist.
    pub fn input(message: String) -> Error {
        Error { code: 1, message }
    }

    /// Exit code 3: the store, or one of its files, can't be read or written.
    pub fn store(message: String) -> Error {
        Error { code: 3, message }
    }
}
