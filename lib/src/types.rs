//! General types and structs to represent metadata and other trivial data.

#[cfg(feature = "arbitrary")]
use arbitrary::Arbitrary;

use derive_more::{From, Into};
use serde_json::Map;

use crate::{
    consts::TOTAL_PIECE_COUNT,
    macros::{metadata_getters_setters, setting_getters_setters},
    InputAction, InputActionKey, InputActionKind,
};
use alloc::{
    fmt::{self},
    format,
    string::{FromUtf8Error, String},
    vec::Vec,
};
use base64::DecodeError;
use core::fmt::Debug;
use libtechmino_vlq::VlqDecodeError;
use miniz_oxide::{deflate::core::TDEFLStatus, inflate::TINFLStatus, MZError};
use semver::Version;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
            return Err(GameInputEventError);
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
        reason = "This function should never panic\
        unless the programmer of this library made a mistake\
        or if an unsound transmutation was done"
    )]
    pub fn key(self) -> InputActionKey {
        let shifted = (self.0.cast_unsigned() >> 56) as u8;
        let masked = shifted & 0x1F;
        InputActionKey::try_from(masked).expect("invalid input action key")
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

/// An entry had an unexpected type and could not be converted into the
/// standardized type.
#[derive(Debug, Error)]
#[error(
    "An entry had an unexpected type and could not be converted into the \
    standardized type."
)]
pub struct TypeError;

/// The error type for when the game input event couldn't be created.
#[derive(Debug, Error)]
#[error("Failed to create GameInputEvent")]
pub struct GameInputEventError;

/// A struct representing all the data contained within the game replay.
///
/// # A note on the `serde` implementations
/// Although this struct derives `Serialize` and `Deserialize`, these
/// impls do not serialize or deserialize to/from the format used by
/// the game.
///
/// Please use the inherent parse and serialize methods for interacting
/// with the game's expected formats.
///
/// This struct derives `Serialize` and `Deserialize` mainly if you want
/// to serialize/deserialize to your own format, e.g. RON or CBOR.
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

/// A struct representing the settings of the player who made the replay.
#[derive(Debug, PartialEq, From, Into, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlayerSettings {
    /// The inner map that stores the setting entries.
    pub map: serde_json::Map<String, serde_json::Value>,
}

impl PlayerSettings {
    /// Creates a new blank [`PlayerSettings`] struct.
    ///
    /// This does not allocate by default.
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: serde_json::Map::new(),
        }
    }

    /// Creates a new blank [`PlayerSettings`] struct with a specified number of
    /// entries allocated.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: serde_json::Map::with_capacity(capacity),
        }
    }

    /// Creates a new [`PlayerSettingsRef`] struct pointing to this
    /// struct's owned map.
    ///
    /// This is different than an [`AsRef`][core::convert::AsRef] implementation
    /// since this function doesn't directly return a reference to a struct, but
    /// a struct containing a reference.
    #[must_use]
    pub fn as_ref(&self) -> PlayerSettingsRef<'_> {
        PlayerSettingsRef { map: &self.map }
    }

    /// Creates a new [`PlayerSettingsMut`] struct pointing to this
    /// struct's owned map.
    ///
    /// This is different than an [`AsRef`][core::convert::AsRef] implementation
    /// since this function doesn't directly return a reference to a struct, but
    /// a struct containing a reference.
    #[must_use]
    pub fn as_mut(&mut self) -> PlayerSettingsMut<'_> {
        PlayerSettingsMut { map: &mut self.map }
    }
}

impl Clone for PlayerSettings {
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.map.clone_from(&source.map);
    }
}

impl TryFrom<serde_json::Value> for PlayerSettings {
    type Error = serde_json::Value;

    fn try_from(value: serde_json::Value) -> Result<Self, serde_json::Value> {
        let serde_json::Value::Object(map) = value else {
            return Err(value);
        };

        Ok(Self { map })
    }
}

/// A struct pointing to the settings of the player who made the replay.
#[derive(Clone, Copy, Debug, PartialEq, From, Into)]
pub struct PlayerSettingsRef<'a> {
    /// The inner map that stores the setting entries.
    pub map: &'a serde_json::Map<String, serde_json::Value>,
}

