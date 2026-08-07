use std::path::{Path, PathBuf};

use ratatui::{crossterm, prelude::Frame};

use crate::{
    backend::{BackendConnection, BackendReply},
    tui::scenes::explorer::ExplorerScene,
};

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

    /// Render this scene to the given frame.
    pub(in crate::tui) fn render(&self, frame: &mut Frame) {
        match self {
            Self::Explorer(inner) => inner.render(frame),
            _ => todo!("Rendering not implemented yet for this state: {self:?}"),
        }
    }

    /// Handle a crossterm event.
    pub(in crate::tui) fn handle_event(
        &mut self,
        event: crossterm::event::Event,
        backend: &mut BackendConnection,
        ret_path: &mut PathBuf,
    ) {
        match self {
            Scene::Explorer(explorer_scene) => {
                if let Some(go_to_ops) = explorer_scene.handle_event(event, ret_path) {
                    todo!("Go to operations scene");
                }
            }
            _ => todo!("Handle event for other scenes"),
        }
    }

    /// Handle a reply from the backend.
    pub(in crate::tui) fn handle_reply(&mut self, reply: BackendReply) {
        match self {
            Scene::Explorer(explorer_scene) => explorer_scene.handle_reply(reply),
            _ => todo!("Handle reply for other scenes"),
        }
    }
}
