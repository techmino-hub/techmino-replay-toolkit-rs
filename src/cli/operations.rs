//! Handles specifically CLI operations and one-off commands.

use crate::cli::{
    clap::{CliOperation, CliReplayFormat},
    io::{ReadFileOrStdin, WriteFileOrStdout},
    types::{CliOpError, ExtractArguments, UnpackedInputEvent},
};
use libtechmino_replay::{ReplayBufferKind, deserialize::ReplayDecoder};

pub fn handle_cli_op(operation: &CliOperation) -> Result<(), CliOpError> {
    match operation {
        CliOperation::Extract {
            extract_mode,
            replay_format,
            override_input_mode,
            input_file,
            output_json,
            retry_args,
        } => extract(ExtractArguments {
            retry_args: *retry_args,
            extract_mode,
            replay_format: *replay_format,
            override_input_mode: *override_input_mode,
            input_file,
            output_json,
        }),
        CliOperation::Create {
            replay_format: _,
            input_json_file: _,
            output_file: _,
        } => todo!(),
        CliOperation::Base64ify {
            input_file: _,
            output_file: _,
        } => todo!(),
        CliOperation::Binaryify {
            input_file: _,
            output_file: _,
        } => todo!(),
    }
}

// TODO: Optimize: Skip unnecessary sections by stabilizing preprocessors
fn extract(args: ExtractArguments) -> Result<(), CliOpError> {
    let mut retry_counter = 0u32;

    eprintln!("> opening input stream...");
    let mut input_stream =
        ReadFileOrStdin::new(args.input_file, &mut retry_counter, args.retry_args)?;
    eprintln!("> opening output stream...");
    let mut output_stream =
        WriteFileOrStdout::new(args.output_json, &mut retry_counter, args.retry_args)?;

    eprintln!("> starting read from input stream...");

    let input_chunk = input_stream.advance_with_retry(&mut retry_counter, args.retry_args)?;
    let replay_kind = infer_replay_kind(args.replay_format, input_chunk)?;
    eprintln!("> replay kind: {replay_kind:?}");

    let mut decoder = ReplayDecoder::new(replay_kind, args.override_input_mode.map(Into::into));

    let (read_metadata, read_inputs) = args.extract_mode.to_keeps();

    if let Some(header) = args.extract_mode.header() {
        output_stream.append_with_retry(header, &mut retry_counter, args.retry_args)?;
    }

    loop {
        let res = decoder.update(input_chunk)?;

        if read_metadata && let Some(metadata) = res.metadata {
            let serialized_metadata = match serde_json::to_string(&*metadata) {
                Ok(m) => m,
                Err(e) => {
                    return Err(CliOpError::MetadataSerializeError {
                        metadata: *metadata,
                        inner: e,
                    });
                }
            };

            output_stream.append_with_retry(
                serialized_metadata.as_bytes(),
                &mut retry_counter,
                args.retry_args,
            )?;

            if read_inputs {
                // Cap off metadata section, start inputs section
                output_stream.append_with_retry(
                    br#","inputs":["#,
                    &mut retry_counter,
                    args.retry_args,
                )?;
            } else {
                // We only care about metadata, we're done!
                break;
            }
        }

        if read_inputs {
            /// How big to make the buffer.
            const PREALLOC_BUFFER_SIZE: usize = br#"{"frame":9999,"type":1,"key":3},"#.len();

            let mut buf = Vec::with_capacity(PREALLOC_BUFFER_SIZE);
            for input in res.inputs {
                let unpacked = UnpackedInputEvent::from_packed(input);

                if let Err(e) = serde_json::to_writer(&mut buf, &unpacked) {
                    return Err(CliOpError::InputSerializeError { input, inner: e });
                }
                buf.push(b',');

                output_stream.append_with_retry(&buf, &mut retry_counter, args.retry_args)?;

                buf.clear();
            }
        }
    }

    if let Some(footer) = args.extract_mode.footer() {
        output_stream.append_with_retry(footer, &mut retry_counter, args.retry_args)?;
    }

    output_stream.flush_with_retry(&mut retry_counter, args.retry_args)
}

/// Infers the replay kind from the first byte of the encoded replay.
fn infer_replay_kind(
    fmt_override: Option<CliReplayFormat>,
    first_chunk: &[u8],
) -> Result<ReplayBufferKind, CliOpError> {
    /// Zlib always begins with 0x78 (`x`): https://en.wikipedia.org/wiki/List_of_file_signatures
    const ZLIB_HEADER_FIRST_BYTE: u8 = b'x';
    /// 0x7800 until 0x78FF always starts with an `e` in base64
    const BASE64_ZLIB_FIRST_BYTE: u8 = b'e';
    /// Raw uncompressed game data begins with a JSON object, which begins with a `{`
    const UNCOMPRESSED_FIRST_BYTE: u8 = b'{';

    if let Some(format) = fmt_override {
        return Ok(format.into());
    }

    let first_byte = first_chunk.first().copied().ok_or(CliOpError::InputEmpty)?;

    match first_byte {
        ZLIB_HEADER_FIRST_BYTE => Ok(ReplayBufferKind::Compressed),
        BASE64_ZLIB_FIRST_BYTE => Ok(ReplayBufferKind::Base64),
        UNCOMPRESSED_FIRST_BYTE => Ok(ReplayBufferKind::Uncompressed),
        _ => Err(CliOpError::ReplayKindInferFailed { first_byte }),
    }
}
