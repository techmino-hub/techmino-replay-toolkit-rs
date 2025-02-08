//! # Techmino Replay Toolkit
//! 
//! A library for [parsing and serializing] Techmino replays.
//! 
//! 
//! 
//! [parsing and serializing]: <https://en.wikipedia.org/wiki/Serialization>

// TODO: Improve crate-level docs and more tests

#![warn(missing_docs)]

mod deserialize;
mod serialize;
mod types;
pub use types::*;

#[cfg(test)]
mod tests;