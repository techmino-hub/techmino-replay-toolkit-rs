//! Represents Techmino replay data structures.

#[cfg(feature = "arbitrary")]
use arbitrary::Arbitrary;

use crate::{
    convert::{json_to_bool, json_to_modlist, json_to_str, json_to_u64, modlist_to_json},
    errors::{GameInputEventError, OwnedTypeError, TypeError, ValueVariant},
    macros::metadata_getters_setters,
    replay::action::{InputAction, InputActionKey, InputActionKind},
};
use alloc::{borrow::ToOwned, boxed::Box, fmt, format, string::String, vec::Vec};
use core::{borrow::Borrow, fmt::Debug};
use derive_more::{From, Into};
use serde::{Deserialize, Serialize};
use serde_json::Map;

pub mod action;
mod settings;

pub use settings::{
    PieceColor, PieceColorIter, PlayerSettings, PlayerSettingsMut, PlayerSettingsRef,
};

/// A packed struct representing a single input event in the game.
///
/// Conceptually, an input event consists of a frame number
/// and an input action. The frame number tells when the input event
/// occurred, and the input action tells what happened in that
/// input event.
///
/// # Layout
/// The bitwise layout of this struct is currently unstable.
///
/// However, here's how it currently looks like:
/// ```text
/// 0bA00BBBBB_00CCCCCC_CCCCCCCC_CCCCCCCC_CCCCCCCC_CCCCCCCC_CCCCCCCC_CCCCCCCC
/// ```
/// where:
/// - `0`: Currently unused.
/// - `A`: The [`InputActionKind`] (1 bit)
/// - `B`: The [`InputActionKey`] (5 bits, extendable up to 7, maybe 9)
/// - `C`: The frame number when this event occurred, from the start of the countdown.
///   Note that the countdown ends at frame 180, at which point the game begins.
///   Nevertheless, inputs before that point are still recorded.
///   Note that since the original game uses Lua, which uses floats, it wouldn't
///   handle integers above 2^53 properly, which is why this one was given 54 bits.
///   Also see [`Self::MAX_FRAME`].
///
/// ## A note on `serde` implementations
/// **For input event data, `serde` implementations do not
/// serialize/deserialize to the game's expected format.**
///
/// The game uses JSON for metadata, so that should be in the game's expected
/// format.
///
/// However, for input event data (i.e., keypresses), it uses a specialized
/// VLQ-based format. Therefore, `Serialize`/`Deserialize` implementations on
/// their related structs have **no use in interopping with the game** and may
/// only be useful for if you want to e.g. store it in another format.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "GameInputEventRepr", into = "GameInputEventRepr")]
#[repr(transparent)]
pub struct GameInputEvent(i64);

impl GameInputEvent {
    /// The final frame number where events can be reliably played back by the game.
    pub const MAX_FRAME: u64 = 1 << 53;

    /// Create a new packed [`GameInputEvent`].
    ///
    /// # Errors
    /// For this to work, `frame` must be at most [`Self::MAX_FRAME`].
    /// This will return an error if this condition is not met.
    pub const fn new(frame: u64, action: InputAction) -> Result<Self, GameInputEventError> {
        if frame > Self::MAX_FRAME {
            return Err(GameInputEventError { frame });
        }

        let InputAction { kind, key } = action;

        let kind = kind.into_bool();
        let kind = kind as i64;

        let key = key.into_byte();
        let key = key as i64;

        let res: i64 = kind << 63 | key << 56 | frame.cast_signed();
        Ok(Self(res))
    }

    /// A number representing the frame this event occurred in.
    ///
    /// Note that the game starts at frame 180, and the frames before that
    /// happen during the game start countdown. Nevertheless,
    /// the game still records inputs before the countdown finishes.
    #[must_use]
    pub fn frame(self) -> u64 {
        self.0.cast_unsigned() & 0x003F_FFFF_FFFF_FFFF
    }

