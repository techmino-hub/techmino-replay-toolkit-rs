use std::{collections::HashMap, string::FromUtf8Error};

use base64::DecodeError;
use miniz_oxide::inflate::DecompressError;
use semver::Version;
use serde::{Deserialize, Serialize};

/// Represents the type of input event this is.  
/// That is, whether or not this is a button press event, or a button release event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InputEventKind {
    /// A certain button is being pressed in the event.
    Press = 0,
    /// A certain button is being released in the event.
    Release = 1,
}

impl TryFrom<u8> for InputEventKind {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Press),
            1 => Ok(Self::Release),
            _ => Err(()),
        }
    }
}

impl From<bool> for InputEventKind {
    fn from(value: bool) -> Self {
        match value {
            false => Self::Press,
            true => Self::Release,
        }
    }
}

impl From<InputEventKind> for u8 {
    fn from(value: InputEventKind) -> Self {
        match value {
            InputEventKind::Press => 0,
            InputEventKind::Release => 1,
        }
    }
}

impl From<InputEventKind> for bool {
    fn from(value: InputEventKind) -> Self {
        match value {
            InputEventKind::Press => false,
            InputEventKind::Release => true,
        }
    }
}

/// Represents the key/button of the input event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum InputEventKey {
    MoveLeft = 1,
    MoveRight = 2,
    RotateRight = 3,
    RotateLeft = 4,
    Rotate180 = 5,
    HardDrop = 6,
    SoftDrop = 7,
    Hold = 8,

    Function1 = 9,
    Function2 = 10,

    InstantLeft = 11,
    InstantRight = 12,
    SonicDrop = 13,
    Down1 = 14,
    Down4 = 15,
    Down10 = 16,
    LeftDrop = 17,
    RightDrop = 18,
    LeftZangi = 19,
    RightZangi = 20,
}

impl TryFrom<u8> for InputEventKey {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use InputEventKey::*;

        match value {
            1 => Ok(MoveLeft),
            2 => Ok(MoveRight),
            3 => Ok(RotateRight),
            4 => Ok(RotateLeft),
            5 => Ok(Rotate180),
            6 => Ok(HardDrop),
            7 => Ok(SoftDrop),
            8 => Ok(Hold),
            9 => Ok(Function1),
            10 => Ok(Function2),
            11 => Ok(InstantLeft),
            12 => Ok(InstantRight),
            13 => Ok(SonicDrop),
            14 => Ok(Down1),
            15 => Ok(Down4),
            16 => Ok(Down10),
            17 => Ok(LeftDrop),
            18 => Ok(RightDrop),
            19 => Ok(LeftZangi),
            20 => Ok(RightZangi),
            _ => Err(()),
        }
    }
}

impl From<InputEventKey> for u8 {
    fn from(value: InputEventKey) -> Self {
        use InputEventKey::*;

        match value {
            MoveLeft => 1,
            MoveRight => 2,
            RotateRight => 3,
            RotateLeft => 4,
            Rotate180 => 5,
            HardDrop => 6,
            SoftDrop => 7,
            Hold => 8,
            Function1 => 9,
            Function2 => 10,
            InstantLeft => 11,
            InstantRight => 12,
            SonicDrop => 13,
            Down1 => 14,
            Down4 => 15,
            Down10 => 16,
            LeftDrop => 17,
            RightDrop => 18,
            LeftZangi => 19,
            RightZangi => 20,
        }
    }
}

/// A struct representing a single input event in the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GameInputEvent {
    /// A number representing the frame this event occurred in.
    /// 
    /// Note that the game starts at frame 180, and the frames before that
    /// happen during the game start countdown. Nevertheless,
    /// the game still records inputs before the countdown finishes.
    pub frame: u64,
    /// The kind of input event this represents.  
    /// That is - whether or not this is a key press event or a key release event.
    pub kind: InputEventKind,
    /// The key that is being pressed or released.
    /// 
    /// See [`InputEventKey`] for more details.
    pub key: InputEventKey,
}

/// A struct representing all the data contained within the game replay.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct GameReplayData {
    /// A list of game input events that happened during the replay.
    pub inputs: Vec<GameInputEvent>,
    /// Metadata contained within the replay data.
    pub metadata: GameReplayMetadata,
}

