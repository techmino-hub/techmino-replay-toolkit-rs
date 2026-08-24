//! Represents stuff found in game settings.

use crate::{
    consts::TOTAL_PIECE_COUNT,
    convert::{
        json_to_bool, json_to_f64, json_to_piece_bytes, json_to_piece_colors, json_to_str,
        json_to_u8, piece_colors_to_json,
    },
    errors::{TypeError, ValueVariant},
    macros::setting_getters_setters,
    replay::GameReplayMetadata,
};
use derive_more::{From, Into};
use serde::{Deserialize, Serialize};

/// A struct representing the settings of the player who made the replay.
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
    /// This is different than an [`AsRef`] implementation
    /// since this function doesn't directly return a reference to a struct, but
    /// a struct containing a reference.
    #[must_use]
    pub fn as_ref(&self) -> PlayerSettingsRef<'_> {
        PlayerSettingsRef { map: &self.map }
    }

    /// Creates a new [`PlayerSettingsMut`] struct pointing to this
    /// struct's owned map.
    ///
    /// This is different than an [`AsRef`] implementation
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
    /// This isn't part of the [`ToOwned`] impl since
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
    type Error = TypeError<'a>;

    fn try_from(value: &'a serde_json::Value) -> Result<Self, Self::Error> {
        const EXPECTED_TYPE: ValueVariant = ValueVariant::Object;

        let variant = ValueVariant::from(value);

        let serde_json::Value::Object(map) = value else {
            debug_assert_ne!(
                variant, EXPECTED_TYPE,
                "Invariant breached: expected type shouldn't match (this is a `libtechmino-replay` bug, please report there)"
            );

            return Err(TypeError {
                key: GameReplayMetadata::KEY_SETTINGS,
                exp_ty: EXPECTED_TYPE,
                value,
            });
        };

        debug_assert_eq!(
            ValueVariant::from(value),
            EXPECTED_TYPE,
            "Invariant breached: expected type should match (this is a `libtechmino-replay` bug, please report there)"
        );

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
    /// This isn't part of the [`ToOwned`] impl since
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
    "irs" irs: bool where { from_json: json_to_bool },

    /// The IHS (initial hold system) checkbox in the control settings.
    ///
    /// Learn more about IHS: <https://tetris.wiki/IHS>
    "ihs" ihs: bool where { from_json: json_to_bool },

    /// The IMS (initial movement system) checkbox in the control settings.
    ///
    /// Analogous to [IRS](<https://tetris.wiki/IRS>) and [IHS](<https://tetris.wiki/IHS>),
    /// but for movement instead of rotating and holding, respectively.
    "ims" ims: bool where { from_json: json_to_bool },

    /// The rotation system used in the replay.
    ///
    /// Normal values (as of January 2025):
    /// - `TRS`
    /// - [`SRS`](<https://tetris.wiki/SRS>)
    /// - `SRS_plus`
    /// - `SRS_X`
    /// - `BiRS`
    /// - [`ARS_Z`](<https://tetris.wiki/ARS>)
    /// - [`DRS_weak`](<https://tetris.wiki/DTET_Rotation_System>)
    /// - [`ASC`](<https://tetris.wiki/Ascension>)
    /// - `ASC_plus`
    /// - [`C2`](<https://tetris.wiki/Cultris_II>)
    /// - `C2_sym`
    /// - [`N64`](<https://tetris.wiki/The_New_Tetris>)
    /// - `N64_plus`
    /// - [`Classic`](<https://tetris.wiki/Nintendo_Rotation_System>)
    /// - `Classic_plus`
    /// - `None`
    /// - `None_plus`
    "RS" rs: &str | String where { from_json: json_to_str },

    /// The bag separator option in the video settings.
    "bagLine" bag_line: bool where { from_json: json_to_bool },

    /// The "draw active piece" option in the video settings.
    "block" block: bool where { from_json: json_to_bool },

    /// The rotation center opacity option in the video settings.
    "center" center: f64 where { from_json: json_to_f64 },

    /// The starting orientations of all the pieces.
    ///
    /// # Piece-Specific Information
    /// Use the [`Piece`][Piece] enum to help index into the given array.
    ///
    /// ```
    /// use libtechmino_replay::consts::{TOTAL_PIECE_COUNT, Piece};
    ///
    /// let faces = [0u8; TOTAL_PIECE_COUNT];
    ///
    /// let t_face = faces[Piece::T.get_index()];
    /// ```
    ///
    /// [Piece]: crate::consts::Piece
    "face" face: [u8; TOTAL_PIECE_COUNT] where { from_json: json_to_piece_bytes },

    /// The ghost piece opacity option in the video settings.
    "ghost" ghost: f64 where { from_json: json_to_f64 },

    /// The grid opacity option in the video settings.
    "grid" grid: f64 where { from_json: json_to_f64 },

    /// The screen scrolling option in the video settings.
    "highCam" high_cam: bool where { from_json: json_to_bool },

    /// The spawn preview option in the video settings.
    "nextPos" next_pos: bool where { from_json: json_to_bool },

    /// The "score pop-ups" option in the video settings.
    "score" score: bool where { from_json: json_to_bool },

    /// The colors of all the pieces, represented as `u8`.
    ///
    /// For the getter/setter that accepts/returns [`PieceColor`]s instead of
    /// `u8`s, see `*_skin_enum()` (e.g. `.get_skin_enum()`).
    ///
    /// # Piece-Specific Information
    /// Use the [`Piece`][Piece] enum to help index into the given array.
    ///
    /// ```
    /// use libtechmino_replay::consts::{TOTAL_PIECE_COUNT, Piece};
    ///
    /// let skins = [0u8; TOTAL_PIECE_COUNT];
    ///
    /// let t_skin = skins[Piece::T.get_index()];
    /// ```
    ///
    /// [Piece]: crate::consts::Piece
    "skin" skin: [u8; TOTAL_PIECE_COUNT] where { from_json: json_to_piece_bytes },

    /// The colors of all the pieces, represented as [`PieceColor`].
    ///
    /// For the getter/setter that accepts/returns `u8`s instead of
    /// [`PieceColor`]s, see `*_skin()` (e.g. `.get_skin()`).
    ///
    /// # Piece-Specific Information
    /// Use the [`Piece`][Piece] enum to help index into the given array.
    ///
    /// ```
    /// use libtechmino_replay::{
    ///     consts::{TOTAL_PIECE_COUNT, Piece},
    ///     replay::PieceColor,
    /// };
    ///
    /// let skins = [PieceColor::Purple; TOTAL_PIECE_COUNT];
    ///
    /// let t_skin = skins[Piece::T.get_index()];
    /// ```
    ///
    /// [Piece]: crate::consts::Piece
    "skin" skin_enum: [PieceColor; TOTAL_PIECE_COUNT] where {
        from_json: json_to_piece_colors,
        to_json: piece_colors_to_json,
    },

    /// The smooth falling option option in the video settings.
    "smooth" smooth: bool where { from_json: json_to_bool },

    /// The line clear popups option in the video settings.
    "text" text: bool where { from_json: json_to_bool },

    /// The danger alerts option in the video settings.
    "warn" warn: bool where { from_json: json_to_bool },

    /// The "Frame skip" option in the video settings.
    ///
    /// This option was removed in version 0.17.2 of the game.
    "FTLock" ft_lock: bool where { from_json: json_to_bool },
}

