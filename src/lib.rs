//! # Techmino Replay Toolkit
//!
//! A library for [parsing and serializing] Techmino replays.
//!
//!
//!
//! [parsing and serializing]: <https://en.wikipedia.org/wiki/Serialization>

// TODO: Improve crate-level docs and more tests

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

#[cfg(all(feature = "std", feature = "alloc"))]
compile_error!("Only one of `std` or `alloc` features may be enabled");

#[cfg(not(any(feature = "std", feature = "alloc")))]
compile_error!("You must enable either the `std` feature or the `alloc` feature");

extern crate alloc;

mod deserialize;
mod serialize;
mod types;
pub use types::*;

#[cfg(test)]
mod tests;
