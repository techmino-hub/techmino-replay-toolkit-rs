//! A terminal user interface powered by Ratatui.

use core::num::NonZeroI32;

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
