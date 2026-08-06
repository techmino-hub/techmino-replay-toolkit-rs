use std::path::Path;

use crate::tui::ui_state::explorer::ExplorerState;

pub(in crate::tui) mod explorer;

/// Represents the state of the interface.
#[derive(Debug)]
pub(in crate::tui) enum UiState {
    /// The explorer menu, where the user can traverse directories and
    /// select a file.
    Explorer(ExplorerState),
    /// The operations menu, where the user can select an operation to perform on
    /// that file.
    Operations(core::convert::Infallible),
}

impl UiState {
    /// Initializes the UI state given a certain initialization path.
    ///
    /// If the path points to a file, initializes at the operations menu.
    /// Otherwise, initializes at the explorer menu.
    pub fn new(path: &Path) -> Self {
        if path.is_dir() {
            Self::Explorer(ExplorerState::new(path))
        } else {
            Self::Operations(todo!())
        }
    }
}
