use crate::{
    ParseOrIoError,
    backend::{BackendConnection, BackendRequest, BackendResponse},
    consts::backend::EMSG_BACKEND_CONNECTION_BROKE,
};
use core::time::Duration;
use libtechmino_replay::replay::GameInputEvent;
use ratatui::{crossterm, prelude::Frame};
use std::{io, path::PathBuf, time::Instant};

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
        /// The amount of time that has passed between the request and the
        /// error.
        processed_for: Duration,
    },
}

impl EventInspectScene {
    pub(in crate::tui) fn new(path: PathBuf, backend: &BackendConnection) -> Self {
        let request_id: u64 = rand::random();

        let request = BackendRequest::FetchEventData {
            path: path.clone(),
            request_id,
        };

        let Ok(()) = backend.tx.send(request) else {
            return Self::Failed {
                error: io::Error::new(io::ErrorKind::BrokenPipe, EMSG_BACKEND_CONNECTION_BROKE)
                    .into(),
                processed_for: Duration::ZERO,
            };
        };

        Self::Loading {
            path,
            start_time: Instant::now(),
            request_id,
        }
    }

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
