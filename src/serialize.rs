use crate::types::*;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use miniz_oxide::deflate::compress_to_vec_zlib as compress;

// TODO: Add tests

impl GameReplayData {

    /// Sort the inputs so that they are sorted by time.
    /// 
    /// This can be necessary sometimes as serializing the replay (e.g., into base64)
    /// requires that the inputs are sorted for the algorithm to work properly.
    pub fn sort_inputs(&mut self) {
        self.inputs.sort_by_key(|i| i.frame);
    }

    /// Serialize into a raw, uncompressed byte array.
    /// 
    /// This function serializes the GameReplayData into a raw, uncompressed byte array.
    /// 
    /// This will not be playable by the game as the game automatically compresses and decompresses the data.  
    /// For serializing the data into the `.rep` file format used by the game's saved replays, use
    /// [`serialize_to_compressed`][GameReplayData::serialize_to_compressed] instead.  
    /// For serializing the data into a copiable text/base64 format, use
    /// [`serialize_to_base64`][GameReplayData::serialize_to_base64] instead.
    /// 
    /// Note that the serialization algorithm requires that the inputs in the replay are sorted to time.  
    /// If this isn't always the case, consider calling [`sort_inputs`][GameReplayData::sort_inputs] before calling this function,
    /// otherwise an [`UnsortedInput`][ReplaySerializeError::UnsortedInput] error will be returned.
    pub fn serialize_to_raw(&self, input_mode: Option<InputParseMode>) -> Result<Vec<u8>, ReplaySerializeError> {
        let input_mode = match input_mode
            .or_else(|| InputParseMode::try_infer_from_version(&self.metadata.version))
        {
            Some(mode) => mode,
            None => {
                return Err(ReplaySerializeError::UnknownInputParseMode(
                    self.metadata.version.clone(),
                ))
            }
        };

        let json = serde_json::to_string(&self.metadata)?;

        let mut buffer = Vec::from(json.as_bytes());

        let inputs = &self.inputs;

        if let Some(u) = get_first_unsorted(&inputs) {
            return Err(u);
        }

        let mut bytes = Vec::with_capacity(inputs.len() * 2);

        let mut prev_time = 0;
        for input in inputs {
            let key = u8::from(input.key) | (u8::from(input.kind) << 5);

            let time = match input_mode {
                InputParseMode::Relative => input.frame - prev_time,
                InputParseMode::Absolute => input.frame,
            };

            prev_time = input.frame;

            bytes.push(key as u64);
            bytes.push(time);
        }
        
        buffer.push(10);
        append_vlqs(&mut buffer, &bytes);

        Ok(buffer)
    }
    
    /// Serialize into a compressed byte array used by the game.
    /// 
    /// This data format is used by the game in the form of `.rep` files that are placed in
    /// the `replays/` directory of the game's save directory.  
    /// For serializing the data into a copiable text/base64 format, use
    /// [`serialize_to_base64`][GameReplayData::serialize_to_base64] instead.  
    /// FOr serializing the data into a raw, uncompressed byte array form, use
    /// [`serialize_to_raw`][GameReplayData::serialize_to_raw] instead.
    /// 
    /// Note that the serialization algorithm requires that the inputs in the replay are sorted to time.  
    /// If this isn't always the case, consider calling [`sort_inputs`][GameReplayData::sort_inputs] before calling this function,
    /// otherwise an [`UnsortedInput`][ReplaySerializeError::UnsortedInput] error will be returned.
    pub fn serialize_to_compressed(
        &self,
        input_mode: Option<InputParseMode>,
    ) -> Result<Vec<u8>, ReplaySerializeError> {
        let raw_bytes = self.serialize_to_raw(input_mode)?;
    
        Ok(compress(&raw_bytes, 10))
    }
    
    /// Serialize into a copiable text-based base64 format.
    /// 
    /// This data format is used by the game for importing/exporting replays.
    /// For serializing the data into the `.rep` file format used by the game's saved replays, use
    /// [`serialize_to_compressed`][GameReplayData::serialize_to_compressed] instead.  
    /// FOr serializing the data into a raw, uncompressed byte array form, use
    /// [`serialize_to_raw`][GameReplayData::serialize_to_raw] instead.
    /// 
    /// Note that the serialization algorithm requires that the inputs in the replay are sorted to time.  
    /// If this isn't always the case, consider calling [`sort_inputs`][GameReplayData::sort_inputs] before calling this function,
    /// otherwise an [`UnsortedInput`][ReplaySerializeError::UnsortedInput] error will be returned.
    pub fn serialize_to_base64(
        &self,
        input_mode: Option<InputParseMode>,
    ) -> Result<String, ReplaySerializeError> {
        let bytes = self.serialize_to_compressed(input_mode)?;
    
        Ok(B64.encode(&bytes))
    }
}

fn get_first_unsorted(inputs: &[GameInputEvent]) -> Option<ReplaySerializeError> {
    for (index, window) in inputs.windows(2).enumerate() {
        let prev = window[0];
        let cur = window[1];

        if cur.frame < prev.frame {
            return Some(ReplaySerializeError::UnsortedInput {
                first_unsorted_index: index + 1,
                prev_time: prev.frame,
                unsorted_time: cur.frame
            });
        }
    }

    None
}

fn _create_vlqs(values: &[u64]) -> Vec<u8> {
    // Estimation: most values need around 2 bytes
    let mut vlqs = Vec::with_capacity(values.len() * 2);

    // u64 is up to 9 VLQ bytes
    let mut vlq = Vec::with_capacity(9);
    for &value in values {
        vlq.clear();
        let mut value = value;

        vlq.push((value & 0x7F) as u8);
        value >>= 7;

        while value > 0 {
            vlq.push(((value & 0x7F) | 0x80) as u8);
            value >>= 7;
        }

        vlq.reverse();
        vlqs.append(&mut vlq);
    }

    vlqs
}

fn append_vlqs(buffer: &mut Vec<u8>, values: &[u64]) {
    // Estimation: most values need around 2 bytes
    buffer.reserve(values.len() * 2 + 1);
    
    // u64 is up to 9 VLQ bytes
    let mut vlq = Vec::with_capacity(9);
    for &value in values {
        vlq.clear();
        let mut value = value;

        vlq.push((value & 0x7F) as u8);
        value >>= 7;

        while value > 0 {
            vlq.push(((value & 0x7F) | 0x80) as u8);
            value >>= 7;
        }

        vlq.reverse();
        buffer.append(&mut vlq);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vlq_creation() {
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

        for (expected, values) in cases {
            assert_eq!(_create_vlqs(&values), expected);
        }
    }

    #[test]
    fn test_vlq_append() {
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

        for (expected, values) in cases {
            let mut vec = Vec::new();
            append_vlqs(&mut vec, &values);
            assert_eq!(vec, expected);
        }
    }

    #[test]
    fn test_input_slice_parse() {
        use crate::deserialize::parse_input_slice;

        struct InputSliceParseTestcase {
            raw: Vec<u8>,
            expect_pass: bool,
        }

        let cases = [
            InputSliceParseTestcase {
                raw: vec![2, 1, 9, 1, 3, 1],
                expect_pass: false,
            },
        ];

        for InputSliceParseTestcase { raw, expect_pass } in cases {
            let inputs = parse_input_slice(&raw, InputParseMode::Absolute)
                .unwrap();
            let data = GameReplayData {
                inputs,
                ..Default::default()
            };

            let reserialized =
                data.serialize_to_raw(Some(InputParseMode::Absolute));

            if expect_pass {
                reserialized.unwrap();
            } else {
                reserialized.unwrap_err();
            }
        }
    }
}
