//! Handles the `base64ify` operation.

use core::cell::RefCell;
use std::{
    io::{self, prelude::Write, BufWriter},
    rc::Rc,
};

use crate::cli::{clap::Base64ifyArguments, types::CliOpError};

pub(super) fn base64ify(args: &Base64ifyArguments) -> Result<(), CliOpError> {
    let mut retry_counter = 0u32;

    let (mut input_stream, mut output_stream) = args.io_args.get_rw(&mut retry_counter)?;

    let encode_buffer = RefCellBuffer::new();
    let mut encoder = base64::write::EncoderWriter::new(
        encode_buffer.clone(),
        &base64::engine::general_purpose::STANDARD,
    );

    loop {
        let input = input_stream.buffer_with_retry(&mut retry_counter, args.io_args.retry_args)?;

        if input.is_empty() {
            break;
        }

        encoder
            .write_all(input)
            .expect("writing into Vec should never fail");

        output_stream.append_with_retry(
            &encode_buffer.inner().borrow(),
            &mut retry_counter,
            args.io_args.retry_args,
        )?;
        encode_buffer.inner().borrow_mut().clear();

        let input_len = input.len();
        input_stream.consume(input_len);
    }

    encoder
        .finish()
        .map_err(|e| CliOpError::OutputWriteError { inner: e })?;
    output_stream.append_with_retry(
        &encode_buffer.inner().borrow(),
        &mut retry_counter,
        args.io_args.retry_args,
    )?;

    output_stream.flush_with_retry(&mut retry_counter, args.io_args.retry_args)?;

    Ok(())
}

/// An `Rc<RefCell>`-based in-memory buffer that implements `Write`.
#[derive(Clone)]
struct RefCellBuffer {
    inner: Rc<RefCell<Vec<u8>>>,
}

impl RefCellBuffer {
    pub fn new() -> Self {
        let (_, res) = BufWriter::new(std::io::sink()).into_parts();
        let default_buf_vec = res.expect("uninitialized bufwriter should not have panicked");

        Self {
            inner: Rc::new(RefCell::new(default_buf_vec)),
        }
    }

    pub fn inner(&self) -> &Rc<RefCell<Vec<u8>>> {
        &self.inner
    }
}

impl Write for RefCellBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner()
            .try_borrow_mut()
            .map_err(io::Error::other)?
            .write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner()
            .try_borrow_mut()
            .map_err(io::Error::other)?
            .flush()
    }

    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
        self.inner()
            .try_borrow_mut()
            .map_err(io::Error::other)?
            .write_vectored(bufs)
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.inner()
            .try_borrow_mut()
            .map_err(io::Error::other)?
            .write_all(buf)
    }

    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> std::io::Result<()> {
        self.inner()
            .try_borrow_mut()
            .map_err(io::Error::other)?
            .write_fmt(args)
    }
}
