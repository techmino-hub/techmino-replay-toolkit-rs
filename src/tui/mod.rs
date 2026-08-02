//! A terminal user interface powered by Ratatui.

use std::path::PathBuf;

use crate::{cli::clap::TuiArguments, paths};

pub fn start(args: TuiArguments) {
    let init_path = args.get_path();

    dbg!(init_path);

    todo!("the rest of the logic");
}

impl TuiArguments {
    /// Gets the path if specified, or uses a fallback.
    fn get_path(&self) -> PathBuf {
        self.path.clone().unwrap_or_else(paths::get_initial_path)
    }
}