/// Represents the skin color of a piece in the game.
///
/// The discriminants are the numbers used by the game to identify each color
/// in the replay metadata.
///
/// The approximate hue may differ between piece skin sets. The given numbers
/// are loosely derived from the `pure` skin by `MrZ`.
#[repr(u8)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "strum",
    derive(strum::EnumIter, strum::IntoStaticStr, strum::VariantArray)
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PieceColor {
    /// Red, the R in RGB.
    ///
    /// Hue ≈ 0°
    Red = 1,
    /// A warm color in between red and orange.
    ///
    /// Hue ≈ 25°
    #[doc(alias = "OrangeRed")]
    Fire = 2,
    /// A warm color in between red and yellow.
    ///
    /// Hue ≈ 40°
    Orange = 3,
    /// A warm color in between red and green.
    ///
    /// Hue ≈ 60°
    Yellow = 4,
    /// A mixture of yellow and green, closer to yellow.
    ///
    /// Hue ≈ 80°
    Lime = 5,
    /// A mixture of yellow and green, closer to green.
    ///
    /// Hue ≈ 100°
    Jade = 6,
    /// Green, the G in RGB.
    ///
    /// Hue ≈ 120°
    Green = 7,
    /// A cool color in between green and cyan.
    ///
    /// Hue ≈ 150°
    Aqua = 8,
    /// A cool color in between green and blue.
    ///
    /// Hue ≈ 180°
    Cyan = 9,
    /// A cool color in between cyan and blue, closer to cyan.
    ///
    /// Hue ≈ 200°
    Navy = 10,
    /// A cool color in between cyan and blue, closer to blue.
    ///
    /// Hue ≈ 220°
    Sea = 11,
    /// Blue, the B in RGB.
    ///
    /// Hue ≈ 240°
    Blue = 12,
    /// A cool color in between blue and purple.
    ///
    /// Hue ≈ 260°
    Violet = 13,
    /// A cool color, and a mix of blue and red, closer to blue.
    ///
    /// Hue ≈ 280°
    Purple = 14,
    /// A vibrant color which is a mix of blue and red.
    ///
    /// Hue ≈ 310°
    Magenta = 15,
    /// A vibrant color which is a mix of blue and red, closer to red.
    ///
    /// Hue ≈ 340°
    Wine = 16,
    /// The bone skin variant, commonly seen in high-level 20G runs.
    ///
    /// No common color.
    Bone = 17,
    /// The invisible skin variant.
    #[doc(alias = "None")]
    Invisible = 18,
    /// The bomb skin variant, with special behaviour.
    ///
    /// Usually only seen in the Custom game mode, where
    /// if a piece is placed on top of a mino in this color, the entire line
    /// containing this mino is removed.
    Bomb = 19,
    /// The dark grey (dark gray) skin color.
    ///
    /// Usually seen in the garbage lines that appear in bot, online, attacker,
    /// defender, driller, and backfire modes.
    #[doc(alias = "DarkGray")]
    DarkGrey = 20,
    /// The light grey (light gray) skin color.
    ///
    /// (Light as in the color of the garbage, not the amount.)
    ///
    /// Usually seen in the garbage lines that appear in bot, online, attacker,
    /// defender, driller, and backfire modes.
    #[doc(alias = "LightGray")]
    LightGrey = 21,
    /// The light violet skin color.
    ///
    /// Usually seen in the garbage lines that appear in bot, online, attacker,
    /// defender, driller, and backfire modes.
    ///
    /// Hue ≈ 245°
    LightViolet = 22,
    /// The light magenta skin color.
    ///
    /// Usually seen in the garbage lines that appear in bot, online, attacker,
    /// defender, driller, and backfire modes.
    ///
    /// Hue ≈ 315°
    LightMagenta = 23,
    /// The light green skin color.
    ///
    /// Usually seen in the garbage lines that appear in bot, online, attacker,
    /// defender, driller, and backfire modes.
    ///
    /// Hue ≈ 145°
    LightGreen = 24,
}