impl PlayerSettingsRef<'_> {
    /// Converts this reference struct into an owned version of the
    /// `PlayerSettings` struct.
    ///
    /// This isn't part of the [`ToOwned`][alloc::borrow::ToOwned] impl since
    /// this struct itself is a reference to a Map and not to a `PlayerSettings`
    /// struct, and therefore we can't provide an `Owned` type of `PlayerSettings`
    /// because `PlayerSettings` does not implement `Borrow<PlayerSettingsMut>`.
    #[must_use]
    pub fn to_owned(&self) -> PlayerSettings {
        let map = self.map.clone();
        PlayerSettings { map }
    }
}

impl<'a> TryFrom<&'a serde_json::Value> for PlayerSettingsRef<'a> {
    type Error = TypeError;

    fn try_from(value: &'a serde_json::Value) -> Result<Self, Self::Error> {
        let serde_json::Value::Object(map) = value else {
            return Err(TypeError);
        };

        Ok(Self { map })
    }
}

/// A struct mutably pointing to the settings of the player who made the replay.
#[derive(Debug, PartialEq, From, Into)]
pub struct PlayerSettingsMut<'a> {
    /// The inner map that stores the setting entries.
    pub map: &'a mut serde_json::Map<String, serde_json::Value>,
}

impl PlayerSettingsMut<'_> {
    /// Gets an immutable-reference–based [`PlayerSettingsRef`] struct based on
    /// this [`PlayerSettingsMut`] struct.
    #[must_use]
    pub fn as_immutable(&self) -> PlayerSettingsRef<'_> {
        PlayerSettingsRef { map: self.map }
    }

    /// Converts this reference struct into an owned version of the
    /// `PlayerSettings` struct.
    ///
    /// This isn't part of the [`ToOwned`][alloc::borrow::ToOwned] impl since
    /// this struct itself is a reference to a Map and not to a `PlayerSettings`
    /// struct, and therefore we can't provide an `Owned` type of `PlayerSettings`
    /// because `PlayerSettings` does not implement `Borrow<PlayerSettingsMut>`.
    #[must_use]
    pub fn to_owned(&self) -> PlayerSettings {
        let map = self.map.clone();
        PlayerSettings { map }
    }
}

