//! A terminal user interface powered by Ratatui.

use core::num::NonZeroI32;
use std::io;

use libtechmino_replay::ReplayParseError;

use crate::{backend::BackendState, cli::clap::TuiArguments};

mod frontend;
mod ui_state;

/// Starts a TUI instance.
///
/// # Errors
/// On error, returns the desired non-zero exit code of the process.
pub fn start(args: TuiArguments) -> Result<(), NonZeroI32> {
    let conn = match BackendState::spawn() {
        Ok(bh) => bh.connection,
        Err(e) => {
            eprintln!("Error: Failed to start processing backend thread: {e}");
            return Err(NonZeroI32::MIN);
        }
    };

    let mut frontend = match frontend::AppFrontend::new(args, conn) {
        Ok(fe) => fe,
        Err(e) => {
            eprintln!("Error: Failed to create frontend: {e}");
            return Err(NonZeroI32::MIN);
        }
    };

    ratatui::run(move |terminal| frontend.run(terminal));

    Ok(())
}

/// Either a replay-parsing or I/O error.
#[derive(Debug, thiserror::Error)]
enum ParseOrIoError {
    #[error("Failed to parse replay file")]
    Parse(#[from] ReplayParseError),
    #[error("Failed to read from replay file")]
    Io(#[from] io::Error),
}
