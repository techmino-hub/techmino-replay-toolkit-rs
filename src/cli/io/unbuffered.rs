//! # Unbuffered I/O
//!
//! This is an abstraction for unbuffered I/O operations for the CLI.

use core::time::Duration;
use std::{
    fs::{File, OpenOptions},
    io::{self, BufWriter, ErrorKind, IsTerminal, Stdin, Stdout, prelude::Read},
    path::{Path, PathBuf},
};

use crate::cli::{clap::RetryArguments, io::buffered::OutputBufWriter, types::CliOpError};

/// Either an open `File` in read-only mode, or stdin. Unbuffered.
pub(in crate::cli) enum InputReader {
    File { file: File },
    Stdin { stdin: Stdin },
}

impl InputReader {
    pub(in crate::cli) fn new(
        input_path: &Option<PathBuf>,
        retry_counter: &mut u32,
        retry_args: RetryArguments,
    ) -> Result<Self, CliOpError> {
        if let Some(path) = input_path {
            Self::new_file(path, retry_counter, retry_args)
        } else {
            let stdin = std::io::stdin();
            Ok(Self::Stdin { stdin })
        }
    }

    fn new_file(
        input_file: &Path,
        retry_counter: &mut u32,
        retry_args: RetryArguments,
    ) -> Result<Self, CliOpError> {
        let file = loop {
            let e = match File::open(input_file) {
                Ok(f) => break f,
                Err(e) => e,
            };

            if !retry_args.retry_all_io && e.kind() == ErrorKind::Interrupted {
                return Err(CliOpError::OutputStreamOpenError {
                    inner: e,
                    path: input_file.to_owned(),
                });
            }

            if retry_args.max_retries.is_at_limit(*retry_counter) {
                return Err(CliOpError::OutputStreamOpenError {
                    inner: e,
                    path: input_file.to_owned(),
                });
            }

            *retry_counter = retry_counter.wrapping_add(1);

            eprintln!(
                "Error opening input stream: {e}. Waiting {retry_delay_ms} ms before retrying (attempt {retry_counter} of {max_retries}).",
                retry_delay_ms = retry_args.retry_delay_ms,
                max_retries = retry_args.max_retries
            );

            std::thread::sleep(Duration::from_millis(retry_args.retry_delay_ms));
            continue;
        };

        Ok(Self::File { file })
    }

    /// Gets the size of the file in bytes, if it's possible to get.
    pub(in crate::cli) fn len(&self) -> Option<u64> {
        match self {
            Self::File { file } => file.metadata().ok().map(|m| m.len()),
            Self::Stdin { .. } => None,
        }
    }

    /// Gets the suggested preallocated buffer for this input.
    pub(in crate::cli) fn buf_size(&self) -> usize {
        /// The guessed size of an average replay.
        ///
        /// Used for setting initial capacities.
        const REPLAY_SIZE_GUESS: usize = 16384;

        self.len()
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or(REPLAY_SIZE_GUESS)
    }

    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::File { file } => file.read(buf),
            Self::Stdin { stdin } => stdin.read(buf),
        }
    }

    /// Read the next chunk of bytes, retrying according to the retry arguments.
    ///
    /// Returns the amount of bytes read to the passed-in buffer.
    pub(in crate::cli) fn read_with_retry(
        &mut self,
        buf: &mut [u8],
        retry_counter: &mut u32,
        retry_args: RetryArguments,
    ) -> Result<usize, CliOpError> {
        loop {
            let e = match self.read(buf) {
                Ok(n) => break Ok(n),
                Err(e) => e,
            };

            if !retry_args.retry_all_io && e.kind() != ErrorKind::Interrupted {
                return Err(CliOpError::InputReadError { inner: e });
            }

            if retry_args.max_retries.is_at_limit(*retry_counter) {
                return Err(CliOpError::InputReadError { inner: e });
            }

            eprintln!(
                "Error reading from input stream: {e}. Waiting {retry_delay_ms} ms before retrying (attempt {retry_counter} of {max_retries}).",
                retry_delay_ms = retry_args.retry_delay_ms,
                max_retries = retry_args.max_retries
            );

            std::thread::sleep(Duration::from_millis(retry_args.retry_delay_ms));
            continue;
        }
    }
}

/// Either an open `File` in write-only mode, or stdout. Unbuffered.
pub(in crate::cli) enum OutputWriter {
    File { file: File },
    Stdout { stdout: Stdout },
}

impl OutputWriter {
    /// Returns either a writer of the file at the output file path, or stdout
    /// if it's `None`.
    ///
    /// # Errors
    /// Returns an error if there was an error trying to open the file.
    pub(in crate::cli) fn new(
        output_file: &Option<PathBuf>,
        retry_counter: &mut u32,
        retry_args: RetryArguments,
    ) -> Result<Self, CliOpError> {
        if let Some(path) = output_file {
            Self::new_file(path, retry_counter, retry_args)
        } else {
            let stdout = io::stdout();
            Ok(OutputWriter::Stdout { stdout })
        }
    }

    /// Inner part of `new`.
    fn new_file(
        output_file: &Path,
        retry_counter: &mut u32,
        retry_args: RetryArguments,
    ) -> Result<Self, CliOpError> {
        let options = {
            let mut o = OpenOptions::new();
            o.write(true).create(true).truncate(true);
            o
        };

        let file = loop {
            let e = match options.open(output_file) {
                Ok(f) => break f,
                Err(e) => e,
            };

            if !retry_args.retry_all_io && e.kind() != ErrorKind::Interrupted {
                return Err(CliOpError::InputStreamOpenError {
                    inner: e,
                    path: output_file.to_owned(),
                });
            }

            if retry_args.max_retries.is_at_limit(*retry_counter) {
                return Err(CliOpError::InputStreamOpenError {
                    inner: e,
                    path: output_file.to_owned(),
                });
            }

            *retry_counter = retry_counter.wrapping_add(1);

            eprintln!(
                "Error opening output stream: {e}. Waiting {retry_delay_ms} ms before retrying (attempt {retry_counter} of {max_retries}).",
                retry_delay_ms = retry_args.retry_delay_ms,
                max_retries = retry_args.max_retries
            );

            std::thread::sleep(Duration::from_millis(retry_args.retry_delay_ms));
            continue;
        };

        Ok(OutputWriter::File { file })
    }

    /// Converts this unbuffered writer into a buffered writer.
    pub(in crate::cli) fn into_buffered(self) -> OutputBufWriter {
        match self {
            OutputWriter::File { file } => OutputBufWriter::File {
                file: BufWriter::new(file),
            },
            OutputWriter::Stdout { stdout } => OutputBufWriter::Stdout {
                stdout: BufWriter::new(stdout),
            },
        }
    }

    /// Returns whether or not this is a terminal.
    ///
    /// See [`std::io::IsTerminal`] for more details.
    pub(in crate::cli) fn is_terminal(&self) -> bool {
        match self {
            OutputWriter::File { file } => file.is_terminal(),
            OutputWriter::Stdout { stdout } => stdout.is_terminal(),
        }
    }
}
