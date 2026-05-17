//! # Techmino Replay Toolkit
//!
//! A library for [parsing and serializing] Techmino replays.
//!
//! ## Overview of Main Data Structures
//! - [`GameReplayData`] contains all the data in a replay.
//!     - [`GameReplayMetadata`] contains all the metadata in the replay, like the settings
//!       used for the replay and which mode is played.
//!     - <code>Vec<[GameInputEvent]></code> contains a list of all input events in the replay. It contains
//!         - the frame number when the event occured, as well as
//!         - an [`InputAction`] which contains the action that happens at that point.
//!             - [`InputActionKind`] tells whether or not it was a key press or a key release.
//!             - [`InputActionKey`] tells which key was acted upon.
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

mod action;
mod deserialize;
mod serialize;
mod types;
mod vlq;

pub use action::*;
pub use types::*;

#[cfg(test)]
mod tests;
