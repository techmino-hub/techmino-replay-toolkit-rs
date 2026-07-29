//! Handles the `binaryify` operation.

use core::cell::RefCell;
use std::{
    collections::VecDeque,
    io::{
        BufReader, IsTerminal as _,
        prelude::{Read, Write},
    },
    rc::Rc,
};

use base64::read::DecoderReader;

use crate::cli::{clap::BinaryifyArguments, io::buffered::OutputBufWriter, types::CliOpError};

pub(super) fn binaryify(args: &BinaryifyArguments) -> Result<(), CliOpError> {
    const SCRATCH_RING_BUF_SIZE: usize = 8192;
    const COPY_BUF_SIZE: usize = SCRATCH_RING_BUF_SIZE * 3 / 4;

    let mut retry_counter = 0u32;

    let (mut input_stream, mut output_stream) = args.io_args.get_buffered(&mut retry_counter)?;

    if !args.skip_console_check
        && matches!(output_stream, OutputBufWriter::Stdout { .. })
        && std::io::stdout().is_terminal()
    {
        return Err(CliOpError::BinaryConsoleOutput);
    }

    let scratch_ring_buf = RefCellDeque::new();
    let mut decoder = DecoderReader::new(
        scratch_ring_buf.clone(),
        &base64::engine::general_purpose::STANDARD,
    );

    loop {
        let input = input_stream.buffer_with_retry(&mut retry_counter, args.io_args.retry_args)?;
        if input.is_empty() {
            break;
        }

        let input_len = input.len();
        scratch_ring_buf
            .inner()
            .borrow_mut()
            .write_all(input)
            .map_err(|e| CliOpError::InputReadError { inner: e })?;
        input_stream.consume(input_len);

        output_stream.copy_with_retry::<COPY_BUF_SIZE>(
            &mut decoder,
            &mut retry_counter,
            args.io_args.retry_args,
        )?;
    }

    Ok(())
}

/// An `Rc<RefCell<VecDeque>>`-based in-memory `Read` buffer.
#[derive(Clone)]
struct RefCellDeque {
    inner: Rc<RefCell<VecDeque<u8>>>,
}

impl RefCellDeque {
    pub fn new() -> Self {
        let capacity = BufReader::new(std::io::empty()).capacity();

        let deque = VecDeque::with_capacity(capacity);

        Self {
            inner: Rc::new(RefCell::new(deque)),
        }
    }

    pub fn inner(&self) -> &Rc<RefCell<VecDeque<u8>>> {
        &self.inner
    }
}

impl Read for RefCellDeque {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.borrow_mut().read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [std::io::IoSliceMut<'_>]) -> std::io::Result<usize> {
        self.inner.borrow_mut().read_vectored(bufs)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        self.inner.borrow_mut().read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        self.inner.borrow_mut().read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.inner.borrow_mut().read_exact(buf)
    }
}