    /// The kind of input event this represents.\
    /// That is - whether or not this is a key press event or a key release event.
    #[must_use]
    pub fn kind(self) -> InputActionKind {
        if self.0 < 0 {
            InputActionKind::Release
        } else {
            InputActionKind::Press
        }
    }

    /// The key that is being pressed or released.
    #[must_use]
    #[expect(
        clippy::missing_panics_doc,
        reason = "This function should never panic"
    )]
    pub fn key(self) -> InputActionKey {
        let shifted = (self.0.cast_unsigned() >> 56) as u8;
        let masked = shifted & 0x1F;
        InputActionKey::try_from(masked).expect("invariant breached: invalid input action key")
    }

    /// Gets the action that happened in this event.
    #[must_use]
    pub fn action(self) -> InputAction {
        InputAction {
            key: self.key(),
            kind: self.kind(),
        }
    }
}

impl Debug for GameInputEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GameInputEvent")
            .field("raw", &format!("0x{:X?}", self.0))
            .field("_calc_action", &self.action())
            .field("_calc_frame", &self.frame())
            .finish()
    }
}

impl From<GameInputEvent> for GameInputEventRepr {
    fn from(value: GameInputEvent) -> Self {
        Self {
            action: value.action(),
            frame: value.frame(),
        }
    }
}

impl TryFrom<GameInputEventRepr> for GameInputEvent {
    type Error = GameInputEventError;

    fn try_from(value: GameInputEventRepr) -> Result<Self, Self::Error> {
        Self::new(value.frame, value.action)
    }
}

/// A serializable representation of [`GameInputEvent`] used for serde.
///
/// ## A note on `serde` implementations
/// **For input event data, `serde` implementations do not
/// serialize/deserialize to the game's expected format.**
///
/// The game uses JSON for metadata, so that should be in the game's expected
/// format.
///
/// However, for input event data (i.e., keypresses), it uses a specialized
/// VLQ-based format. Therefore, `Serialize`/`Deserialize` implementations on
/// their related structs have **no use in interopping with the game** and may
/// only be useful for if you want to e.g. store it in another format.
#[derive(Serialize, Deserialize)]
#[serde(rename = "GameInputEvent")]
struct GameInputEventRepr {
    action: InputAction,
    frame: u64,
}

/// A serialized replay, in either `String` or `Vec<u8>` form, depending
/// on the requested replay kind.
///
/// [`Uncompressed`][rbk-unc] or [`Compressed`][rbk-com] replay kinds
/// correspond to the [`Bytes`][Self::Bytes] variant, while the
/// [`Base64`][rbk-b64] replay kind corresponds to the
/// [`Base64`][Self::Base64] variant.
///
/// [rbk-unc]: crate::config::ReplayBufferKind::Uncompressed
/// [rbk-com]: crate::config::ReplayBufferKind::Compressed
/// [rbk-b64]: crate::config::ReplayBufferKind::Base64
#[cfg_attr(feature = "strum", derive(strum::EnumIs))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SerializedReplay {
    /// A serialized replay containing non-string bytes.
    ///
    /// This is what's returned when you call for a [`Compressed`][rbk-com]
    /// or [`Uncompressed`][rbk-unc] replay buffer.
    ///
    /// [rbk-unc]: crate::config::ReplayBufferKind::Uncompressed
    /// [rbk-com]: crate::config::ReplayBufferKind::Compressed
    Bytes(Vec<u8>),
    /// A serialized replay containing base64-encoded data.
    ///
    /// This is what's returned when you call for a [`Base64`][rbk-b64] replay buffer.
    ///
    /// [rbk-b64]: crate::config::ReplayBufferKind::Base64
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

#[cfg(test)]
mod tests {
    use fastrand::Rng;
    #[cfg(feature = "strum")]
    use strum::IntoEnumIterator;

    use super::*;

