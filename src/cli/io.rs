use crate::cli::{clap::RetryArguments, types::CliOpError};
use core::time::Duration;
use std::{
    fs::{File, OpenOptions},
    io::{
        self,
        prelude::{BufRead, Write},
        BufReader, BufWriter, ErrorKind, Stdin, Stdout,
    },
    path::{Path, PathBuf},
};

/// Either an open `File` in read-only mode, or stdin.
pub(super) enum ReadFileOrStdin {
    File { file: BufReader<File> },
    Stdin { stdin: BufReader<Stdin> },
}

impl ReadFileOrStdin {
    /// Returns either an `impl Read` of the file at the input file path, or stdin if it's `None`.
    ///
    /// Retries according to the retry arguments.
    ///
    /// # Errors
    /// Errors if there was an error trying to open the file.
    pub(super) fn new(
        input_file: &Option<PathBuf>,
        retry_counter: &mut u32,
        retry_args: RetryArguments,
    ) -> Result<Self, CliOpError> {
        if let Some(path) = input_file {
            Self::new_file(path, retry_counter, retry_args)
        } else {
            let stdin = BufReader::new(io::stdin());

            Ok(Self::Stdin { stdin })
        }
    }

    /// Inner part of `new`.
    fn new_file(
        input_file: &Path,
        retry_counter: &mut u32,
        retry_args: RetryArguments,
    ) -> Result<Self, CliOpError> {
        let file = loop {
            let e = match File::open(input_file).map(BufReader::new) {
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

    /// Read the next chunk of bytes, retrying according to the retry arguments.
    ///
    /// Returns bytes from the source.
    ///
    /// This does not advance the cursor! This is meant to be used with
    /// [`Self::consume`].
    ///
    /// # Errors
    /// See [`std::io::Error`] for more information.
    pub(super) fn buffer_with_retry(
        &mut self,
        retry_counter: &mut u32,
        retry_args: RetryArguments,
    ) -> Result<&[u8], CliOpError> {
        loop {
            let res = self.fill_buf();
            let e = match res {
                Ok(_) => {
                    return Ok(self.buffer());
                }
                Err(e) => e,
            };

            if !retry_args.retry_all_io && e.kind() != ErrorKind::Interrupted {
                return Err(CliOpError::InputReadError { inner: e });
            }

            if retry_args.max_retries.is_at_limit(*retry_counter) {
                return Err(CliOpError::InputReadError { inner: e });
            }

            *retry_counter = retry_counter.wrapping_add(1);

            eprintln!(
                "Error reading from input stream: {e}. Waiting {retry_delay_ms} ms before retrying (attempt {retry_counter} of {max_retries}).",
                retry_delay_ms = retry_args.retry_delay_ms,
                max_retries = retry_args.max_retries
            );

            std::thread::sleep(Duration::from_millis(retry_args.retry_delay_ms));
            continue;
        }
    }

    /// Calls the internal [`BufReader::buffer`] function.
    fn buffer(&self) -> &[u8] {
        match self {
            Self::File { file } => file.buffer(),
            Self::Stdin { stdin } => stdin.buffer(),
        }
    }

    /// Calls the internal [`BufRead::fill_buf`] function.
    ///
    /// # Errors
    /// See [`BufRead::fill_buf`] for details.
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        match self {
            Self::File { file } => file.fill_buf(),
            Self::Stdin { stdin } => stdin.fill_buf(),
        }
    }

    /// Calls the internal [`BufRead::consume`] function.
    pub(super) fn consume(&mut self, amount: usize) {
        match self {
            Self::File { file } => file.consume(amount),
            Self::Stdin { stdin } => stdin.consume(amount),
        }
    }
}

pub(super) enum WriteFileOrStdout {
    File { file: BufWriter<File> },
    Stdout { stdout: BufWriter<Stdout> },
}

impl WriteFileOrStdout {
    /// Returns either an `impl Write` of the file at the output file path, or stdout if it's `None`.
    ///
    /// # Errors
    /// Returns an error if there was an error trying to open the file.
    pub(super) fn new(
        output_file: &Option<PathBuf>,
        retry_counter: &mut u32,
        retry_args: RetryArguments,
    ) -> Result<Self, CliOpError> {
        if let Some(path) = output_file {
            Self::new_file(path, retry_counter, retry_args)
        } else {
            let stdout = BufWriter::new(io::stdout());
            Ok(WriteFileOrStdout::Stdout { stdout })
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
            o.create(true).truncate(true).append(true).write(true);
            o
        };

        let file = loop {
            let e = match options.open(output_file) {
                Ok(f) => break BufWriter::new(f),
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

        Ok(WriteFileOrStdout::File { file })
    }

    /// Appends the entire buffer, retrying according to the retry
    /// arguments.
    ///
    /// # Errors
    /// See [`std::io::Error`] for more information.
    pub(super) fn append_with_retry(
        &mut self,
        buf: &[u8],
        retry_counter: &mut u32,
        retry_args: RetryArguments,
    ) -> Result<(), CliOpError> {
        while let Err(e) = self.write_all(buf) {
            if !retry_args.retry_all_io && e.kind() != ErrorKind::Interrupted {
                return Err(CliOpError::OutputWriteError { inner: e });
            }

            if retry_args.max_retries.is_at_limit(*retry_counter) {
                return Err(CliOpError::OutputWriteError { inner: e });
            }

            *retry_counter = retry_counter.wrapping_add(1);

            eprintln!(
                "Error writing into output stream: {e}. Waiting {retry_delay_ms} ms before retrying (attempt {retry_counter} of {max_retries}).",
                retry_delay_ms = retry_args.retry_delay_ms,
                max_retries = retry_args.max_retries
            );
            std::thread::sleep(Duration::from_millis(retry_args.retry_delay_ms));
            continue;
        }

        Ok(())
    }

    /// Flushes the buffer, retrying according to the retry arguments.
    pub(super) fn flush_with_retry(
        &mut self,
        retry_counter: &mut u32,
        retry_args: RetryArguments,
    ) -> Result<(), CliOpError> {
        while let Err(e) = self.flush() {
            if !retry_args.retry_all_io && e.kind() != ErrorKind::Interrupted {
                return Err(CliOpError::OutputFlushError { inner: e });
            }

            if retry_args.max_retries.is_at_limit(*retry_counter) {
                return Err(CliOpError::OutputFlushError { inner: e });
            }

            *retry_counter = retry_counter.wrapping_add(1);

            eprintln!(
                "Error flushing output stream: {e}. Waiting {retry_delay_ms} ms before retrying (attempt {retry_counter} of {max_retries}).",
                retry_delay_ms = retry_args.retry_delay_ms,
                max_retries = retry_args.max_retries
            );
            std::thread::sleep(Duration::from_millis(retry_args.retry_delay_ms));
            continue;
        }

        Ok(())
    }

    /// Calls the internal [`Write::flush`] function.
    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::File { file } => file.flush(),
            Self::Stdout { stdout } => stdout.flush(),
        }
    }

    /// Calls the internal [`Write::write_all`] function.
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        match self {
            Self::File { file } => file.write_all(buf),
            Self::Stdout { stdout } => stdout.write_all(buf),
        }
    }
}
