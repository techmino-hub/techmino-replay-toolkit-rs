use alloc::string::String;
use alloc::vec::Vec;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use miniz_oxide::inflate;

use crate::{
    action::{InputActionKey, InputActionKind},
    types::{GameInputEvent, GameReplayData, GameReplayMetadata, InputParseMode, ReplayParseError},
    vlq::{VlqData, VlqReader},
};

impl GameReplayData {
    /// Parses a base64 string into a game replay.
    ///
    /// For parsing a replay from the contents of a `.rep` file in the game's `replays` directory,
    /// see [`parse_compressed_bytes`] instead.
    ///
    /// `parse_mode` is an optional argument used to specify how you want the inputs to be parsed.\
    /// This is useful for preventing errors from occurring if this function fails to recognize
    /// the game version to automatically infer its parse mode.\
    /// For more information, see [`InputParseMode`].
    ///
    /// # Errors
    /// For more information on possible errors, look at the [`ReplayParseError`] struct.
    pub fn try_from_base64(
        string: &str,
        parse_mode: Option<InputParseMode>,
    ) -> Result<GameReplayData, ReplayParseError> {
        let data = B64.decode(string)?;

        Self::try_from_compressed(&data, parse_mode)
    }

    /// Parses a compressed byte array into a game replay.
    ///
    /// The byte array can be in the form of the contents of a `.rep` file in the game's `replays` directory.
    ///
    /// For parsing a replay from a base64 string, see [`parse_base64`] instead.
    ///
    /// `parse_mode` is an optional argument used to specify how you want the inputs to be parsed.\
    /// This is useful for preventing errors from occurring if this function fails to recognize
    /// the game version to automatically infer its parse mode.
    /// For more information, see [`InputParseMode`].
    ///
    /// # Errors
    /// For more information on possible errors, look at the [`ReplayParseError`] struct.
    pub fn try_from_compressed(
        data: &[u8],
        parse_mode: Option<InputParseMode>,
    ) -> Result<GameReplayData, ReplayParseError> {
        let data = inflate::decompress_to_vec_zlib(data)?;

        Self::try_from_raw(&data, parse_mode)
    }

    /// Parses a raw, uncompressed byte array into a game replay.
    ///
    /// Usually, Techmino compresses the replay using `zlib` before saving it, either as a
    /// base64 string, or a `.rep` file in the game's `replays` directory.\
    /// In which case, this is not what you are looking for.\
    /// See [`parse_base64`] and [`parse_compressed_bytes`] instead.
    ///
    /// This function is only useful if you managed to get the replay in the uncompressed form,
    /// which doesn't usually seem to be the case.
    ///
    /// # Errors
    /// For more information on possible errors, look at the [`ReplayParseError`] struct.
    pub fn try_from_raw(
        data: &[u8],
        parse_mode: Option<InputParseMode>,
    ) -> Result<GameReplayData, ReplayParseError> {
        let Some(first_newline) = data.iter().position(|&el| el == b'\n') else {
            return Err(ReplayParseError::MetadataSeparatorNotFound);
        };

        let (metadata_slice, input_slice) = data.split_at(first_newline);

        // This will never panic becaause we already know that there is a
        // separator
        let input_slice = &input_slice[1..];

        let metadata = GameReplayMetadata::try_from(metadata_slice)?;

        let Some(parse_mode) =
            parse_mode.or_else(|| InputParseMode::try_infer_from_version(&metadata.version))
        else {
            return Err(ReplayParseError::UnknownInputParseMode(metadata.version));
        };

        Ok(GameReplayData {
            inputs: parse_input_slice(input_slice, parse_mode)?,
            metadata,
        })
    }
}

impl TryFrom<&[u8]> for GameReplayMetadata {
    type Error = ReplayParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let string = String::from_utf8(Vec::from(value))?;

        Ok(serde_json::from_str(&string)?)
    }
}

pub(crate) fn parse_input_slice(
    input_slice: &[u8],
    parse_mode: InputParseMode,
) -> Result<Vec<GameInputEvent>, ReplayParseError> {
    todo!();
    // let mut events = Vec::with_capacity(input_slice.len());

    // let mut prev_timestamp = 0;
    // for (position, chunk) in values.chunks_exact(2).enumerate() {
    //     let (time, key) = (chunk[0], chunk[1]);

    //     let frame = match parse_mode {
    //         InputParseMode::Relative => time + prev_timestamp,
    //         InputParseMode::Absolute => time,
    //     };

    //     let Ok(key) = u8::try_from(key) else {
    //         return Err(ReplayParseError::MalformedInputData {
    //             position: position as u64 * 2,
    //             frame,
    //             kind: key,
    //         });
    //     };

    //     let kind = InputEventKind::from(key > 0b0010_0000);
    //     let Ok(key) = InputEventKey::try_from(key & 0b0001_1111) else {
    //         return Err(ReplayParseError::MalformedInputData {
    //             frame,
    //             position: position as u64 * 2,
    //             kind: u64::from(key),
    //         });
    //     };

    //     let Ok(event) = GameInputEvent::new(kind, key, frame) else {
    //         return Err(ReplayParseError::MalformedInputData {
    //             frame,
    //             position: position as u64 * 2,
    //             kind: u64::from(u8::from(key)),
    //         });
    //     };

    //     prev_timestamp = frame;

    //     events.push(event);
    // }

    // Ok(events)
}

pub(crate) fn parse_input_iter<I>(
    mut input_iter: I,
    parse_mode: InputParseMode,
) -> Result<Vec<GameInputEvent>, ReplayParseError>
where
    I: Iterator<Item = u8>,
{
    loop {
        // It goes frame first then key.
        let mut reader = VlqReader::new(input_iter);

        let frame_vlq = reader.next();

        input_iter = reader.into_inner();
    }

    todo!();
}

//: hmmm should i really output once even though i know it's an iterator
//: i don't think so

/// Gets the raw frame-key input pairs, with no relative/absolute
/// processing.
///
/// Use [`parse_input_iter`] for the version with that kind of processing.
fn get_raw_input_pairs<I>(mut input_iter: I) -> RawInputPairsIter<I>
where
    I: Iterator<Item = u8>,
{
    todo!();
}

struct RawInputPairsIter<I: Iterator<Item = u8>>(Option<I>);

impl<I> Iterator for RawInputPairsIter<I>
where
    I: Iterator<Item = u8>,
{
    type Item = (VlqData, (InputActionKey, InputActionKind));

    fn next(&mut self) -> Option<Self::Item> {
        let mut reader = VlqReader::new(self.0.take()?);

        let frame = reader.next()?.unwrap(); // FIXME: error handling

        let mut iter = reader.into_inner();

        let keycode = iter.next()?;

        todo!();
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let Some(ref iter) = self.0 else {
            return (0, Some(0));
        };

        let bytes_hint = iter.size_hint();

        // Min case: Each frame is 8 bytes, so each event is 9 bytes
        let min = bytes_hint.0 / 9;

        // Max case: Each frame is 1 byte, so each event is 2 bytes
        let max = bytes_hint.1.map(|max| max / 2);

        (min, max)
    }
}
