//! Module for representing the three different ways to have inputs/outputs in this library.

use core::borrow::Borrow;

/// Represents the different kinds of ways that Techmino replays could be represented.
///
/// The most common ones are [`Base64`][Self::Base64] for copy-pasteable text from the
/// "Replays" menu and the [`Compressed`][Self::Compressed] from the `.rep` files in the
/// save directory.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReplayBufferKind {
    /// Base64-encoding of zlib-compressed bytes.
    ///
    /// This is the form of replay data you get when you copy
    /// a replay in the game's "Replays" menu.
    Base64,
    /// Zlib-compressed bytes.
    ///
    /// This is the form of replay data the game stores in
    /// `<game_save_dir>/replay/*.rep`.
    Compressed,
    /// Uncompressed bytes.
    ///
    /// This form is not seen anywhere in the game, but useful
    /// if you have already decompressed a `.rep` file in
    /// memory.
    ///
    /// This format will not be directly playable using the unmodified game.
    Uncompressed,
}

/// A serialized replay, in either `String` or `Vec<u8>` form, depending
/// on the requested replay kind.
///
/// [`Uncompressed`][ReplayBufferKind::Uncompressed] or
/// [`Compressed`][ReplayBufferKind::Compressed] replay kinds correspond to the
/// [`Bytes`][Self::Bytes] variant, while the
/// [`Base64`][ReplayBufferKind::Base64] replay kind corresponds to the
/// [`Base64`][Self::Base64] variant.
#[cfg_attr(feature = "strum", derive(strum::EnumIs))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SerializedReplay {
    /// A serialized replay containing non-string bytes.
    ///
    /// This is what's returned when you call for a
    /// [`Compressed`][ReplayBufferKind::Compressed] or
    /// [`Uncompressed`][ReplayBufferKind::Uncompressed]
    /// replay buffer.
    Bytes(Vec<u8>),
    /// A serialized replay containing base64-encoded data.
    ///
    /// This is what's returned when you call for a
    /// [`Base64`][ReplayBufferKind::Base64] replay buffer.
    Base64(String),
}

impl SerializedReplay {
    /// Gets the byte representation of this serialized replay.
    ///
    /// Returns a reference to the byte slice or the byte slice representation of the
    /// string, based on the enum variant.
    ///
    /// This function never fails.
    ///
    /// # Example
    /// ```
    /// use libtechmino_replay::format::SerializedReplay;
    ///
    /// let serialized = SerializedReplay::Bytes(vec![1, 2, 3]);
    /// assert_eq!(serialized.as_bytes(), &[1, 2, 3]);
    ///
    /// let serialized = SerializedReplay::Base64("VGVjaG1pbm8gaXMgZnVuIQo=".into());
    /// assert_eq!(serialized.as_bytes(), b"VGVjaG1pbm8gaXMgZnVuIQo=".as_slice());
    /// ```
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Bytes(b) => b.as_slice(),
            Self::Base64(str) => str.as_bytes(),
        }
    }

    /// Attempts to get the base64 string representation of this
    /// serialized replay.
    ///
    /// Returns `None` if this serialized replay does not use the base64
    /// representation.
    ///
    /// This returns a reference to the string. If you want an owned version,
    /// use the [`TryInto<String>`][core::convert::TryInto] implementation.
    ///
    /// # Example
    /// ```
    /// use libtechmino_replay::format::SerializedReplay;
    ///
    /// let serialized = SerializedReplay::Bytes(vec![1, 2, 3]);
    /// assert_eq!(serialized.try_as_string(), None);
    ///
    /// let serialized = SerializedReplay::Base64("VGVjaG1pbm8gaXMgZnVuIQo=".into());
    /// assert_eq!(serialized.try_as_string(), Some("VGVjaG1pbm8gaXMgZnVuIQo="));
    /// ```
    #[must_use]
    pub const fn try_as_string(&self) -> Option<&str> {
        match self {
            Self::Bytes(_) => None,
            Self::Base64(str) => Some(str.as_str()),
        }
    }
}

impl AsRef<[u8]> for SerializedReplay {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Bytes(bytes) => bytes.as_slice(),
            Self::Base64(ascii) => ascii.as_bytes(),
        }
    }
}

impl Borrow<[u8]> for SerializedReplay {
    fn borrow(&self) -> &[u8] {
        match self {
            Self::Bytes(bytes) => bytes.as_slice(),
            Self::Base64(ascii) => ascii.as_bytes(),
        }
    }
}

impl From<SerializedReplay> for Vec<u8> {
    fn from(val: SerializedReplay) -> Self {
        match val {
            SerializedReplay::Bytes(b) => b,
            SerializedReplay::Base64(s) => s.into_bytes(),
        }
    }
}

impl From<SerializedReplay> for Box<[u8]> {
    fn from(val: SerializedReplay) -> Self {
        match val {
            SerializedReplay::Bytes(b) => b.into_boxed_slice(),
            SerializedReplay::Base64(s) => s.into_bytes().into_boxed_slice(),
        }
    }
}

impl TryFrom<SerializedReplay> for String {
    /// The original serialized replay.
    type Error = SerializedReplay;

    fn try_from(value: SerializedReplay) -> Result<Self, Self::Error> {
        match value {
            SerializedReplay::Bytes(_) => Err(value),
            SerializedReplay::Base64(string) => Ok(string),
        }
    }
}