setting_getters_setters! {
    {
        owned_struct: PlayerSettings => map,
        ref_struct: PlayerSettingsRef<'_> => map,
        mut_ref_struct: PlayerSettingsMut<'_> => map,
    };
    /// The attack FX slider in the video settings.
    ///
    /// Normal values: integer from 0 to 5
    "atkFX" atk_fx: u8 where { from_json: json_to_u8 },

    /// The clear FX slider in the video settings.
    ///
    /// Normal values: integer from 0 to 5
    "clearFX" clear_fx: u8 where { from_json: json_to_u8 },

    /// The drop FX slider in the video settings.
    ///
    /// Normal values: integer from 0 to 5
    "dropFX" drop_fx: u8 where { from_json: json_to_u8 },

    /// The lock FX slider in the video settings.
    ///
    /// Normal values: integer from 0 to 5
    "lockFX" lock_fx: u8 where { from_json: json_to_u8 },

    /// The move FX slider in the video settings.
    ///
    /// Normal values: integer from 0 to 5
    "moveFX" move_fx: u8 where { from_json: json_to_u8 },

    /// The field sway slider in the video settings.
    ///
    /// Normal values: integer from 0 to 5
    "shakeFX" shake_fx: u8 where { from_json: json_to_u8 },

    /// The splash FX slider in the video settings.
    ///
    /// Normal values: integer from 0 to 5
    "splashFX" splash_fx: u8 where { from_json: json_to_u8 },

    /// The DAS (delayed auto-shift) slider in the control settings.
    ///
    /// Normal values: integer from 0 to 20, measured in frames\
    /// Learn more about DAS and ARR: <https://tetris.wiki/DAS>
    "das" das: u8 where { from_json: json_to_u8 },

    /// The ARR (auto-repeat rate) slider in the control settings.
    ///
    /// Normal values: integer from 0 to 15, measured in frames\
    /// Learn more about DAS and ARR: <https://tetris.wiki/DAS>
    "arr" arr: u8 where { from_json: json_to_u8 },

    /// The soft-drop DAS (delayed auto-shift) slider in the control settings.
    ///
    /// Normal values: integer from 0 to 10, measured in frames\
    /// Learn more about DAS and ARR: <https://tetris.wiki/DAS>
    "sddas" sddas: u8 where { from_json: json_to_u8 },

    /// The soft-drop ARR (auto-repeat rate) slider in the control settings.
    ///
    /// Normal values: integer from 0 to 4, measured in frames\
    /// Learn more about DAS and ARR: <https://tetris.wiki/DAS>
    "sdarr" sdarr: u8 where { from_json: json_to_u8 },

    /// The DAS (delayed auto-shift) cut slider in the control settings.
    ///
    /// Normal values: integer from 0 to 20, measured in frames\
    /// Learn more about DAS: <https://tetris.wiki/DAS>
    "dascut" dascut: u8 where { from_json: json_to_u8 },

    /// The IRS (initial rotation system) cut slider in the control settings.
    ///
    /// Normal values: integer from 0 to 20, measured in frames\
    /// Learn more about IRS: <https://tetris.wiki/IRS>\
    /// Version info: This is only available on game versions >=0.17.22
    "irscut" irscut: u8 where { from_json: json_to_u8 },

    /// The auto-lock cut slider in the control settings.
    ///
    /// Normal values: integer from 0 to 10, measured in frames
    "dropcut" dropcut: u8 where { from_json: json_to_u8 },

    /// The IRS (initial rotation system) checkbox in the control settings.
    ///
    /// Learn more about IRS: <https://tetris.wiki/IRS>
    "irs" irs: bool where { from_json: serde_json::Value::as_bool },

    /// The IHS (initial hold system) checkbox in the control settings.
    ///
    /// Learn more about IHS: <https://tetris.wiki/IHS>
    "ihs" ihs: bool where { from_json: serde_json::Value::as_bool },

    /// The IMS (initial movement system) checkbox in the control settings.
    ///
    /// Analogous to [IRS][<https://tetris.wiki/IRS>] and [IHS][<https://tetris.wiki/IHS>],
    /// but for movement instead of rotating and holding, respectively.
    "ims" ims: bool where { from_json: serde_json::Value::as_bool },

    /// The rotation system used in the replay.
    ///
    /// Normal values (as of January 2025):
    /// - `TRS`
    /// - [`SRS`][<https://tetris.wiki/SRS>]
    /// - `SRS_plus`
    /// - `SRS_X`
    /// - `BiRS`
    /// - [`ARS_Z`][<https://tetris.wiki/ARS>]
    /// - [`DRS_weak`][<https://tetris.wiki/DTET_Rotation_System>]
    /// - [`ASC`][<https://tetris.wiki/Ascension>]
    /// - `ASC_plus`
    /// - [`C2`][<https://tetris.wiki/Cultris_II>]
    /// - `C2_sym`
    /// - [`N64`][<https://tetris.wiki/The_New_Tetris>]
    /// - `N64_plus`
    /// - [`Classic`][<https://tetris.wiki/Nintendo_Rotation_System>]
    /// - `Classic_plus`
    /// - `None`
    /// - `None_plus`
    "RS" rs: &str | String where { from_json: serde_json::Value::as_str },

    /// The bag separator option in the video settings.
    "bagLine" bag_line: bool where { from_json: serde_json::Value::as_bool },

    /// The "draw active piece" option in the video settings.
    "block" block: bool where { from_json: serde_json::Value::as_bool },

    /// The rotation center opacity option in the video settings.
    "center" center: f64 where { from_json: serde_json::Value::as_f64 },

    // TODO: Figure out the order of the specific elements
    /// The starting orientations of all the pieces.
    ///
    /// Normally contains 29 elements: 7 tetrominoes, 18 pentominoes, 2 trominoes, 1 domino, and 1 monomino, in that order.
    "face" face: [u8; TOTAL_PIECE_COUNT] where { from_json: json_to_piece_bytes },

    /// The ghost piece opacity option in the video settings.
    "ghost" ghost: f64 where { from_json: serde_json::Value::as_f64 },

    /// The grid opacity option in the video settings.
    "grid" grid: f64 where { from_json: serde_json::Value::as_f64 },

    /// The screen scrolling option in the video settings.
    "highCam" high_cam: bool where { from_json: serde_json::Value::as_bool },

    /// The spawn preview option in the video settings.
    "nextPos" next_pos: bool where { from_json: serde_json::Value::as_bool },

    /// The "score pop-ups" option in the video settings.
    "score" score: bool where { from_json: serde_json::Value::as_bool },

    // TODO: Figure out the order of the specific elements
    /// The colors of all the pieces.
    ///
    /// Normally contains 29 elements: 7 tetrominoes, 18 pentominoes, 2 trominoes, 1 domino, and 1 monomino, in that order.
    "skin" skin: [u8; TOTAL_PIECE_COUNT] where { from_json: json_to_piece_bytes },

    /// The smooth falling option option in the video settings.
    "smooth" smooth: bool where { from_json: serde_json::Value::as_bool },

    /// The line clear popups option in the video settings.
    "text" text: bool where { from_json: serde_json::Value::as_bool },

    /// The danger alerts option in the video settings.
    "warn" warn: bool where { from_json: serde_json::Value::as_bool },

    /// The "Frame skip" option in the video settings.
    ///
    /// This option was removed in version 0.17.2 of the game.
    "FTLock" ft_lock: bool where { from_json: serde_json::Value::as_bool },
}

