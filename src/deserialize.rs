use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use miniz_oxide::inflate;

use crate::types::*;

impl GameReplayData {
    /// Parses a base64 string into a game replay.
    ///
    /// For parsing a replay from the contents of a `.rep` file in the game's `replays` directory,
    /// see [`parse_compressed_bytes`] instead.
    ///
    /// `parse_mode` is an optional argument used to specify how you want the inputs to be parsed.  
    /// This is useful for preventing errors from occurring if this function fails to recognize
    /// the game version to automatically infer its parse mode.  
    /// For more information, see [`InputParseMode`].
    pub fn try_from_base64(
        string: &str,
        parse_mode: Option<InputParseMode>,
    ) -> Result<GameReplayData, ReplayParseError> {
        let data = B64.decode(string)?;

        Ok(Self::try_from_compressed(&data, parse_mode)?)
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
    pub fn try_from_compressed(
        data: &[u8],
        parse_mode: Option<InputParseMode>,
    ) -> Result<GameReplayData, ReplayParseError> {
        let data = inflate::decompress_to_vec_zlib(data)?;

        Ok(Self::try_from_raw(&data, parse_mode)?)
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
    pub fn try_from_raw(
        data: &[u8],
        parse_mode: Option<InputParseMode>,
    ) -> Result<GameReplayData, ReplayParseError> {
        let first_newline = match data.iter().position(|&el| el == 10) {
            Some(loc) => loc,
            None => return Err(ReplayParseError::MetadataSeparatorNotFound),
        };

        let (metadata_slice, input_slice) = data.split_at(first_newline);

        let input_slice = &input_slice[1..];

        let metadata = GameReplayMetadata::try_from(metadata_slice)?;

        let parse_mode = match parse_mode
            .or_else(|| InputParseMode::try_infer_from_version(&metadata.version))
        {
            Some(mode) => mode,
            None => return Err(ReplayParseError::UnknownInputParseMode(metadata.version)),
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

fn parse_input_slice(
    input_slice: &[u8],
    parse_mode: InputParseMode,
) -> Result<Vec<GameInputEvent>, ReplayParseError> {
    let values = extract_vlqs(input_slice);

    let mut events = Vec::with_capacity(values.len() / 2);

    let mut prev_timestamp = 0;
    for (position, chunk) in values.chunks_exact(2).enumerate() {
        let (time, key) = (chunk[0], chunk[1]);

        let frame = match parse_mode {
            InputParseMode::Relative => time + prev_timestamp,
            InputParseMode::Absolute => time,
        };

        let kind = InputEventKind::from(key > 0b100000);
        let key = InputEventKey::try_from(key as u8 & 0b011111).map_err(|_| {
            ReplayParseError::MalformedInputData {
                frame,
                position: position as u64 * 2,
                kind: key,
            }
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
}
