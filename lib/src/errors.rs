//! Represents the different kinds of errors that could happen.

#[cfg(all(doc, not(feature = "std")))]
use alloc::vec::Vec;

use crate::{consts::TOTAL_PIECE_COUNT, replay::GameInputEvent};
use alloc::string::{FromUtf8Error, String};
use base64::DecodeError;
use core::fmt::Display;
use libtechmino_vlq::VlqDecodeError;
use miniz_oxide::{MZError, deflate::core::TDEFLStatus, inflate::TINFLStatus};
use thiserror::Error;

/// An entry had an unexpected type and could not be converted into the
/// standardized type.
///
/// This is the owned variant of [`TypeError`].
#[derive(Debug, Error)]
pub struct OwnedTypeError {
    /// The key used to index into the map.
    pub(crate) key: &'static str,
    /// The expected variant to be retrieved.
    pub(crate) exp_ty: ValueVariant,
    /// The value retrieved from the JSON object.
    pub(crate) value: serde_json::Value,
}

impl OwnedTypeError {
    /// Borrows the inner value retrieved from the JSON object that
    /// had an unexpected type.
    #[must_use]
    pub fn inner(&self) -> &serde_json::Value {
        &self.value
    }

    /// Deconstructs this type to take its inner value.
    #[must_use]
    pub fn take_inner(self) -> serde_json::Value {
        self.value
    }

    /// Create a borrowed [`TypeError`] from this owned variant.
    #[must_use]
    pub fn get_ref(&self) -> TypeError<'_> {
        TypeError {
            key: self.key,
            exp_ty: self.exp_ty,
            value: &self.value,
        }
    }
}

impl Display for OwnedTypeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <TypeError as Display>::fmt(&self.get_ref(), f)
    }
}

/// An entry had an unexpected type and could not be converted into the
/// standardized type.
#[derive(Debug, Error)]
pub struct TypeError<'a> {
    /// The key used to index into the map.
    pub(crate) key: &'static str,
    /// The expected variant to be retrieved.
    pub(crate) exp_ty: ValueVariant,
    /// The value retrieved from the JSON object.
    pub(crate) value: &'a serde_json::Value,
}

impl<'a> TypeError<'a> {
    /// Returns the inner value retrieved from the JSON object that
    /// had an unexpected type.
    #[must_use]
    pub fn inner(&self) -> &'a serde_json::Value {
        self.value
    }
}

impl Display for TypeError<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Entry with key {key} had an unexpected type \
            {val_ty} instead of the expected {exp_ty}",
            key = self.key,
            exp_ty = self.exp_ty.to_str(),
            val_ty = ValueVariant::from(self.value).to_str(),
        )
    }
}

/// Represents the different kinds of [`serde_json::Value`]s that could be expected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ValueVariant {
    /// Null.
    Null,
    /// A boolean.
    Bool,
    /// A number.
    Number,
    /// A floating-point number representable by binary64 (`f64`).
    ///
    /// Subtype of [`Number`][Self::Number].
    Float,
    /// An integer number representable by an unsigned byte (`u8`).
    ///
    /// Subtype of [`Number`][Self::Number].
    Byte,
    /// An integer number representable by an unsigned long/quadword (`u64`).
    ///
    /// Subtype of [`Number`][Self::Number].
    Long,
    /// A string.
    String,
    /// A formatted string representing a naive date/time with no timezone.
    #[cfg(feature = "chrono")]
    NaiveDateTimeString,
    /// A JSON array of any length.
    Array,
    /// A JSON array specifically with a length of [`TOTAL_PIECE_COUNT`].
    ///
    /// Subtype of [`Array`][Self::Array].
    PieceArray,
    /// A JSON array specifically with a length of [`TOTAL_PIECE_COUNT`] that
    /// contains valid color indices.
    ///
    /// Subtype of [`Array`][Self::Array].
    PieceColorArray,
    /// A JSON object/map.
    Object,
}

