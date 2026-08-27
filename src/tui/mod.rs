//! A terminal user interface powered by Ratatui.

use crate::{backend::BackendState, cli::clap::TuiArguments};
use core::num::NonZeroI32;

mod event;
mod frontend;
mod scenes;
mod ui;

/// Starts a TUI instance.
///
/// # Errors
/// On error, returns the desired non-zero exit code of the process.
pub fn start(args: TuiArguments) -> Result<(), NonZeroI32> {
    let conn = match BackendState::spawn() {
        Ok(c) => c,
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

    let res = ratatui::run(move |terminal| frontend.run(terminal));

    if let Err(e) = res {
        eprintln!("Run-time error: {e}");
        return Err(NonZeroI32::MIN);
    }

    Ok(())
}