    #[cfg(feature = "strum")]
    #[test]
    fn test_event_roundtrip() {
        const ROUNDS: usize = 10_000_000;

        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

        for i in 0..ROUNDS {
            let kind: InputActionKind = rng.bool().into();
            let key = rng.choice(InputActionKey::iter()).unwrap();
            let action = InputAction { kind, key };
            let frame = rng.u64(0..=GameInputEvent::MAX_FRAME);

            let Ok(event) = GameInputEvent::new(frame, action) else {
                panic!(
                    "Failed to create GameInputEvent from args:
                    Kind: {kind:?} = {kind_discriminant:?}
                    Key: {key:?} = {key_discriminant:?}
                    Frame: {frame} = {frame:x}",
                    kind_discriminant = core::mem::discriminant(&kind),
                    key_discriminant = core::mem::discriminant(&key),
                );
            };

            let (rt_kind, rt_key, rt_frame) = (event.kind(), event.key(), event.frame());

            assert_eq!(kind, rt_kind);
            assert_eq!(key, rt_key);
            assert_eq!(frame, rt_frame);

            if i % 1_000_000 == 0 {
                eprintln!("{i} of {ROUNDS}");
            }
        }
    }
}

/// A struct representing all the data contained within the game replay.
///
/// ## A note on `serde` implementations
/// **For input event data, `serde` implementations do not
/// serialize/deserialize to the game's expected format.**
///
/// The game uses JSON for metadata, so that should be in the game's expected
/// format.
///
/// However, for input event data (i.e., keypresses), it uses a specialized
/// VLQ-based format. Therefore, `Serialize`/`Deserialize` implementations on
/// their related structs have **no use in interopping with the game** and may
/// only be useful for if you want to e.g. store it in another format.
#[expect(
    clippy::unsafe_derive_deserialize,
    reason = "this is for internal testing purposes only"
)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct GameReplayData {
    /// A list of game input events that happened during the replay.
    pub inputs: Vec<GameInputEvent>,
    /// Metadata contained within the replay data.
    pub metadata: GameReplayMetadata,
}

/// A struct representing the metadata stored within the replay.
///
/// ## A note on `serde` implementations
/// **For input event data, `serde` implementations do not
/// serialize/deserialize to the game's expected format.**
///
/// The game uses JSON for metadata, so that should be in the game's expected
/// format.
///
/// However, for input event data (i.e., keypresses), it uses a specialized
/// VLQ-based format. Therefore, `Serialize`/`Deserialize` implementations on
/// their related structs have **no use in interopping with the game** and may
/// only be useful for if you want to e.g. store it in another format.
#[cfg_attr(feature = "arbitrary", derive(Arbitrary))]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, From, Into)]
#[serde(transparent)]
pub struct GameReplayMetadata {
    /// The inner map that stores the metadata entries.
    #[cfg_attr(feature = "arbitrary", arbitrary(with = crate::arbitrary::arbitrary_json_map))]
    pub map: Map<String, serde_json::Value>,
}

impl GameReplayMetadata {
    /// Creates a new blank [`GameReplayMetadata`] struct.
    ///
    /// This does not allocate by default.
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: serde_json::Map::new(),
        }
    }

    /// Creates a new blank [`GameReplayMetadata`] struct with the given initial
    /// capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: serde_json::Map::with_capacity(capacity),
        }
    }
}

impl GameReplayMetadata {
    /// Gets the key for the `private` entry in the map (is currently "private").
    ///
    /// This is useful for manually indexing into the map to get a
    /// specific entry. However, usually, the default `get_*` or
    /// `set_*` methods should be enough for almost all usecases.
    pub const KEY_PRIVATE: &'static str = "private";

    /// Gets the key for the `settings` entry in the map (is currently "setting").
    ///
    /// This is useful for manually indexing into the map to get a
    /// specific entry. However, usually, the default `get_*` or
    /// `set_*` methods should be enough for almost all usecases.
    pub const KEY_SETTINGS: &'static str = "setting";