impl PieceColor {
    /// Converts this [`PieceColor`] into its `u8` representation.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Tries to convert a `u8` into a [`PieceColor`] if it is valid.
    #[must_use]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Red),
            2 => Some(Self::Fire),
            3 => Some(Self::Orange),
            4 => Some(Self::Yellow),
            5 => Some(Self::Lime),
            6 => Some(Self::Jade),
            7 => Some(Self::Green),
            8 => Some(Self::Aqua),
            9 => Some(Self::Cyan),
            10 => Some(Self::Navy),
            11 => Some(Self::Sea),
            12 => Some(Self::Blue),
            13 => Some(Self::Violet),
            14 => Some(Self::Purple),
            15 => Some(Self::Magenta),
            16 => Some(Self::Wine),
            17 => Some(Self::Bone),
            18 => Some(Self::Invisible),
            19 => Some(Self::Bomb),
            20 => Some(Self::DarkGrey),
            21 => Some(Self::LightGrey),
            22 => Some(Self::LightViolet),
            23 => Some(Self::LightMagenta),
            24 => Some(Self::LightGreen),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn piece_color_enum_u8_roundtrip() {
        for i in 0..u8::MAX {
            let Some(color) = PieceColor::try_from_u8(i) else {
                continue;
            };

            assert_eq!(color.as_u8(), i);
        }
    }
}