impl ValueVariant {
    const fn to_str(self) -> &'static str {
        match self {
            ValueVariant::Null => "null",
            ValueVariant::Bool => "bool",
            ValueVariant::Number => "(unspecified kind of) number",
            ValueVariant::Float => "64-bit floating-point number",
            ValueVariant::Byte => "8-bit unsigned integer",
            ValueVariant::Long => "64-bit unsigned integer",
            ValueVariant::String => "string",
            #[cfg(feature = "chrono")]
            ValueVariant::NaiveDateTimeString => {
                use crate::consts::METADATA_DATE_FORMAT;
                const_format::formatc!("datetime string with format '{METADATA_DATE_FORMAT}'")
            }
            ValueVariant::Array => "array",
            ValueVariant::PieceArray => {
                const_format::formatc!("array with {TOTAL_PIECE_COUNT} elements")
            }
            ValueVariant::PieceColorArray => {
                const_format::formatc!(
                    "array of valid color indices with {TOTAL_PIECE_COUNT} elements"
                )
            }
            ValueVariant::Object => "object",
        }
    }
}

impl From<&serde_json::Value> for ValueVariant {
    fn from(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(_) => Self::Bool,
            serde_json::Value::Number(_) => Self::Number,
            serde_json::Value::String(_) => Self::String,
            serde_json::Value::Array(v) if v.len() == TOTAL_PIECE_COUNT => Self::PieceArray,
            serde_json::Value::Array(_) => Self::Array,
            serde_json::Value::Object(_) => Self::Object,
        }
    }
}

/// The error type for when the game input event couldn't be created.
#[derive(Debug, Error)]
#[error(
    "Failed to create GameInputEvent: Frame number {frame} is greater than max of {max_frame}",
    max_frame = GameInputEvent::MAX_FRAME
)]
pub struct GameInputEventError {
    pub(crate) frame: u64,
}

/// The error type for when a u8 greater than 1 tried to be converted
/// into a bool.
#[derive(Debug, Error)]
#[error("Expected value of either 0 or 1, found {value}")]
pub struct NotABool {
    pub(crate) value: u8,
}

