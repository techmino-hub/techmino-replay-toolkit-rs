//! Handles the `extract` operation.

use core::ops::ControlFlow;

use crate::cli::{
    clap::{ExtractArguments, RetryArguments},
    io::buffered::OutputBufWriter,
    operations::infer_replay_kind,
    types::{CliOpError, UnpackedInputEvent},
};
use libtechmino_replay::{GameInputEvent, GameReplayMetadata, deserialize::ReplayDecoder};

// TODO: Optimize: Skip unnecessary sections by stabilizing preprocessors
pub(super) fn extract(args: &ExtractArguments) -> Result<(), CliOpError> {
    let mut retry_counter = 0u32;

    let (mut input_stream, mut output_stream) = args.io_args.get_buffered(&mut retry_counter)?;

    eprintln!("> starting read from input stream...");

    let mut input_chunk =
        input_stream.buffer_with_retry(&mut retry_counter, args.io_args.retry_args)?;
    let replay_kind = infer_replay_kind(args.replay_format, input_chunk)?;
    eprintln!("> replay kind: {replay_kind:?}");

    let mut decoder = ReplayDecoder::new(replay_kind, args.override_input_mode.map(Into::into));

    let (read_metadata, read_inputs) = args.scope.to_keeps();

    let mut is_first_input = true;

    if let Some(header) = args.scope.header() {
        output_stream.append_with_retry(header, &mut retry_counter, args.io_args.retry_args)?;
    }

    loop {
        let res = decoder.update(input_chunk)?;

        let chunk_len = input_chunk.len();
        input_stream.consume(chunk_len);

        if read_metadata {
            let cf = extract_metadata(
                res.metadata,
                &mut output_stream,
                read_inputs,
                &mut retry_counter,
                args.io_args.retry_args,
            );
            if let ControlFlow::Break(res) = cf {
                return res;
            }
        }

        if read_inputs {
            extract_inputs(
                &res.inputs,
                &mut output_stream,
                &mut is_first_input,
                &mut retry_counter,
                args.io_args.retry_args,
            )?;
        }

        input_chunk =
            input_stream.buffer_with_retry(&mut retry_counter, args.io_args.retry_args)?;

        if input_chunk.is_empty() {
            if !decoder.is_finished() {
                return Err(CliOpError::UnexpectedEof);
            }

            break;
        }
    }

    if let Some(footer) = args.scope.footer() {
        output_stream.append_with_retry(footer, &mut retry_counter, args.io_args.retry_args)?;
    }

    output_stream.flush_with_retry(&mut retry_counter, args.io_args.retry_args)
}

/// Inner function of [`extract`].
fn extract_metadata(
    metadata: Option<Box<GameReplayMetadata>>,
    output_stream: &mut OutputBufWriter,
    read_inputs: bool,
    retry_counter: &mut u32,
    retry_args: RetryArguments,
) -> ControlFlow<Result<(), CliOpError>> {
    let Some(metadata) = metadata else {
        return ControlFlow::Continue(());
    };

    let serialized_metadata = match serde_json::to_string(&*metadata) {
        Ok(m) => m,
        Err(e) => {
            return ControlFlow::Break(Err(CliOpError::MetadataSerializeError {
                metadata: *metadata,
                inner: e,
            }));
        }
    };

    if let Err(e) =
        output_stream.append_with_retry(serialized_metadata.as_bytes(), retry_counter, retry_args)
    {
        return ControlFlow::Break(Err(e));
    }

    if read_inputs {
        // Cap off metadata section, start inputs section
        let res = output_stream.append_with_retry(br#","inputs":["#, retry_counter, retry_args);
        if let Err(e) = res {
            return ControlFlow::Break(Err(e));
        }
    } else {
        // We only care about metadata, we're done!
        return ControlFlow::Break(Ok(()));
    }

    ControlFlow::Continue(())
}

fn extract_inputs(
    inputs: &[GameInputEvent],
    output_stream: &mut OutputBufWriter,
    is_first_input: &mut bool,
    retry_counter: &mut u32,
    retry_args: RetryArguments,
) -> Result<(), CliOpError> {
    /// How big to make the buffer.
    const PREALLOC_BUFFER_SIZE: usize = br#"{"frame":9999,"type":1,"key":3},"#.len();

    let mut buf = Vec::with_capacity(PREALLOC_BUFFER_SIZE);

    for input in inputs {
        let input = *input;
        let unpacked = UnpackedInputEvent::from_packed(input);

        if !*is_first_input {
            buf.push(b',');
        }

        if let Err(e) = serde_json::to_writer(&mut buf, &unpacked) {
            return Err(CliOpError::InputSerializeError { input, inner: e });
        }

        output_stream.append_with_retry(&buf, retry_counter, retry_args)?;

        buf.clear();

        *is_first_input = false;
    }

    Ok(())
}