    /// The 'private' field of the replay, used to store mode-specific data.\
    /// Its contents differ based on the mode played.\
    /// Currently, only the `custom_clear` and `custom_puzzle` modes are known to
    /// store any data here.
    #[must_use]
    pub fn get_private(&self) -> Option<&serde_json::Value> {
        self.map.get(Self::KEY_PRIVATE)
    }

    /// The 'private' field of the replay, used to store mode-specific data.\
    /// Its contents differ based on the mode played.\
    /// Currently, only the `custom_clear` and `custom_puzzle` modes are known to
    /// store any data here.
    #[must_use]
    pub fn get_private_mut(&mut self) -> Option<&mut serde_json::Value> {
        self.map.get_mut(Self::KEY_PRIVATE)
    }

    /// The 'private' field of the replay, used to store mode-specific data.\
    /// Its contents differ based on the mode played.\
    /// Currently, only the `custom_clear` and `custom_puzzle` modes are known to
    /// store any data here.
    ///
    /// # Returns
    /// Returns the old value of the field, if there was any.
    pub fn set_private(&mut self, value: serde_json::Value) -> Option<serde_json::Value> {
        if let Some(mutref) = self.map.get_mut(Self::KEY_PRIVATE) {
            return Some(core::mem::replace(mutref, value));
        }
        self.map.insert(Self::KEY_PRIVATE.to_owned(), value)
    }

    /// The 'private' field of the replay, used to store mode-specific data.\
    /// Its contents differ based on the mode played.\
    /// Currently, only the `custom_clear` and `custom_puzzle` modes are known to
    /// store any data here.
    ///
    /// # Returns
    /// Returns the old value of the field, if there was any.
    pub fn remove_private(&mut self) -> Option<serde_json::Value> {
        self.map.remove(Self::KEY_PRIVATE)
    }

    /// The settings of the game when the run was played.
    ///
    /// # Strict Getter
    /// Strict getter methods attempt to convert the stored value into
    /// the normal strictly-typed version for convenience.
    ///
    /// # Errors
    /// The [`TypeError`] struct contains a reference to the raw
    /// [`serde_json::Value`] value if you need it.
    /// Alternatively, use the shortcut method `get_*_or_raw()` if you
    /// don't need the error reason.
    #[must_use]
    pub fn get_settings(&self) -> Option<Result<PlayerSettingsRef<'_>, TypeError<'_>>> {
        const EXPECTED_TYPE: ValueVariant = ValueVariant::Object;

        let entry = self.map.get(Self::KEY_SETTINGS)?;

        let variant = ValueVariant::from(entry);

        let serde_json::Value::Object(map) = entry else {
            debug_assert_ne!(
                variant, EXPECTED_TYPE,
                "Invariant breached: expected type shouldn't match (this is a `libtechmino-replay` bug, please report there)"
            );

            return Some(Err(TypeError {
                key: Self::KEY_SETTINGS,
                exp_ty: EXPECTED_TYPE,
                value: entry,
            }));
        };

        debug_assert_eq!(
            variant, EXPECTED_TYPE,
            "Invariant breached: expected type should match (this is a `libtechmino-replay` bug, please report there)"
        );

        Some(Ok(PlayerSettingsRef { map }))
    }

    /// The settings of the game when the run was played.
    #[must_use]
    pub fn get_settings_mut(&mut self) -> Option<Result<PlayerSettingsMut<'_>, TypeError<'_>>> {
        const EXPECTED_TYPE: ValueVariant = ValueVariant::Object;

        let entry = self.map.get_mut(Self::KEY_SETTINGS)?;

        let entry_kind = ValueVariant::from(&*entry);

        let serde_json::Value::Object(map) = entry else {
            debug_assert_ne!(
                entry_kind, EXPECTED_TYPE,
                "Invariant breached: expected type shouldn't match (this is a `libtechmino-replay` bug, please report there)"
            );

            return Some(Err(TypeError {
                key: Self::KEY_SETTINGS,
                exp_ty: EXPECTED_TYPE,
                value: entry,
            }));
        };

        debug_assert_eq!(
            entry_kind, EXPECTED_TYPE,
            "Invariant breached: expected type should match (this is a `libtechmino-replay` bug, please report there)"
        );

        Some(Ok(PlayerSettingsMut { map }))
    }

