use core::ops::ControlFlow;
use std::path::{Path, PathBuf};

use ratatui::{crossterm, prelude::Frame};

use crate::{
    backend::{BackendConnection, BackendReply},
    tui::scenes::{
        explorer::{ExplorerScene, ExplorerTransition},
        operations::{OperationsScene, OperationsTransition},
    },
};

pub(in crate::tui) mod explorer;
pub(in crate::tui) mod operations;

/// Represents the currently-displayed UI scene.
#[derive(Debug)]

pub(in crate::tui) enum Scene {
    /// The explorer scene, where the user can traverse directories and
    /// select a file.
    Explorer(ExplorerScene),
    /// The operations scene, where the user can select an operation to perform on
    /// that file.
    Operations(OperationsScene),
}

impl Scene {
    /// Initializes the UI scene given a certain initialization path.
    ///
    /// If the path points to a file, initializes at the operations scene.
    /// Otherwise, initializes at the explorer scene.
    pub(in crate::tui) fn new(path: &Path) -> Self {
        if path.is_file() {
            Self::Operations(OperationsScene::new(path.to_owned()))
        } else {
            Self::Explorer(ExplorerScene::new(path))
        }
    }

    /// Render this scene to the given frame.
    pub(in crate::tui) fn render(&self, frame: &mut Frame) {
        match self {
            Self::Explorer(inner) => inner.render(frame),
            Self::Operations(inner) => inner.render(frame),
        }
    }

    /// Handle a crossterm event.
    pub(in crate::tui) fn handle_event(
        &mut self,
        event: crossterm::event::Event,
        terminal: &ratatui::DefaultTerminal,
        _backend: &mut BackendConnection,
        ret_path: &mut PathBuf,
    ) -> ControlFlow<()> {
        match self {
            Scene::Explorer(scene) => match scene.handle_event(event, terminal, ret_path) {
                Some(ExplorerTransition::OperationsScene { file_path }) => {
                    *self = Scene::Operations(OperationsScene::new(file_path));
                    ControlFlow::Continue(())
                }
                Some(ExplorerTransition::Quit) => ControlFlow::Break(()),
                None => ControlFlow::Continue(()),
            },
            Scene::Operations(scene) => match scene.handle_event(event) {
                Some(OperationsTransition::SelectOperation(o)) => {
                    todo!("Go to specific operation scene: {o}")
                }
                Some(OperationsTransition::Explorer) => {
                    *self = Scene::Explorer(ExplorerScene::new(ret_path));
                    ControlFlow::Continue(())
                }
                Some(OperationsTransition::Quit) => ControlFlow::Break(()),
                None => ControlFlow::Continue(()),
            },
        }
    }

    /// Handle a reply from the backend.
    pub(in crate::tui) fn handle_reply(&mut self, reply: BackendReply) {
        match self {
            Scene::Explorer(explorer_scene) => explorer_scene.handle_reply(reply),
            Scene::Operations(operations_scene) => operations_scene.handle_reply(reply),
        }
    }
}
