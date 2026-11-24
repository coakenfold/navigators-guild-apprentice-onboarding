//! The tracker library: the store, its files and the item format.
//! `src/main.rs` holds the command line and calls into these modules.

pub mod checks;
pub mod config;
pub mod error;
pub mod gates;
pub mod git;
pub mod item;
pub mod judge;
pub mod states;
pub mod store;
pub mod styled;
pub mod text;
pub mod triggers;
pub mod tty;
