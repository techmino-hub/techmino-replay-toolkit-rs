use crate::{
    ParseOrIoError,
    backend::{BackendConnection, BackendRequest, BackendResponse},
    consts::{
        backend::EMSG_BACKEND_CONNECTION_BROKE,
        tui::{
            ENTRY_STYLE_LINE_SEL, ENTRY_STYLE_LINE_UNSEL, OP_EVEINSP_LOADING_MSG,
            OP_EVEINSP_STYLE_FRAME_SEL, OP_EVEINSP_STYLE_FRAME_UNSEL, OP_EVEINSP_STYLE_KEY_SEL,
            OP_EVEINSP_STYLE_KEY_UNSEL, OP_EVEINSP_STYLE_KIND_SEL, OP_EVEINSP_STYLE_KIND_UNSEL,
            OP_EVEINSP_TEXT_PRESS, OP_EVEINSP_TEXT_RELEASE,
        },
    },
    tui::scenes::common_inspect::{inspect_outer_block, render_error, render_loading},
};
use core::{cell::RefCell, time::Duration};
use libtechmino_replay::replay::{
    GameInputEvent,
    action::{InputActionKey, InputActionKind},
};
use ratatui::{
    crossterm,
    prelude::{Frame, Line, Span, Style},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use std::{
    io,
    path::{Path, PathBuf},
    time::Instant,
};

/// The input event data inspection scene, where the user can inspect a selected
/// replay's input event data.
#[derive(Debug)]
pub(in crate::tui) enum EventInspectScene {
    /// Awaiting a reply from the backend.
    Loading(LoadingState),
    /// The backend has replied with the input event data.
    Done(CompleteState),
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

/// The state used for the `Done` state in the scene.
#[doc(hidden)]
#[derive(Debug)]
pub(in crate::tui) struct CompleteState {
    /// The path of the replay being inspected.
    path: PathBuf,
    /// The retrieved input event data.
    inputs: Vec<GameInputEvent>,
    /// The currently-selected input.
    selection_idx: usize,
    /// The state of the scrollbar.
    scrollbar_state: RefCell<ScrollbarState>,
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
            Self::Done(state) => {
                Self::render_done(state, frame);
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

    fn render_done(state: &CompleteState, frame: &mut Frame) {
        let block = inspect_outer_block(&state.path, frame.area());
        let inner_area = block.inner(frame.area());
        frame.render_widget(block, frame.area());

        let scrollbar_offset = state.scrollbar_state.borrow().get_position();

        for (row_idx, row_rect) in inner_area.rows().enumerate() {
            let idx = row_idx + scrollbar_offset;

            let Some(&event) = state.inputs.get(idx) else {
                break;
            };

            let selected = state.selection_idx == idx;

            let line = get_event_line(event, selected);

            frame.render_widget(line, row_rect);
        }

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);

        frame.render_stateful_widget(
            scrollbar,
            frame.area(),
            &mut state.scrollbar_state.borrow_mut(),
        );
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
                let input_len = inputs.len();
                *self = Self::Done(CompleteState {
                    path: core::mem::take(&mut state.path),
                    inputs,
                    selection_idx: 0,
                    scrollbar_state: RefCell::new(ScrollbarState::new(input_len)),
                });
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

/// Gets a [`Line`] representing a certain [`GameInputEvent`].
#[must_use = "render the `Line`"]
fn get_event_line(event: GameInputEvent, selected: bool) -> Line<'static> {
    const fn display_action_kind(action_kind: InputActionKind) -> &'static str {
        match action_kind {
            InputActionKind::Press => OP_EVEINSP_TEXT_PRESS,
            InputActionKind::Release => OP_EVEINSP_TEXT_RELEASE,
        }
    }

    const fn display_action_key(key: InputActionKey) -> &'static str {
        match key {
            InputActionKey::MoveLeft => "Move Left",
            InputActionKey::MoveRight => "Move Right",
            InputActionKey::RotateRight => "Rotate CW",
            InputActionKey::RotateLeft => "Rotate CCW",
            InputActionKey::Rotate180 => "Rotate 180°",
            InputActionKey::HardDrop => "Hard Drop",
            InputActionKey::SoftDrop => "Soft Drop",
            InputActionKey::Hold => "Hold",
            InputActionKey::Function1 => "Fn 1",
            InputActionKey::Function2 => "Fn 2",
            InputActionKey::InstantLeft => "Insta Left",
            InputActionKey::InstantRight => "Insta Right",
            InputActionKey::SonicDrop => "Sonic Drop",
            InputActionKey::Down1 => "Down 1",
            InputActionKey::Down4 => "Down 4",
            InputActionKey::Down10 => "Down 10",
            InputActionKey::LeftDrop => "Left Drop",
            InputActionKey::RightDrop => "Right Drop",
            InputActionKey::LeftZangi => "Left Zangi",
            InputActionKey::RightZangi => "Right Zangi",
        }
    }

    let frame = format!("F#{:07}:", event.frame());
    let frame = Span::raw(frame).style(style_frame(selected));

    let kind = display_action_kind(event.kind());
    let kind = Span::raw(kind).style(style_kind(selected));

    let key = display_action_key(event.key());
    let key = Span::raw(key).style(style_key(selected));

    Line {
        style: style_line(selected),
        alignment: None,
        spans: vec![frame, kind, key],
    }
}

/// Gets the style for the input entry line.
fn style_line(selected: bool) -> Style {
    if selected {
        ENTRY_STYLE_LINE_SEL
    } else {
        ENTRY_STYLE_LINE_UNSEL
    }
}

/// Gets the style for the frame counter.
fn style_frame(selected: bool) -> Style {
    if selected {
        OP_EVEINSP_STYLE_FRAME_SEL
    } else {
        OP_EVEINSP_STYLE_FRAME_UNSEL
    }
}

/// Gets the style for the input kind display.
fn style_kind(selected: bool) -> Style {
    if selected {
        OP_EVEINSP_STYLE_KIND_SEL
    } else {
        OP_EVEINSP_STYLE_KIND_UNSEL
    }
}

/// Gets the style for the input key display.
fn style_key(selected: bool) -> Style {
    if selected {
        OP_EVEINSP_STYLE_KEY_SEL
    } else {
        OP_EVEINSP_STYLE_KEY_UNSEL
    }
}
