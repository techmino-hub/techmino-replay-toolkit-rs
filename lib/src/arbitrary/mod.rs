use crate::{GameInputEvent, GameReplayData, GameReplayMetadata, InputAction};

use arbitrary::{Arbitrary, Unstructured};

use serde_json::Map;

mod json;

use json::ArbitraryValue;

impl<'a> Arbitrary<'a> for GameInputEvent {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let frame = u.int_in_range(0..=Self::MAX_FRAME)?;
        let action: InputAction = u.arbitrary()?;

        Self::new(frame, action).map_err(|_| arbitrary::Error::IncorrectFormat)
    }

    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        (9, Some(9))
    }
}

impl<'a> Arbitrary<'a> for GameReplayData {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let metadata = GameReplayMetadata::arbitrary(u)?;

        let mut inputs = Vec::new();

        let mut keep_going = u.arbitrary().unwrap_or(false);
        let mut prev_frame = 0u64;

        while keep_going {
            let frame = u.int_in_range(prev_frame..=GameInputEvent::MAX_FRAME)?;
            let action: InputAction = u.arbitrary()?;

            let input = GameInputEvent::new(frame, action)
                .map_err(|_| arbitrary::Error::IncorrectFormat)?;

            prev_frame = frame;

            inputs.push(input);

            keep_going = u.arbitrary().unwrap_or(false);
        }

        Ok(Self { inputs, metadata })
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        // Lower bound: Minimum metadata, no inputss
        // (Lack of) upper bound: Max metadata, infinite game input event sequence

        let metadata = GameReplayMetadata::size_hint(depth);

        (metadata.0, None)
    }
}

pub(crate) fn arbitrary_json_map(
    u: &mut Unstructured,
) -> arbitrary::Result<Map<String, serde_json::Value>> {
    let mut map = Map::new();

    let mut keep_going = u.arbitrary().unwrap_or(false);

    while keep_going {
        let entry: (String, ArbitraryValue) = u.arbitrary()?;
        let (k, v) = (entry.0, entry.1.into());

        map.insert(k, v);

        keep_going = u.arbitrary().unwrap_or(false);
    }

    Ok(map)
}
