//! Handles the `shrink` operation.

use crate::cli::{clap::ShrinkArguments, types::CliOpError};

pub(super) fn shrink(args: &ShrinkArguments) -> Result<(), CliOpError> {
    _ = args;
    todo!();
}