/// A struct representing the settings of the player who made the replay.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSettings {
    #[serde(rename = "atkFX")]
    pub atk_fx: Option<u64>,
    #[serde(rename = "clearFX")]
    pub clear_fx: Option<u64>,
    #[serde(rename = "dropFX")]
    pub drop_fx: Option<u64>,
    #[serde(rename = "lockFX")]
    pub lock_fx: Option<u64>,
    #[serde(rename = "moveFX")]
    pub move_fx: Option<u64>,
    #[serde(rename = "shakeFX")]
    pub shake_fx: Option<u64>,
    #[serde(rename = "splashFX")]
    pub splash_fx: Option<u64>,

    pub das: Option<u64>,
    pub arr: Option<u64>,
    pub sddas: Option<u64>,
    pub sdarr: Option<u64>,
    pub dascut: Option<u64>,
    pub dropcut: Option<u64>,

    pub irs: Option<bool>,
    pub ihs: Option<bool>,
    pub ims: Option<bool>,
    #[serde(rename = "RS")]
    pub rs: Option<String>,

    pub bag_line: Option<bool>,
    pub block: Option<bool>,
    pub center: Option<u64>,
    pub face: Option<Vec<u64>>,
    pub ghost: Option<f64>,
    pub grid: Option<f64>,
    pub high_cam: Option<bool>,
    pub next_pos: Option<bool>,
    pub score: Option<bool>,
    pub skin: Option<Vec<u64>>,
    pub smooth: Option<bool>,
    pub swap: Option<bool>,
    pub text: Option<bool>,
    pub warn: Option<bool>,
    
    /// Additional settings that may not be standard.
    #[serde(flatten)]
    pub nonstandard: HashMap<String, serde_json::Value>,
}

/// A struct representing the metadata stored within the replay.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GameReplayMetadata {
    /// Whether or not the replay is marked as a TAS.
    pub tas_used: Option<bool>,

    /// The 'private' field of the replay, used to store mode-specific data.  
    /// Its contents differ based on the mode played.  
    /// Currently, only the `custom_clear` and `custom_puzzle` modes are known to
    /// store any data here.
    pub private: Option<serde_json::Value>,

    /// The username of the player.
    pub player: String,

    /// The seed for the random number generator.
    pub seed: u64,

    /// The version of the game the replay was made in.
    ///
    /// Usually conforms to semver (major.minor.patch), but some mods
    /// may use a different or custom format.
    pub version: String,

    /// The date and time the replay was initially created.
    pub date: String,

    /// A list of mods applied to the run.
    ///
    /// It's in the format of [mod, value], where mod is the mod ID and value is the value given to the mod.
    /// 
    /// Note: the original metadata JSON has calls this value `mod`, but since it's misleading (not plural)
    /// and is a special keyword in Rust, this has been renamed to `mods` in the struct.  
    /// This probably means nothing to you, since all the serialization and deserialization will
    /// convert between the two forms automatically.
    #[serde(rename = "mod")]
    pub mods: Option<Vec<(u64, serde_json::Value)>>,

    /// The name of the mode that was played.
    ///
    /// This refers to the internal/codename of the mode, i.e. `sprint_10l` instead of `Sprint 10L`.
    pub mode: String,

    /// The settings of the game when the run was played.
    pub setting: PlayerSettings,

    /// Additional replay metadata, if any, that may not be standard.
    #[serde(flatten)]
    pub nonstandard: HashMap<String, serde_json::Value>,
}

/// An error from parsing the replay data.
#[derive(Debug)]
pub enum ReplayParseError {
    /// An error occurred when zlib tried to decompress the replay data.
    ///
    /// See [DecompressError] for more information.
    ZlibDecompressError(DecompressError),

    /// An error occurred while parsing the base64 string.
    ///
    /// See [DecodeError] for more information.
    Base64DecodeError(DecodeError),

    /// The separator between the replay metadata and the input data was not found.
    ///
    /// The separator is a linefeed character, or a byte with a decimal value of `10`.
    MetadataSeparatorNotFound,

    /// The metadata was found to not be valid UTF-8.
    ///
    /// See [FromUtf8Error] for more information.
    MetadataNotUtf8(FromUtf8Error),