/// A struct representing the metadata stored within the replay.
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
    /// Returns the old value of the field.
    /// - If there was no old value of the field, returns `None`.
    /// - If conversion succeeds, returns the strictly typed version
    ///   (`Some(Ok(T))`).
    /// - Otherwise, returns the raw JSON value.
    ///   (`Some(Err(serde_json::Value)))`)
    pub fn set_private(&mut self, value: Option<serde_json::Value>) -> Option<serde_json::Value> {
        let Some(value) = value else {
            return self.map.remove(Self::KEY_PRIVATE);
        };

        if let Some(mutref) = self.map.get_mut(Self::KEY_PRIVATE) {
            return Some(core::mem::replace(mutref, value));
        }
        self.map.insert(Self::KEY_PRIVATE.to_owned(), value)
    }

    /// The settings of the game when the run was played.
    #[must_use]
    pub fn get_settings(&self) -> Option<Result<PlayerSettingsRef<'_>, TypeError>> {
        let entry = self.map.get(Self::KEY_SETTINGS)?;

        let serde_json::Value::Object(map) = entry else {
            return Some(Err(TypeError));
        };

        Some(Ok(PlayerSettingsRef { map }))
    }

    /// The settings of the game when the run was played.
    #[must_use]
    pub fn get_settings_mut(&mut self) -> Option<Result<PlayerSettingsMut<'_>, TypeError>> {
        let entry = self.map.get_mut(Self::KEY_SETTINGS)?;

        let serde_json::Value::Object(map) = entry else {
            return Some(Err(TypeError));
        };

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
    /// - Otherwise, returns the raw JSON value.
    ///   (`Some(Err(serde_json::Value)))`)
    pub fn set_settings(
        &mut self,
        value: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Option<Result<PlayerSettings, serde_json::Value>> {
        let Some(value) = value else {
            let json = self.map.remove(Self::KEY_SETTINGS)?;
            return match PlayerSettings::try_from(json) {
                Ok(ps) => Some(Ok(ps)),
                Err(json) => Some(Err(json)),
            };
        };

        if let Some(dest) = self.map.get_mut(Self::KEY_SETTINGS) {
            if let serde_json::Value::Object(dest) = dest {
                let old = core::mem::replace(dest, value);
                Some(Ok(PlayerSettings { map: old }))
            } else {
                let src = serde_json::Value::Object(value);
                let old = core::mem::replace(dest, src);
                Some(Err(old))
            }
        } else {
            self.map.insert(
                Self::KEY_SETTINGS.to_owned(),
                serde_json::Value::Object(value),
            );
            None
        }
    }

    metadata_getters_setters! {
        (map);
        /// Whether or not the replay is marked as a TAS.
        "tasUsed" tas_used: bool where { from_json: serde_json::Value::as_bool },

        /// The username of the player.
        "player" player: &str | String where { from_json: serde_json::Value::as_str },

        /// The seed for the random number generator.
        "seed" seed: u64 where { from_json: serde_json::Value::as_u64 },

        /// The version of the game the replay was made in.
        ///
        /// Usually conforms to semver (major.minor.patch), but some mods
        /// may use a different or custom format.
        "version" version: &str | String where { from_json: serde_json::Value::as_str },

        /// The date and time the replay was initially created.
        "date" date: &str | String where { from_json: serde_json::Value::as_str },

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
        "mode" mode: &str | String where { from_json: serde_json::Value::as_str },

        // "setting" settings: PlayerSettings where { from_json: serde_json::Value::as_object },
    }
}

