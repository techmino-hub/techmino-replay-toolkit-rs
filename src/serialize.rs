use crate::types::*;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use miniz_oxide::deflate::compress_to_vec_zlib as compress;

// TODO: Restructure, add tests

pub fn create_replay_raw_bytes(
    replay_data: GameReplayData,
    input_mode: Option<InputParseMode>,
) -> Result<Vec<u8>, ReplayCreateError> {
    let input_mode = match input_mode
        .or_else(|| InputParseMode::try_infer_from_version(&replay_data.metadata.version))
    {
        Some(mode) => mode,
        None => {
            return Err(ReplayCreateError::UnknownInputParseMode(
                replay_data.metadata.version,
            ))
        }
    };

    let json = serde_json::to_string(&replay_data.metadata)?;

    let mut buffer = Vec::from(json.as_bytes());

    let mut inputs = replay_data.inputs;

    inputs.sort_by_key(|e| e.frame);

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

    append_vlqs(&mut buffer, &bytes);

    Ok(buffer)
}

pub fn create_replay_compressed_bytes(
    replay_data: GameReplayData,
    input_mode: Option<InputParseMode>,
) -> Result<Vec<u8>, ReplayCreateError> {
    let raw_bytes = create_replay_raw_bytes(replay_data, input_mode)?;

    Ok(compress(&raw_bytes, 10))
}

pub fn create_replay_base64_string(
    replay_data: GameReplayData,
    input_mode: Option<InputParseMode>,
) -> Result<String, ReplayCreateError> {
    let bytes = create_replay_compressed_bytes(replay_data, input_mode)?;

    Ok(B64.encode(&bytes))
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

    buffer.push(10);

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
}
