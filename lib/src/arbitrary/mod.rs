use crate::{GameInputEvent, GameReplayData, GameReplayMetadata, InputAction};

use arbitrary::{Arbitrary, Unstructured};

#[cfg(feature = "alloc")]
use hashbrown::HashMap;
#[cfg(feature = "std")]
use std::collections::HashMap;

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

pub(crate) fn arbitrary_modlist(
    u: &mut Unstructured,
) -> arbitrary::Result<Option<Vec<(u64, serde_json::Value)>>> {
    let is_some: bool = u.arbitrary()?;

    if !is_some {
        return Ok(None);
    }

    let mut mods = Vec::new();

    loop {
        let keep_going = u.arbitrary().unwrap_or(false);

        if !keep_going {
            break;
        }

        let mod_id: u64 = u.arbitrary()?;
        let mod_value: ArbitraryValue = u.arbitrary()?;
        let mod_value: serde_json::Value = mod_value.into();

        mods.push((mod_id, mod_value));
    }

    Ok(Some(mods))
}

pub(crate) fn arbitrary_nonstandard(
    u: &mut Unstructured,
) -> arbitrary::Result<HashMap<String, serde_json::Value>> {
    let mut map = HashMap::new();

    let mut keep_going = u.arbitrary().unwrap_or(false);

    while keep_going {
        let entry: (String, ArbitraryValue) = u.arbitrary()?;
        let (k, v) = (entry.0, entry.1.into());

        map.insert(k, v);

        keep_going = u.arbitrary().unwrap_or(false);
    }

    Ok(map)
}

pub(crate) fn arbitrary_optional_value(
    u: &mut Unstructured,
) -> arbitrary::Result<Option<serde_json::Value>> {
    let value: Option<ArbitraryValue> = u.arbitrary()?;
    let value: Option<serde_json::Value> = value.map(core::convert::Into::into);

    Ok(value)
}
