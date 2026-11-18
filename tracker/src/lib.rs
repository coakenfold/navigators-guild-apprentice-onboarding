//! The tracker library: the store, its files and the item format.
//! `src/main.rs` holds the command line and calls into these modules.

pub mod config;
pub mod error;
pub mod gates;
pub mod item;
pub mod states;
pub mod store;
pub mod styled;
pub mod text;
pub mod tty;