/// An error from parsing the replay data.
#[derive(Debug, Error)]
pub enum ReplayParseError {
    /// An error occurred when zlib tried to decompress the replay data.
    ///
    /// See [`DecompressError`] for more information.
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

    /// The metadata could not be deserialized into the [`GameReplayMetadata`] struct,
    /// possibly due to missing values.
    ///
    /// See [`serde_json`'s Error type][serde_json::Error] for more information.
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
        /// the same as `frame` if [`InputParseMode`] is
        /// [`Absolute`][InputParseMode::Absolute].
        raw_frame: u64,
        /// The "frame"/time value of the associated input event.
        ///
        /// This is the same as `raw_frame` if [`InputParseMode`] is
        /// [`Absolute`][InputParseMode::Absolute].
        frame: u64,
        /// The action of the associated input event.
        ///
        /// See [`InputAction`] for more details.
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
pub enum ReplaySerializeError {
    /// The mode in which to serialize the inputs could not be inferred from the version string.
    ///
    /// Contains the [`String`] or the [`serde_json::Value`] of the version
    /// entry if it was found.
    ///
    /// To fix this error, consider passing in the input parse mode explicitly.
    #[error("could not infer input parse mode from version metadata")]
    UnknownInputParseMode(Option<Result<String, serde_json::Value>>),

    /// There was an attempt to call a function at the wrong state.
    ///
    /// For example, if the replay encoder expects metadata, but input data
    /// was given instead (or vice versa), this error will be returned.
    #[error("attempted to call a function at the wrong state")]
    InvalidOperation,

    /// The input [`Vec`] isn't sorted.
    ///
    /// The serializer expects the input [`Vec`] to be sorted, or the game may parse the inputs
    /// in a strange way.
    ///
    /// To fix this error, consider calling [`sort_inputs`][GameReplayData::sort_inputs] on the
    /// [`GameReplayData`] before serializing it.
    #[error("unsorted input data: found input for frame {unsorted_time} after input for frame {prev_time}")]
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

/// Determines how to parse the inputs of the replay.
///
/// Replays made before version 0.17.22 of the game (i.e., 0.17.21 and before it)
/// use relative timing for its inputs.\
/// However, starting from version 0.17.22 of the game, absolute timing is used.
#[cfg_attr(feature = "arbitrary", derive(Arbitrary))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputParseMode {
    /// Relative timing.
    ///
    /// Replays made before version 0.17.22 of the game (i.e., 0.17.21 and before it)
    /// use relative timing for its inputs. That is, the time in each key-time
    /// pair are relative to the frame of the previous input.
    ///
    /// For example, if you press two keys at the exact same frame, the first input
    /// has a stored time of the number of frames since the previous input,
    /// while the second input has a time of 0.
    Relative,
    /// Absolute timing.
    ///
    /// Replays made after version 0.17.21 of the game (i.e., 0.17.22 and onwards)
    /// use absolute timing for its inputs. That is, the time in each key-time
    /// pair are relative to the beginning of the replay (i.e., frame zero).
    ///
    /// For example, if you press two keys at the exact same frame, the first input
    /// has a time of the current frame number, as well as the second input.
    Absolute,
}

impl InputParseMode {
    /// The first version where absolute timing is used.
    pub const ABSOLUTE_TIMING_START: Version = Version::new(0, 17, 22);

    /// Tries to infer the input parse mode based on the game version.
    ///
    /// If parsing the version fails, it will return `None`.
    #[must_use]
    pub fn try_infer_from_version(version: &str) -> Option<InputParseMode> {
        let lower = version.to_ascii_lowercase();
        let lower = lower
            .trim_start_matches('v')
            .trim_start_matches("alpha")
            .trim_start();

        if lower.contains("wtf") {
            // Matches Techmino WTF mod from April 2024
            // https://github.com/MelloBoo44/Techmino-WTF
            return Some(InputParseMode::Relative);
        }

        if lower.trim_start().starts_with("unofficial expansion") {
            // Matches Techmino Unofficial Expansion mod from August 2023
            // https://github.com/Another-Soul/Techmino-Unofficial-Expansion
            return Some(InputParseMode::Relative);
        }

        // Snapshots use @ as version@commit delimiter
        let lower = match lower.find('@') {
            Some(idx) => &lower[..idx],
            None => lower,
        };

        // Electra's mods have multiple elements to them
        let lower = lower.split(' ').next().unwrap_or_default();

        let filtered_version: String = lower
            .chars()
            .filter(|c| c.is_numeric() || *c == '.')
            .collect();

        let version = Version::parse(&filtered_version);

        if let Ok(v) = version {
            if v < Self::ABSOLUTE_TIMING_START {
                return Some(InputParseMode::Relative);
            }
            return Some(InputParseMode::Absolute);
        }

        None
    }

