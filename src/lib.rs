//! # Techmino Replay Toolkit
//!
//! This is the library part of Techmino Replay Toolkit.
//!
//! This features code designated specifically for the binary, and **everything
//! in this crate may change at any time!** If you want to make something that interfaces
//! with Techmino replays, **use the `libtechmino-replay` library instead:
//! <https://crates.io/crates/libtechmino-replay>**

pub mod cli;
#[cfg(feature = "gui")]
pub mod gui;
#[cfg(feature = "tui")]
pub mod tui;

#[cfg(any(feature = "tui", feature = "gui"))]
mod backend;
mod consts;
#[cfg(feature = "tui")]
mod paths;

/// Either a replay-parsing or I/O error.
#[cfg(feature = "tui")]
#[derive(Debug, thiserror::Error)]
enum ParseOrIoError {
    #[error("Failed to parse replay file: {0}")]
    Parse(#[from] libtechmino_replay::errors::ReplayParseError),
    #[error("Failed to read from replay file: {0}")]
    Io(#[from] std::io::Error),
}