    /// The metadata could not be deserialized into the [GameReplayMetadata] struct,
    /// possibly due to missing values.
    ///
    /// See [`serde_json`'s Error type][serde_json::Error] for more information.
    MetadataDeserializeError(serde_json::Error),

    /// The mode in which to parse the inputs could not be inferred from the version string.
    ///
    /// Contains a [`String`] containing the version string.
    ///
    /// To fix this error, consider passing in the input parse mode explicitly.
    UnknownInputParseMode(String),

    /// The input data was malformed and could not be casted into the proper enum types.
    MalformedInputData {
        /// The first input data index in which the input data is malformed.
        position: u64,
        /// The "frame"/time value of the input data point.
        frame: u64,
        /// The "kind" value of the input data point.
        kind: u64,
    },
}

impl From<DecompressError> for ReplayParseError {
    fn from(value: DecompressError) -> Self {
        ReplayParseError::ZlibDecompressError(value)
    }
}

impl From<DecodeError> for ReplayParseError {
    fn from(value: DecodeError) -> Self {
        ReplayParseError::Base64DecodeError(value)
    }
}

impl From<FromUtf8Error> for ReplayParseError {
    fn from(value: FromUtf8Error) -> Self {
        ReplayParseError::MetadataNotUtf8(value)
    }
}

impl From<serde_json::Error> for ReplayParseError {
    fn from(value: serde_json::Error) -> Self {
        Self::MetadataDeserializeError(value)
    }
}

/// An error from serializing the replay data, e.g. to base64.
#[derive(Debug)]
pub enum ReplaySerializeError {
    /// The mode in which to serialize the inputs could not be inferred from the version string.
    ///
    /// Contains a [`String`] containing the version string.
    ///
    /// To fix this error, consider passing in the input parse mode explicitly.
    UnknownInputParseMode(String),

    /// The input [`Vec`] isn't sorted.
    /// 
    /// The serializer expects the input [`Vec`] to be sorted, or the game may parse the inputs
    /// in a strange way.
    /// 
    /// To fix this error, consider calling [`sort_inputs`][GameReplayData::sort_inputs] on the
    /// [`GameReplayData`] before serializing it.
    UnsortedInput {
        /// The first data point index in which the array isn't sorted.
        first_unsorted_index: usize,
        /// The frame number of the previous data point.
        prev_time: u64,
        /// The frame number of the first data point which caused the array to not be sorted.
        unsorted_time: u64,
    },

    /// The metadata could not be serialized into JSON.
    ///
    /// See [`serde_json`'s Error type][serde_json::Error] for more information.
    MetadataSerializeError(serde_json::Error),
}

impl From<serde_json::Error> for ReplaySerializeError {
    fn from(value: serde_json::Error) -> Self {
        Self::MetadataSerializeError(value)
    }
}

/// Determines how to parse the inputs of the replay.
///
/// Replays made before version 0.17.22 of the game (i.e., 0.17.21 and before it)
/// use relative timing for its inputs.  
/// However, starting from version 0.17.22 of the game, absolute timing is used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputParseMode {
    /// Relative timing.
    ///
    /// Replays made before version 0.17.22 of the game (i.e., 0.17.21 and before it)
    /// use relative timing for its inputs. That is, the time in each key-time
    /// pair are relative to the frame of the previous input.
    ///
    /// For example, if you press two keys at the exact same frame, the first input
    /// has a time of the current frame number, while the second input has a time of 0.
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
    pub fn try_infer_from_version(version: &str) -> Option<InputParseMode> {
        let filtered_version: String = version
            .chars()
            .filter(|c| c.is_numeric() || *c == '.')
            .collect();

        let version = Version::parse(&filtered_version);

        if version.ok()? < Self::ABSOLUTE_TIMING_START {
            Some(InputParseMode::Relative)
        } else {
            Some(InputParseMode::Absolute)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inferred_mode() {
        use InputParseMode::*;
        let cases = [
            ("Techmino is fun!", None),
            ("Alpha v0.15.1", Some(Relative)),
            ("V0.16.2", Some(Relative)),
            ("0.17.22", Some(Absolute)),
        ];

        for (input, expected) in cases {
            assert_eq!(InputParseMode::try_infer_from_version(input), expected);
        }
    }
}