    /// Tries to infer the input parse mode based on the input slice.
    ///
    /// Returns [`None`] if the input parse mode could not be inferred.
    #[must_use]
    pub fn try_infer_from_input_data(input_slice: &[u64]) -> Option<InputParseMode> {
        // Absolute mode: expects increasing frame times
        let mut prev_time = 0;
        for &time in input_slice.iter().step_by(2) {
            if time < prev_time {
                // Definitely not absolute!
                return Some(InputParseMode::Relative);
            }

            prev_time = time;
        }

        // It's not really possible to "disprove" relative mode, so we're still unsure
        None
    }
}

fn json_to_u8(value: &serde_json::Value) -> Option<u8> {
    value.as_number()?.as_u64()?.try_into().ok()
}

/// Attempts to convert a JSON value into a byte array for every piece in the game.
fn json_to_piece_bytes(value: &serde_json::Value) -> Option<[u8; TOTAL_PIECE_COUNT]> {
    let arr: &[_; TOTAL_PIECE_COUNT] = value.as_array()?.as_array()?;

    let mut bytes = [0u8; TOTAL_PIECE_COUNT];

    for i in 0..TOTAL_PIECE_COUNT {
        bytes[i] = json_to_u8(&arr[i])?;
    }

    Some(bytes)
}

/// Attempts to convert a JSON value into a mod list type.
fn json_to_modlist(value: &serde_json::Value) -> Option<Vec<(u64, serde_json::Value)>> {
    let source = value.as_array()?;

    let mut processed_list = Vec::with_capacity(source.len());

    for entry in source {
        let entry = entry.as_array()?;
        let [mod_id, mod_value] = entry.as_array()?;
        let mod_id = mod_id.as_u64()?;
        let mod_value = mod_value.clone();

        processed_list.push((mod_id, mod_value));
    }

    Some(processed_list)
}

/// Converts a modlist into JSON format.
fn modlist_to_json(modlist: Vec<(u64, serde_json::Value)>) -> serde_json::Value {
    let values: Vec<serde_json::Value> = modlist
        .into_iter()
        .map(|(mod_id, mod_value)| {
            serde_json::Value::Array(vec![
                serde_json::Value::Number(serde_json::Number::from(mod_id)),
                mod_value,
            ])
        })
        .collect();

    serde_json::Value::Array(values)
}

#[cfg(test)]
mod tests {
    use fastrand::Rng;
    use strum::IntoEnumIterator;

    use super::*;

    #[test]
    fn test_inferred_mode() {
        use InputParseMode::*;
        let cases = [
            ("Techmino is fun!", None),
            ("Alpha v0.15.1", Some(Relative)),
            ("V0.16.2", Some(Relative)),
            ("0.17.22", Some(Absolute)),
            ("v0.17.6@26fc", Some(Relative)),
            ("v 1.2.3", Some(Absolute)),
            // https://github.com/MelloBoo44/Techmino-WTF/blob/main/version.lua
            ("WTF", Some(Relative)),
            // https://github.com/Another-Soul/Techmino-Unofficial-Expansion/blob/main/version.lua
            ("Unofficial Expansion v0.2.1", Some(Relative)),
            // https://github.com/electraminer/Techmino/blob/king_of_stackers/version.lua
            (
                "V0.17.22 IRSv1.1 PASSTHROUGHFIXv1.0 KOSv1.2beta TE:Cv1.0",
                Some(Absolute),
            ),
            // https://github.com/electraminer/Techmino/blob/irs/version.lua
            ("V0.17.22 + IRSv1.1.1", Some(Absolute)),
            // https://github.com/electraminer/Techmino/blob/king_of_cheesers/version.lua
            (
                "V0.17.22 IRSv1.1 PASSTHROUGHFIXv1.0 KOCv0.1beta TE:Cv1.0",
                Some(Absolute),
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(InputParseMode::try_infer_from_version(input), expected);
        }
    }

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
