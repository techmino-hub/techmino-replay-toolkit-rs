use crate::{ParseOrIoError, backend::BackendResponse};
use libtechmino_replay::replay::GameInputEvent;
use ratatui::{crossterm, prelude::Frame};
use std::{path::PathBuf, time::Instant};

/// The input event data inspection scene, where the user can inspect a selected
/// replay's input event data.
#[derive(Debug)]
pub(in crate::tui) enum EventInspectScene {
    /// Awaiting a reply from the backend.
    Loading {
        /// The path of the replay being inspected.
        path: PathBuf,
        /// The time at which the operation was invoked.
        start_time: Instant,
        /// The request ID for this invocation.
        request_id: u64,
    },
    /// The backend has replied with the input event data.
    Done {
        /// The retrieved input event data.
        metadata: Vec<GameInputEvent>,
    },
    /// The backend has replied with an error.
    Failed {
        /// The error from the backend.
        error: ParseOrIoError,
    },
}

impl EventInspectScene {
    pub(in crate::tui) fn render(&self, frame: &mut Frame) {
        todo!("Render scene: {self:?} | {frame:?}");
    }

    pub(in crate::tui) fn handle_event(&self, ev: crossterm::event::Event) {
        todo!("Handle event: {self:?} | {ev:?}");
    }

    pub(in crate::tui) fn handle_response(&self, response: BackendResponse) {
        todo!("Handle response: {self:?} | {response:?}");
    }
}
