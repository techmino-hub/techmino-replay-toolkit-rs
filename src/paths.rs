//! Module for file paths, especially those made by Techmino.
//!
//! # Terminology
//! - Data directory: The data directory as designated by the `dirs` crate.
//! - Save directory: Techmino's save directory, containing the `replay` folder
//!   etc.
//! - Replay directory: The `replay` folder inside Techmino's save directory.

use std::path::PathBuf;

/// The path to Techmino's save directory **relative to the data directory**.
const SAVE_RELPATH: Option<&str> = {
    if cfg!(windows) {
        Some("Techmino")
    } else if cfg!(target_os = "macos") {
        Some("LOVE/Techmino")
    } else if cfg!(target_os = "linux") {
        Some("love/Techmino")
    } else {
        None
    }
};

/// The alternate path to Techmino's save directory **relative to the data
/// directory**, if there is an alternate path.
const ALT_SAVE_RELPATH: Option<&str> = if cfg!(windows) {
    Some(r"LOVE\Techmino")
} else {
    None
};

/// The path to Techmino's replay directory **relative to the save directory**.
const REPLAY_RELPATH: &str = "replay";

/// An abstraction over Techmino's save directory.
#[derive(Clone, Debug)]
pub(crate) struct TechminoSaveDir {
    path: PathBuf,
}

impl TechminoSaveDir {
    /// Try to get this save directory if it exists.
    ///
    /// Returns `None` if this save directory doesn't exist in any of the
    /// expected/checked paths.
    pub fn new() -> Option<Self> {
        let save_relpath = SAVE_RELPATH?;
        let data_dir = dirs::data_dir()?;

        let main_path = data_dir.join(save_relpath);

        if main_path.exists() {
            return Some(Self { path: main_path });
        }

        let alt_relpath = ALT_SAVE_RELPATH?;
        let alt_path = data_dir.join(alt_relpath);

        if alt_path.exists() {
            return Some(Self { path: main_path });
        }

        None
    }

    /// Try to get Techmino's replay directory if it exists.
    ///
    /// Returns `None` if it doesn't.
    pub fn get_replay_dir(&self) -> Option<TechminoReplayDir> {
        let path = self.path.join(REPLAY_RELPATH);

        if path.exists() {
            return Some(TechminoReplayDir { path });
        }

        None
    }
}

/// An abstraction over Techmino's replay directory.
#[derive(Clone, Debug)]
pub(crate) struct TechminoReplayDir {
    path: PathBuf,
}

/// Gets the initial path to start the TUI in, if not overridden.
pub fn get_initial_path() -> PathBuf {
    let Some(dir) = TechminoSaveDir::new() else {
        return get_fallback_start_path();
    };

    dir.get_replay_dir().map(|d| d.path).unwrap_or(dir.path)
}

/// Gets a fallback initial path for when the Techmino replay directory doesn't
/// exist or is inaccessible.
fn get_fallback_start_path() -> PathBuf {
    if let Ok(dir) = std::env::current_dir() {
        return dir;
    }

    if let Some(dir) = dirs::home_dir()
        && dir.exists()
    {
        return dir;
    }

    PathBuf::from("/")
}
