//! Handles the `create` operation.

use core::num::NonZeroUsize;
use std::borrow::Cow;

use libtechmino_replay::{
    GameInputEvent, GameReplayData, GameReplayMetadata, InputAction, InputActionKey,
    InputActionKind, InputParseMode, serialize::ReplayEncoder,
};
use serde_json::{Map, Value as JsonValue};

use crate::{
    cli::{clap::CreateArguments, types::CliOpError},
    consts::{
        KEYWORD_FRAME, KEYWORD_INPUTS, KEYWORD_KEY, KEYWORD_METADATA, KEYWORD_TYPE,
        TRT_CREATION_MARKER_KEY, TRT_CREATION_MARKER_VALUE,
    },
};

/// The size of the I/O copying scratch buffer.
const IO_COPY_BUFFER_SIZE: usize = 8192;

pub(super) fn create(args: &CreateArguments) -> Result<(), CliOpError> {
    let mut retry_counter = 0u32;

    let (mut in_stream, out_stream) = args.io_args.get_unbuffered(&mut retry_counter)?;

    if (!args.skip_console_check) && out_stream.is_terminal() {
        return Err(CliOpError::BinaryConsoleOutput);
    }

    let mut input_data = Vec::with_capacity(in_stream.buf_size());
    let mut buf = [0u8; IO_COPY_BUFFER_SIZE];

    loop {
        let bytes_read =
            in_stream.read_with_retry(&mut buf, &mut retry_counter, args.io_args.retry_args)?;

        if bytes_read == 0 {
            break;
        }

        input_data.extend_from_slice(&buf[..bytes_read]);
    }

    drop(in_stream);

    let data = serde_json::from_slice(&input_data)
        .map(process_json_replay_data)
        .map_err(|e| CliOpError::ReplayJsonParseError { inner: e })??;

    let mut out_stream = out_stream.into_buffered();
    let mut encoder = ReplayEncoder::new(args.replay_format.into(), args.compression_level);
    let mut out_buf = encoder.feed_metadata(
        &data.metadata,
        args.override_input_mode.map(InputParseMode::from),
    )?;

    out_stream.append_with_retry(&out_buf, &mut retry_counter, args.io_args.retry_args)?;
    out_buf.clear();

    encoder.feed_input_data(&data.inputs, &mut out_buf)?;

    out_stream.append_with_retry(&out_buf, &mut retry_counter, args.io_args.retry_args)?;
    out_buf.clear();

    encoder.finish(&mut out_buf)?;
    out_stream.append_with_retry(&out_buf, &mut retry_counter, args.io_args.retry_args)?;
    out_buf.clear();

    out_stream.flush_with_retry(&mut retry_counter, args.io_args.retry_args)?;

    Ok(())
}

/// Extracts the deserialized game replay data from a JSON replay data.
fn process_json_replay_data(value: JsonValue) -> Result<GameReplayData, CliOpError> {
    let JsonValue::Object(mut value) = value else {
        return Err(CliOpError::ReplayJsonSchemaError {
            inner: "the input json must be an object".into(),
        });
    };

    let metadata = value
        .remove(KEYWORD_METADATA)
        .ok_or(CliOpError::ReplayJsonSchemaError {
            inner: "missing field 'metadata' in top-level object".into(),
        })?;

    let JsonValue::Object(mut metadata) = metadata else {
        return Err(CliOpError::ReplayJsonSchemaError {
            inner: "field 'metadata' in top-level object must be an object".into(),
        });
    };

    if let Some(val) = TRT_CREATION_MARKER_VALUE {
        metadata.insert(TRT_CREATION_MARKER_KEY.into(), val.into());
    }

    let inputs = value
        .remove(KEYWORD_INPUTS)
        .ok_or(CliOpError::ReplayJsonSchemaError {
            inner: "missing field 'inputs' in top-level object".into(),
        })?;

    let JsonValue::Array(inputs) = inputs else {
        return Err(CliOpError::ReplayJsonSchemaError {
            inner: "field 'inputs' in top-level object must be an array".into(),
        });
    };

    let inputs = {
        let mut events = Vec::with_capacity(inputs.len());

        for (idx, value) in inputs.into_iter().enumerate() {
            let ordinal = NonZeroUsize::new(idx + 1).unwrap_or(NonZeroUsize::MAX);
            events.push(jsonvalue_to_inputevent(ordinal, value)?);
        }

        events
    };

    Ok(GameReplayData {
        metadata: GameReplayMetadata { map: metadata },
        inputs,
    })
}

/// Converts a [`JsonValue`] into a [`GameInputEvent`], returning a
/// [`CliOpError`] if unsuccessful.
///
/// # `ordinal` parameter
/// The argument for this parameter is used for error reporting and should be
/// the ordinal index of the given value.
///
/// e.g. index 0 becomes `1`, index 1 becomes `2`, etc.
///
/// # `value` parameter
/// This parameter depicts the inner [`JsonValue`] that is to be converted into
/// a [`GameInputEvent`].
///
/// It currently expects an object with the format
/// `{"frame":number, "key":number, "kind":number}`.
fn jsonvalue_to_inputevent(
    ordinal: NonZeroUsize,
    value: JsonValue,
) -> Result<GameInputEvent, CliOpError> {
    let JsonValue::Object(mut value) = value else {
        return Err(CliOpError::ReplayJsonSchemaError {
            inner: format!("input events must be an object (input #{ordinal} is not)").into(),
        });
    };

    let missing_errmsg = move |keyword: &'static str| CliOpError::ReplayJsonSchemaError {
        inner: Cow::Owned(format!(
            "input event objects must contain a '{keyword}' field (input #{ordinal} does not)"
        )),
    };

    let not_int_errmsg = move |keyword: &'static str| CliOpError::ReplayJsonSchemaError {
        inner: Cow::Owned(format!(
            "input event objects must have a positive integer in its '{keyword}' field (input #{ordinal} does not)"
        )),
    };

    let invalid_num_errmsg = move |number: u64, keyword: &'static str| {
        CliOpError::ReplayJsonSchemaError {
            inner: Cow::Owned(format!(
                "invalid value '{number}' for the '{keyword}' field of input #{ordinal} (is it out of range?)"
            )),
        }
    };

    let extract_u64 = move |value: &mut Map<String, JsonValue>, keyword: &'static str| {
        value
            .get(keyword)
            .ok_or_else(|| missing_errmsg(keyword))?
            .as_u64()
            .ok_or_else(|| not_int_errmsg(keyword))
    };

    let frame = extract_u64(&mut value, KEYWORD_FRAME)?;
    let key = extract_u64(&mut value, KEYWORD_KEY)?;
    let kind = extract_u64(&mut value, KEYWORD_TYPE)?;

    if frame > GameInputEvent::MAX_FRAME {
        return Err(invalid_num_errmsg(frame, KEYWORD_FRAME));
    }

    let Some(key) = u8::try_from(key)
        .ok()
        .and_then(|byte| InputActionKey::try_from_byte(byte).ok())
    else {
        return Err(invalid_num_errmsg(key, KEYWORD_KEY));
    };

    let Some(kind) = bool::try_from(kind).ok().map(InputActionKind::from_bool) else {
        return Err(invalid_num_errmsg(kind, KEYWORD_TYPE));
    };

    let event = match GameInputEvent::new(frame, InputAction { kind, key }) {
        Ok(e) => e,
        Err(e) => {
            return Err(CliOpError::ReplayJsonSchemaError {
                inner: format!("failed to create game input event: {e}").into(),
            });
        }
    };

    Ok(event)
}
