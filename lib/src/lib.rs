//! # Techmino Replay Toolkit
//!
//! A library for parsing and serializing Techmino replays.
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
//! # Serialization and Parsing
//!
//! For more information about how to serialize and deserialize (parse) Techmino replays,
//! check the [`deserialize`] and [`serialize`] module-level documentation.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

#[cfg(all(feature = "std", feature = "alloc"))]
compile_error!("Only one of `std` or `alloc` features may be enabled");

#[cfg(not(any(feature = "std", feature = "alloc")))]
compile_error!("You must enable either the `std` feature or the `alloc` feature");

extern crate alloc;

#[cfg(feature = "arbitrary")]
mod arbitrary;

/// Module for any constants related to Techmino.
pub mod consts {
    /// The total amount of pieces in the current game.
    ///
    /// There are currently 29 elements:
    /// - 1 monomino
    /// - 1 domino
    /// - 2 trominoes
    /// - 7 tetrominoes
    /// - 18 pentominoes
    pub const TOTAL_PIECE_COUNT: usize = 29;
}

mod action;
pub mod deserialize;
pub mod format;
mod macros;
pub mod serialize;
mod types;

#[cfg(test)]
mod test_utils;

pub use action::*;
pub use format::*;
pub use types::*;
