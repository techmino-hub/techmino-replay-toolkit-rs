use base64::{DecodeError, Engine};
use miniz_oxide::inflate::{self, DecompressError};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, string::FromUtf8Error};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InputEventKind {
    Press = 0,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// A single input event in the game.
///
/// `frame`: A number representing the frame this event occured in.
/// `kind`: The kind of input event this represents (pressed/released).
/// `key`: Which key is being pressed/released.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GameInputEvent {
    pub frame: u64,
    pub kind: InputEventKind,
    pub key: InputEventKey,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct GameReplayData {
    pub inputs: Vec<GameInputEvent>,
    pub metadata: GameReplayMetadata,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct PlayerSettings {
    pub shake_fx: Option<u64>,
    pub splash_fx: Option<u64>,
    pub das: Option<u64>,
    pub high_cam: Option<bool>,
    pub smooth: Option<bool>,
    pub warn: Option<bool>,
    pub dropcut: Option<u64>,
    pub ghost: Option<f64>,
    pub atk_fx: Option<u64>,
    pub next_pos: Option<bool>,
    pub block: Option<bool>,
    pub text: Option<bool>,
    pub ihs: Option<bool>,
    pub face: Option<Vec<u64>>,
    pub score: Option<bool>,
    pub irs: Option<bool>,
    pub center: Option<u64>,
    pub sdarr: Option<u64>,
    pub move_fx: Option<u64>,
    pub drop_fx: Option<u64>,
    pub ims: Option<bool>,
    pub lock_fx: Option<u64>,
    pub arr: Option<u64>,
    pub swap: Option<bool>,
    pub bag_line: Option<bool>,
    pub skin: Option<Vec<u64>>,
    pub grid: Option<f64>,
    pub dascut: Option<u64>,
    pub sddas: Option<u64>,
    pub rs: Option<String>,
    pub clear_fx: Option<u64>,
    /// Additional settings that may not be standard.
    #[serde(flatten)]
    pub nonstandard: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
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
    /// The separator is a linefeed character, or a byte a decimal value of `10`.
    MetadataSeparatorNotFound,

    /// The metadata was found to not be valid UTF-8.
    ///
    /// See [FromUtf8Error] for more information.
    MetadataNotUtf8(FromUtf8Error),

    /// The metadata could not be serialized into the [GameReplayMetadata] struct,
    /// possibly due to missing values.
    ///
    /// See [`serde_json`'s Error type][serde_json::Error] for more information.
    MetadataSerializeError(serde_json::Error),

    /// The mode in which to parse the input could not be inferred from the version string.
    ///
    /// Contains a [`String`] containing the version string.
    ///
    /// To fix this error, consider passing in the input parse mode explicitly.
    UnknownInputParseMode(String),

    /// The input data was malformed and could not be casted into the proper enum types.
    MalformedInputData {
        position: u64,
        frame: u64,
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
    /// use relative timing for its inputs. That is, the duration in each key-duration
    /// pair are relative to the frame of the previous input. For example, if you press
    /// two keys at the exact same frame, the first input has a duration of 0 while the second
    /// input has a duration of 0.
    Relative,
    /// Absolute timing.
    ///
    /// Replays made after version 0.17.21 of the game (i.e., 0.17.22 and onwards)
    /// use absolute timing for its inputs. That is, the duration in each key-duration
    /// pair are relative to the beginning of the replay (i.e., frame zero).
    Absolute,
}

/// Parses a base64 string into a game replay.
///
/// For parsing a replay from the contents of a `.rep` file in the game's `replays` directory,
/// see [`parse_compressed_bytes`] instead.
///
/// `parse_mode` is an optional argument used to specify how you want the inputs to be parsed.  
/// This is useful for preventing errors from occurring if this function fails to recognize
/// the game version to automatically infer its parse mode.  
/// For more information, see [`InputParseMode`].
pub fn parse_base64(
    string: &str,
    parse_mode: Option<InputParseMode>,
) -> Result<GameReplayData, ReplayParseError> {
    let data = base64::engine::general_purpose::STANDARD.decode(string)?;

    Ok(parse_compressed_bytes(&data, parse_mode)?)
}

/// Parses a compressed byte array into a game replay.
///
/// The byte array can be in the form of the contents of a `.rep` file in the game's `replays` directory.
///
/// For parsing a replay from a base64 string, see [`parse_base64`] instead.
///
/// `parse_mode` is an optional argument used to specify how you want the inputs to be parsed.  
/// This is useful for preventing errors from occurring if this function fails to recognize
/// the game version to automatically infer its parse mode.
/// For more information, see [`InputParseMode`].
pub fn parse_compressed_bytes(
    data: &[u8],
    parse_mode: Option<InputParseMode>,
) -> Result<GameReplayData, ReplayParseError> {
    let data = inflate::decompress_to_vec_zlib(data)?;

    Ok(parse_raw_bytes(&data, parse_mode)?)
}

/// Parses a raw, uncompressed byte array into a game replay.
///
/// Usually, Techmino compresses the replay using `zlib` before saving it, either as a
/// base64 string, or a `.rep` file in the game's `replays` directory.  
/// In which case, this is not what you are looking for.  
/// See [`parse_base64`] and [`parse_compressed_bytes`] instead.
///
/// This function is only useful if you managed to get the replay in the uncompressed form,
/// which doesn't usually seem to be the case.
pub fn parse_raw_bytes(
    data: &[u8],
    parse_mode: Option<InputParseMode>,
) -> Result<GameReplayData, ReplayParseError> {
    let first_newline = match data.iter().position(|&el| el == 10) {
        Some(loc) => loc,
        None => return Err(ReplayParseError::MetadataSeparatorNotFound),
    };

    let (metadata_slice, input_slice) = data.split_at(first_newline);

    let input_slice = &input_slice[1..];

    let metadata = parse_metadata_slice(metadata_slice)?;

    let parse_mode = match parse_mode.or(infer_input_parse_mode(&metadata.version)) {
        Some(mode) => mode,
        None => return Err(ReplayParseError::UnknownInputParseMode(metadata.version)),
    };

    Ok(GameReplayData {
        inputs: parse_input_slice(input_slice, parse_mode)?,
        metadata,
    })
}

fn parse_metadata_slice(metadata_slice: &[u8]) -> Result<GameReplayMetadata, ReplayParseError> {
    let string = String::from_utf8(Vec::from(metadata_slice))?;

    Ok(serde_json::from_str(&string)?)
}

/// The first version where absolute timing is used.
const ABSOLUTE_TIMING_START: Version = Version::new(0, 17, 22);

/// Tries to infer the input parse mode based on the game version.
///
/// If parsing the version fails, it will return `None`.
pub fn infer_input_parse_mode(version: &str) -> Option<InputParseMode> {
    let filtered_version: String = version
        .chars()
        .filter(|c| c.is_numeric() || *c == '.')
        .collect();

    let version = Version::parse(&filtered_version);

    if version.ok()? < ABSOLUTE_TIMING_START {
        Some(InputParseMode::Relative)
    } else {
        Some(InputParseMode::Absolute)
    }
}

fn parse_input_slice(
    input_slice: &[u8],
    parse_mode: InputParseMode,
) -> Result<Vec<GameInputEvent>, ReplayParseError> {
    let values = extract_vlqs(input_slice);

    println!("Input slice values: {values:?}");

    let mut events = Vec::with_capacity(values.len() / 2);

    let mut prev_timestamp = 0;
    for (position, chunk) in values.chunks_exact(2).enumerate() {
        let (time, key) = (chunk[0], chunk[1]);

        let frame = match parse_mode {
            InputParseMode::Relative => time + prev_timestamp,
            InputParseMode::Absolute => time,
        };

        let kind = InputEventKind::from(key > 0b100000);
        let key = InputEventKey::try_from(key as u8 & 0b011111)
            .map_err(|_| ReplayParseError::MalformedInputData {
                frame,
                position: position as u64 * 2,
                kind: key,
            })?;

        prev_timestamp = frame;

        events.push(GameInputEvent { frame, key, kind });
    }

    Ok(events)
}

fn extract_vlqs(vlqs: &[u8]) -> Vec<u64> {
    let mut numbers = Vec::with_capacity(vlqs.len());

    let mut cur_num: u64 = 0;
    for &vlq in vlqs.iter() {
        let value = vlq & 0x7F;
        cur_num <<= 7;
        cur_num |= value as u64;

        let msb = vlq >= 0x80;
        if !msb {
            numbers.push(cur_num);
            cur_num = 0;
        }
    }

    numbers
}

fn create_vlqs(values: &[u64]) -> Vec<u8> {
    todo!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vlq_extraction() {
        // Mostly sourced from https://en.wikipedia.org/wiki/Variable-length_quantity#Examples
        let cases = [
            (vec![0x00], vec![0x00]),
            (vec![0x01], vec![0x01]),
            (vec![0x7F], vec![0x7F]),
            (vec![0x81, 0x00], vec![0x80]),
            (vec![0xC0, 0x00], vec![0x2000]),
            (vec![0xFF, 0x7F], vec![0x3FFF]),
            (vec![0x81, 0x80, 0x00], vec![0x4000]),
            (vec![0xFF, 0xFF, 0x7F], vec![0x1FFFFF]),
            (
                vec![0xFF, 0xFF, 0x7F, 0xFF, 0xFF, 0x7F],
                vec![0x1FFFFF, 0x1FFFFF],
            ),
            (vec![0x81, 0x80, 0x80, 0x00], vec![0x200000]),
            (vec![0x01, 0x01, 0x01], vec![1, 1, 1]),
            (vec![0x8F, 0x00], vec![1920]),
        ];

        for (input, expected) in cases {
            assert_eq!(extract_vlqs(&input), expected);
        }
    }

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
            assert_eq!(infer_input_parse_mode(input), expected);
        }
    }
}