    /// The settings of the game when the run was played.
    #[must_use]
    pub fn get_settings_or_raw(&self) -> Option<Result<PlayerSettingsRef<'_>, &serde_json::Value>> {
        let entry = self.map.get(Self::KEY_SETTINGS)?;

        let serde_json::Value::Object(map) = entry else {
            return Some(Err(entry));
        };

        Some(Ok(PlayerSettingsRef { map }))
    }

    /// The settings of the game when the run was played.
    #[must_use]
    pub fn get_settings_mut_or_raw(
        &mut self,
    ) -> Option<Result<PlayerSettingsMut<'_>, &mut serde_json::Value>> {
        let entry = self.map.get_mut(Self::KEY_SETTINGS)?;

        let serde_json::Value::Object(map) = entry else {
            return Some(Err(entry));
        };

        Some(Ok(PlayerSettingsMut { map }))
    }

    /// The settings of the game when the run was played.
    #[must_use]
    pub fn get_settings_raw(&self) -> Option<&serde_json::Value> {
        self.map.get(Self::KEY_SETTINGS)
    }

    /// The settings of the game when the run was played.
    #[must_use]
    pub fn get_settings_raw_mut(&mut self) -> Option<&mut serde_json::Value> {
        self.map.get_mut(Self::KEY_SETTINGS)
    }

    /// The settings of the game when the run was played.
    ///
    /// # Returns
    /// Returns the old value of the field.
    /// - If there was no old value of the field, returns `None`.
    /// - If conversion succeeds, returns the strictly typed version
    ///   (`Some(Ok(T))`).
    /// - Otherwise, returns the error that happened while attempting to
    ///   convert the value.
    ///   (`Some(Err(OwnedTypeError)))`)
    ///   - You can then try to get the inner value using
    ///     [`.inner()`][crate::errors::OwnedTypeError::inner]
    ///
    /// # Errors
    /// Converting the old stored value into a strict form may fail.
    ///
    /// When this happens, the map is still set properly.
    ///
    /// You can then display the error using its `Display` impl or try
    /// to get the inner value using
    /// [`.inner()`][crate::errors::OwnedTypeError::inner].
    pub fn set_settings(
        &mut self,
        value: serde_json::Map<String, serde_json::Value>,
    ) -> Option<Result<PlayerSettings, OwnedTypeError>> {
        const EXPECTED_TYPE: ValueVariant = ValueVariant::Object;

        let Some(dest) = self.map.get_mut(Self::KEY_SETTINGS) else {
            self.map.insert(
                Self::KEY_SETTINGS.to_owned(),
                serde_json::Value::Object(value),
            );

            return None;
        };

        let variant = ValueVariant::from(&*dest);

        // Optimization: Replace inner `Map`s when possible instead of entire
        // `serde_json::Value`

        if let serde_json::Value::Object(dest) = dest {
            debug_assert_eq!(
                variant, EXPECTED_TYPE,
                "Invariant breached: expected type should match (this is a `libtechmino-replay` bug, please report there)"
            );

            let old = core::mem::replace(dest, value);
            Some(Ok(PlayerSettings { map: old }))
        } else {
            debug_assert_ne!(
                variant, EXPECTED_TYPE,
                "Invariant breached: expected type shouldn't match (this is a `libtechmino-replay` bug, please report there)"
            );

            let src = serde_json::Value::Object(value);
            let old = core::mem::replace(dest, src);

            Some(Err(OwnedTypeError {
                key: Self::KEY_SETTINGS,
                exp_ty: EXPECTED_TYPE,
                value: old,
            }))
        }
    }

    /// The settings of the game when the run was played.
    ///
    /// # Returns
    /// Returns the old value of the field.
    /// - If there was no old value of the field, returns `None`.
    /// - If conversion succeeds, returns the strictly typed version
    ///   (`Some(Ok(T))`).
    /// - Otherwise, returns the error that happened while attempting to
    ///   convert the value.
    ///   (`Some(Err(OwnedTypeError)))`)
    ///   - You can then try to get the inner value using
    ///     [`.inner()`][crate::errors::OwnedTypeError::inner]
    ///
    /// # Errors
    /// Converting the old stored value into a strict form may fail.
    ///
    /// When this happens, the map is still set properly.
    ///
    /// You can then display the error using its `Display` impl or try
    /// to get the inner value using
    /// [`.inner()`][crate::errors::OwnedTypeError::inner].
    pub fn remove_settings(&mut self) -> Option<Result<PlayerSettings, OwnedTypeError>> {
        const EXPECTED_TYPE: ValueVariant = ValueVariant::Object;

        let json = self.map.remove(Self::KEY_SETTINGS)?;
        let variant = ValueVariant::from(&json);

        let res = PlayerSettings::try_from(json);

        match res {
            Ok(ps) => {
                debug_assert_eq!(
                    variant, EXPECTED_TYPE,
                    "Invariant breached: expected type should match (this is a `libtechmino-replay` bug, please report there)"
                );
                Some(Ok(ps))
            }
            Err(value) => {
                debug_assert_ne!(
                    variant, EXPECTED_TYPE,
                    "Invariant breached: expected type shouldn't match (this is a `libtechmino-replay` bug, please report there)"
                );
                Some(Err(OwnedTypeError {
                    key: Self::KEY_SETTINGS,
                    exp_ty: EXPECTED_TYPE,
                    value,
                }))
            }
        }
    }

    metadata_getters_setters! {
        (map);
        /// Whether or not the replay is marked as a TAS.
        "tasUsed" tas_used: bool where { from_json: json_to_bool },

        /// The username of the player.
        "player" player: &str | String where { from_json: json_to_str },

        /// The seed for the random number generator.
        "seed" seed: u64 where { from_json: json_to_u64 },

        /// The version of the game the replay was made in.
        ///
        /// Usually conforms to semver (major.minor.patch), but some mods
        /// may use a different or custom format.
        "version" version: &str | String where { from_json: json_to_str },

        /// The local date and time that the replay was initially created.
        ///
        /// The `date` entry contains a string with the format
        /// `%Y/%m/%d %H:%M:%S` (following the C `strftime` function format).
        ///
        /// (This format string is available in
        /// [the `consts` module][crate::consts])
        ///
        /// The timezone used is the player's timezone, **not UTC**.
        ///
        /// Example outputs:
        /// - `2022/09/28 23:09:59`
        /// - `2024/02/29 21:08:54`
        /// - `2016/12/31 23:59:60`
        ///     - The [Lua 5.4 documentation on os.date][os.date] specifically
        ///       mentioned that `%S` ranges between 0–61 because of
        ///       the possibility of [leap seconds][wp-leap-secs]
        ///
        /// [os.date]: https://www.lua.org/manual/5.4/manual.html#pdf-os.date
        /// [wp-leap-secs]: https://en.wikipedia.org/wiki/Leap_second
        "date" date: &str | String where { from_json: json_to_str },

        /// A list of mods applied to the run.
        ///
        /// It's in the format of [mod, value], where mod is the mod ID and value is the value given to the mod.
        "mod" mods: Vec<(u64, serde_json::Value)> where {
            from_json: json_to_modlist,
            to_json: modlist_to_json,
        },

        /// The name of the mode that was played.
        ///
        /// This refers to the internal/codename of the mode, i.e. `sprint_10l` instead of `Sprint 10L`.
        "mode" mode: &str | String where { from_json: json_to_str },

        // "setting" settings: PlayerSettings where { from_json: serde_json::Value::as_object },
    }
}
