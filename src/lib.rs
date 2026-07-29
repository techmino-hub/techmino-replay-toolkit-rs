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

mod consts;
