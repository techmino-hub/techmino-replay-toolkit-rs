use crate::types::*;

pub fn create_replay_raw_bytes(replay_data: GameReplayData, input_mode: InputParseMode) -> Vec<u8> {
    let _ = (replay_data, input_mode);
    todo!();
}

pub fn create_replay_compressed_bytes(replay_data: GameReplayData, input_mode: InputParseMode) -> Vec<u8> {
    let raw_bytes = create_replay_raw_bytes(replay_data, input_mode);

    todo!();
}

pub fn create_replay_base64_string(replay_data: GameReplayData, input_mode: InputParseMode) -> String {
    let compressed_data = create_replay_compressed_bytes(replay_data, input_mode);
    todo!();
}

fn create_vlqs(values: &[u64]) -> Vec<u8> {
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
            assert_eq!(create_vlqs(&values), expected);
        }
    }
}