/// An error from parsing the replay data.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ReplayParseError {
    /// An error occurred when zlib tried to decompress the replay data.
    #[error("zlib failed to decompress the replay data")]
    ZlibDecompressError {
        /// The internal zlib status.
        status: TINFLStatus,
        /// The miniz failure status code.
        mz_error: MZError,
    },

    /// An error occurred while parsing the base64 string.
    ///
    /// See [`DecodeError`] for more information.
    #[error("the given base64 string was not valid base64")]
    Base64DecodeError(DecodeError),

    /// The separator between the replay metadata and the input data was not found.
    ///
    /// The separator is a linefeed character (`b'\n'`).
    #[error("failed to find separator between replay metadata and input data")]
    MetadataSeparatorNotFound,

    /// The metadata was found to not be valid UTF-8.
    ///
    /// See [`FromUtf8Error`] for more information.
    #[error("metadata is not valid utf-8")]
    MetadataNotUtf8(#[from] FromUtf8Error),

    /// The metadata could not be deserialized into the [`GameReplayMetadata`][1] struct,
    /// possibly due to missing values.
    ///
    /// See [`serde_json`'s Error type][serde_json::Error] for more information.
    ///
    /// [1]: crate::replay::GameReplayMetadata
    #[error("failed to deserialize metadata")]
    MetadataDeserializeError(#[from] serde_json::Error),

    /// The mode in which to parse the inputs could not be inferred from the version string.
    ///
    /// Contains the [`String`] or the [`serde_json::Value`] of the version
    /// entry if it was found.
    ///
    /// To fix this error, consider passing in the input parse mode explicitly.
    #[error("could not infer input parse mode from version metadata")]
    UnknownInputParseMode(Option<Result<String, serde_json::Value>>),

    /// The input data was malformed and could not be decoded from the VLQ stream.
    #[error("input data contains invalid vlq")]
    MalformedVlqData {
        /// The inner VLQ decoding error.
        #[from]
        inner: VlqDecodeError,
    },

    /// The input data was malformed and could not be casted into the proper enum types.
    #[error("malformed input data")]
    MalformedInputData {
        /// The unprocessed frame of the associated input event.
        ///
        /// This is what's actually stored in the input data, and is
        /// the same as `frame` if [`InputParseMode`][1] is
        /// [`Absolute`][2].
        ///
        /// [1]: crate::config::InputParseMode
        /// [2]: crate::config::InputParseMode::Absolute
        raw_frame: u64,
        /// The "frame"/time value of the associated input event.
        ///
        /// This is the same as `raw_frame` if [`InputParseMode`][1] is
        /// [`Absolute`][2].
        ///
        /// [1]: crate::config::InputParseMode
        /// [2]: crate::config::InputParseMode::Absolute
        frame: u64,
        /// The action of the associated input event.
        ///
        /// See [`InputAction`][crate::replay::action::InputAction] for more
        /// details.
        action: u64,
    },

    /// Only a portion of the replay was given, i.e.,
    /// more replay data should have been fed
    #[error("replay data unexpectedly ended")]
    UnexpectedEnd,
}

impl From<DecodeError> for ReplayParseError {
    fn from(value: DecodeError) -> Self {
        Self::Base64DecodeError(value)
    }
}

/// An error from serializing the replay data, e.g. to base64.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ReplaySerializeError {
    /// The mode in which to serialize the inputs could not be inferred from the version string.
    ///
    /// Contains the [`String`] or the [`serde_json::Value`] of the version
    /// entry if it was found.
    ///
    /// To fix this error, consider passing in the input parse mode explicitly.
    #[error("could not infer input parse mode from version metadata")]
    UnknownInputParseMode(Option<Result<String, serde_json::Value>>),

    /// The input [`Vec`] isn't sorted in relative-mode encoding.
    ///
    /// The relative-mode serializer expects the input [`Vec`] to be sorted,
    /// otherwise the replay is unrepresentable in that mode.
    ///
    /// The absolute-mode serializer does NOT explicitly check the input's
    /// sorting state.
    ///
    /// In any case, the unmodified game will probably handle unsorted inputs in
    /// a bizzare way.
    ///
    /// To fix this error, consider calling [`GameReplayData::sort_inputs()`][1]
    /// or sorting your input event sequence some other way before serializing it.
    ///
    /// [1]: crate::replay::GameReplayData::sort_inputs
    #[error(
        "unsorted input data: found input for frame {unsorted_time} after input for frame {prev_time}"
    )]
    UnsortedInput {
        /// The frame number of the previous data point.
        prev_time: u64,
        /// The frame number of the first data point which caused the array to not be sorted.
        unsorted_time: u64,
    },

    /// The metadata could not be serialized into JSON.
    ///
    /// See [`serde_json`'s Error type][serde_json::Error] for more information.
    #[error("failed to serialize metadata as JSON")]
    MetadataSerializeError(serde_json::Error),

    /// There was an attempt to encode an oversized `u64` into the VLQ format.
    #[error("could not fit {number} into the VLQ format")]
    VlqOverflow {
        /// The `u64` value that couldn't be encoded into the VLQ format.
        number: u64,
    },

    /// Something went wrong relating to compression.
    #[error("compression error")]
    ZlibError {
        /// The TDEFL status returned from the compressor.
        tdefl_status: TDEFLStatus,
    },
}

impl From<serde_json::Error> for ReplaySerializeError {
    fn from(value: serde_json::Error) -> Self {
        Self::MetadataSerializeError(value)
    }
}

impl From<TDEFLStatus> for ReplaySerializeError {
    fn from(value: TDEFLStatus) -> Self {
        Self::ZlibError {
            tdefl_status: value,
        }
    }
}

/// Could not infer the replay kind.
#[derive(Debug, thiserror::Error)]
#[error("Unknown first byte in replay stream: {first_byte}")]
pub struct UnknownReplayKind {
    pub(crate) first_byte: u8,
}
