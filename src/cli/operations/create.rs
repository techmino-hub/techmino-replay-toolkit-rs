//! Handles the `create` operation.

use crate::cli::{clap::CreateArguments, types::CliOpError};

pub(super) fn create(args: &CreateArguments) -> Result<(), CliOpError> {
    _ = args;
    todo!();
}
