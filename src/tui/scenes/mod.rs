use core::ops::ControlFlow;
use std::path::{Path, PathBuf};

use ratatui::{crossterm, prelude::Frame};

use crate::{
    backend::{BackendConnection, BackendResponse},
    tui::scenes::{
        event_inspect::EventInspectScene,
        explorer::{ExplorerScene, ExplorerTransition},
        meta_inspect::MetaInspectScene,
        operations::{OperationChoice, OperationsScene, OperationsTransition},
    },
};

mod common_inspect;
pub(in crate::tui) mod event_inspect;
pub(in crate::tui) mod explorer;
pub(in crate::tui) mod meta_inspect;
pub(in crate::tui) mod operations;

/// Represents the currently-displayed UI scene.
#[derive(Debug)]

pub(in crate::tui) enum Scene {
    /// The explorer scene, where the user can traverse directories and
    /// select a file.
    Explorer(ExplorerScene),
    /// The operations scene, where the user can select an operation to perform
    /// on that file.
    Operations(OperationsScene),
    /// The metadata inspection scene, where the user can inspect a selected
    /// replay's metadata
    MetadataInspect(MetaInspectScene),
    /// The inputdata inspection scene, where the user can inspect a selected
    /// replay's input event data
    EventDataInspect(EventInspectScene),
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
            Self::MetadataInspect(inner) => inner.render(frame),
            Self::EventDataInspect(inner) => inner.render(frame),
        }
    }

    /// Handle a crossterm event.
    pub(in crate::tui) fn handle_event(
        &mut self,
        event: crossterm::event::Event,
        terminal: &ratatui::DefaultTerminal,
        backend: &mut BackendConnection,
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
                Some(OperationsTransition::SelectOperation {
                    operation: OperationChoice::InspectEvents,
                    rep_path: path,
                }) => {
                    *self = Scene::EventDataInspect(EventInspectScene::new(path, backend));
                    ControlFlow::Continue(())
                }
                Some(OperationsTransition::SelectOperation {
                    operation: OperationChoice::InspectMetadata,
                    rep_path: path,
                }) => {
                    *self = Scene::MetadataInspect(MetaInspectScene::new(path, backend));
                    ControlFlow::Continue(())
                }
                Some(OperationsTransition::Explorer) => {
                    *self = Scene::Explorer(ExplorerScene::new(ret_path));
                    ControlFlow::Continue(())
                }
                Some(OperationsTransition::Quit) => ControlFlow::Break(()),
                None => ControlFlow::Continue(()),
            },
            Scene::MetadataInspect(scene) => {
                scene.handle_event(event);
                todo!("Handle event")
            }
            Scene::EventDataInspect(scene) => {
                scene.handle_event(event);
                todo!("Handle event")
            }
        }
    }

    /// Handle a reply from the backend.
    pub(in crate::tui) fn handle_response(&mut self, response: BackendResponse) {
        match self {
            Scene::Explorer(scene) => scene.handle_response(response),
            Scene::Operations(scene) => scene.handle_response(response),
            Scene::MetadataInspect(scene) => scene.handle_response(response),
            Scene::EventDataInspect(scene) => scene.handle_response(response),
        }
    }
}
