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
//! ## A note on `serde` implementations
//! **For input event data, `serde` implementations do not
//! serialize/deserialize to the game's expected format.**
//!
//! The game uses JSON for metadata, so that should be in the game's expected
//! format.
//!
//! However, for input event data (i.e., keypresses), it uses a specialized
//! VLQ-based format. Therefore, `Serialize`/`Deserialize` implementations on
//! their related structs have **no use in interopping with the game** and may
//! only be useful for if you want to e.g. store it in another format.
//!
//! ## Serialization and Parsing
//!
//! For more information about how to serialize (encode) and deserialize (parse)
//! Techmino replays, check the [`deserialize`] and [`serialize`] module-level
//! documentation.
//!
//! ## Modules
//! - [`consts`]: Some public constants e.g. the amount of pieces Techmino
//!   supports.
//! - [`config`]: Structs related to replay ecnoder and parser configuration.
//! - [`deserialize`]: Parse a replay.
//! - [`errors`]: Error structs and enums for when something went wrong.
//! - [`replay`]: Structs related to the data stored in the replay.
//! - [`serialize`]: Create/encode a replay.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

#[cfg(all(feature = "std", feature = "alloc"))]
compile_error!("Only one of `std` or `alloc` features may be enabled");

#[cfg(not(any(feature = "std", feature = "alloc")))]
compile_error!("You must enable either the `std` feature or the `alloc` feature");

extern crate alloc;

#[cfg(feature = "arbitrary")]
mod arbitrary;

pub mod config;
pub mod consts;
mod convert;
pub mod deserialize;
pub mod errors;
#[deprecated(
    since = "0.2.0",
    note = "this is a compatibility layer and everything here has been relocated to either `replay` or `config`"
)]
pub mod format;
mod macros;
pub mod replay;
pub mod serialize;

#[cfg(test)]
mod test_utils;

#[doc(hidden)]
#[deprecated(since = "0.2.0", note = "import from the `config` module instead")]
pub use crate::config::{InputParseMode, ReplayBufferKind};
#[doc(hidden)]
#[deprecated(since = "0.2.0", note = "import from the `errors` module instead")]
pub use crate::errors::{
    GameInputEventError, NotABool, OwnedTypeError, ReplayParseError, ReplaySerializeError,
    TypeError,
};
#[cfg(feature = "strum")]
#[doc(hidden)]
#[deprecated(
    since = "0.2.0",
    note = "import from the `replay::action` module instead"
)]
pub use crate::replay::action::InputActionKeyIter;
#[doc(hidden)]
#[deprecated(
    since = "0.2.0",
    note = "import from the `replay::action` module instead"
)]
pub use crate::replay::action::{
    InputAction, InputActionKey, InputActionKind, InvalidInputActionKey,
};
#[doc(hidden)]
#[deprecated(since = "0.2.0", note = "import from the `replay` module instead")]
pub use crate::replay::{
    GameInputEvent, GameReplayData, GameReplayMetadata, PlayerSettings, PlayerSettingsMut,
    PlayerSettingsRef, SerializedReplay,
};
