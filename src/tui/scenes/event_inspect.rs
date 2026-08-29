use crate::{
    ParseOrIoError,
    backend::{BackendConnection, BackendRequest, BackendResponse},
    consts::{backend::EMSG_BACKEND_CONNECTION_BROKE, tui::OP_EVEINSP_LOADING_MSG},
    tui::scenes::common_inspect::{render_error, render_loading},
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
    Loading(LoadingState),
    /// The backend has replied with the input event data.
    Done {
        /// The path of the replay being inspected.
        path: PathBuf,
        /// The retrieved input event data.
        inputs: Vec<GameInputEvent>,
    },
    /// The backend has replied with an error.
    Failed {
        /// The path of the replay that tripped the backend.
        path: PathBuf,
        /// The error from the backend.
        error: ParseOrIoError,
        /// The amount of time that has passed between the request and the
        /// error.
        processed_for: Duration,
    },
}

/// The state used for the `Loading` state in the scene.
#[doc(hidden)]
#[derive(Debug)]
pub(in crate::tui) struct LoadingState {
    /// The path of the replay being inspected.
    path: PathBuf,
    /// The time at which the operation was invoked.
    start_time: Instant,
    /// The request ID for this invocation.
    request_id: u64,
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
                path,
                error: io::Error::new(io::ErrorKind::BrokenPipe, EMSG_BACKEND_CONNECTION_BROKE)
                    .into(),
                processed_for: Duration::ZERO,
            };
        };

        Self::Loading(LoadingState {
            path,
            start_time: Instant::now(),
            request_id,
        })
    }

    pub(in crate::tui) fn render(&self, frame: &mut Frame) {
        match self {
            Self::Loading(LoadingState {
                path,
                start_time,
                request_id: _,
            }) => {
                render_loading(path, OP_EVEINSP_LOADING_MSG, *start_time, frame);
            }
            Self::Done { path: _, inputs } => {
                todo!("Render inputs: {inputs:?}");
            }
            Self::Failed {
                path,
                error,
                processed_for,
            } => {
                render_error(path, error, *processed_for, frame);
            }
        }
    }

    pub(in crate::tui) fn handle_event(&self, ev: crossterm::event::Event) {
        todo!("Handle event: {self:?} | {ev:?}");
    }

    pub(in crate::tui) fn handle_response(&mut self, response: BackendResponse) {
        let state = match self {
            Self::Loading(state) => state,
            _ => return,
        };

        let result = match response {
            BackendResponse::EventDataFetch { result, request_id }
                if request_id == state.request_id =>
            {
                result
            }
            _ => return,
        };

        match result {
            Ok(inputs) => {
                *self = Self::Done {
                    path: core::mem::take(&mut state.path),
                    inputs,
                }
            }
            Err(error) => {
                *self = Self::Failed {
                    path: core::mem::take(&mut state.path),
                    error,
                    processed_for: Instant::now()
                        .checked_duration_since(state.start_time)
                        .unwrap_or(Duration::ZERO),
                }
            }
        }
    }
}
