use crate::cli::{
    clap::IoArguments,
    io::{
        buffered::{InputBufReader, OutputBufWriter},
        unbuffered::{InputReader, OutputWriter},
    },
    types::CliOpError,
};

pub(in crate::cli) mod buffered;
pub(in crate::cli) mod unbuffered;

impl IoArguments {
    /// Get input/output buffered structs based on this I/O argument.
    pub(super) fn get_buffered(
        &self,
        retry_counter: &mut u32,
    ) -> Result<(InputBufReader, OutputBufWriter), CliOpError> {
        let input_stream = InputBufReader::new(&self.input_file, retry_counter, self.retry_args)?;
        let output_stream =
            OutputBufWriter::new(&self.output_file, retry_counter, self.retry_args)?;

        Ok((input_stream, output_stream))
    }

    /// Get input/output unbuffered structs based on this I/O argument.
    pub(super) fn get_unbuffered(
        &self,
        retry_counter: &mut u32,
    ) -> Result<(InputReader, OutputWriter), CliOpError> {
        let input_stream = InputReader::new(&self.input_file, retry_counter, self.retry_args)?;
        let output_stream = OutputWriter::new(&self.output_file, retry_counter, self.retry_args)?;

        Ok((input_stream, output_stream))
    }
}
