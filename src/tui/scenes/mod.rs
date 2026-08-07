use std::path::{Path, PathBuf};

use ratatui::{crossterm, prelude::Frame};

use crate::{backend::BackendConnection, tui::scenes::explorer::ExplorerScene};

pub(in crate::tui) mod explorer;

/// Represents the currently-displayed UI scene.
#[derive(Debug)]
pub(in crate::tui) enum Scene {
    /// The explorer scene, where the user can traverse directories and
    /// select a file.
    Explorer(ExplorerScene),
    /// The operations scene, where the user can select an operation to perform on
    /// that file.
    Operations(core::convert::Infallible),
}

impl Scene {
    /// Initializes the UI scene given a certain initialization path.
    ///
    /// If the path points to a file, initializes at the operations scene.
    /// Otherwise, initializes at the explorer scene.
    pub(in crate::tui) fn new(path: &Path) -> Self {
        if path.is_dir() {
            Self::Explorer(ExplorerScene::new(path))
        } else {
            Self::Operations(todo!())
        }
    }

    pub(in crate::tui) fn render(&self, frame: &mut Frame) {
        match self {
            Self::Explorer(inner) => inner.render(frame),
            _ => todo!("Rendering not implemented yet for this state: {self:?}"),
        }
    }

    pub(in crate::tui) fn handle_event(
        &mut self,
        event: crossterm::event::Event,
        backend: &mut BackendConnection,
        ret_path: &mut PathBuf,
    ) {
        todo!("Handle event: {event:?}");
    }
}
