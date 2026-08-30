use core::{cell::RefCell, time::Duration};
use std::{
    io,
    path::{Path, PathBuf},
    sync::mpsc,
    time::Instant,
};

use libtechmino_replay::replay::GameReplayMetadata;
use ratatui::{
    DefaultTerminal, crossterm,
    prelude::{Frame, Line, Span, Style},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::{
    ParseOrIoError,
    backend::{
        BackendConnection, BackendRequest, BackendResponse,
        presentation::tui::{TuiPresentableMeta, TuiPresentableMetaEntry},
    },
    consts::{
        backend::EMSG_BACKEND_CONNECTION_BROKE,
        tui::{
            ENTRY_STYLE_LABEL_SEL, ENTRY_STYLE_LABEL_UNSEL, OP_METAINSP_LOADING_MSG,
            OP_METINSP_STYLE_ARR_SEL, OP_METINSP_STYLE_ARR_UNSEL, OP_METINSP_STYLE_BOOL_SEL,
            OP_METINSP_STYLE_BOOL_UNSEL, OP_METINSP_STYLE_KEY_SEL, OP_METINSP_STYLE_KEY_UNSEL,
            OP_METINSP_STYLE_NULL_SEL, OP_METINSP_STYLE_NULL_UNSEL, OP_METINSP_STYLE_NUM_SEL,
            OP_METINSP_STYLE_NUM_UNSEL, OP_METINSP_STYLE_STR_SEL, OP_METINSP_STYLE_STR_UNSEL,
        },
    },
    tui::{
        event::{VerticalListEvent, meta_inspect::MetaInspectEvent},
        scenes::common_inspect::{
            InspectionTransition, inspect_outer_block, render_error, render_loading,
        },
    },
};

/// The metadata inspection scene, where the user can inspect a selected
/// replay's metadata.
#[derive(Debug)]
pub(in crate::tui) enum MetaInspectScene {
    /// Awaiting a reply from the backend.
    Loading(LoadingState),
    /// The backend has replied with the metadata.
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
    /// The retrieved metadata.
    metadata: TuiPresentableMeta,
    /// The currently-selected input.
    selection_idx: usize,
    /// The state of the scrollbar.
    scrollbar_state: RefCell<ScrollbarState>,
}

impl MetaInspectScene {
    pub(in crate::tui) fn new(path: PathBuf, backend: &BackendConnection) -> Self {
        let request_id: u64 = rand::random();

        let request = BackendRequest::FetchMetadata {
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
                render_loading(path, OP_METAINSP_LOADING_MSG, *start_time, frame);
            }
            Self::Done(state) => {
                state.render(frame);
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

    pub(in crate::tui) fn handle_event(
        &mut self,
        ev: crossterm::event::Event,
        terminal: &DefaultTerminal,
    ) -> Option<InspectionTransition> {
        let ev = MetaInspectEvent::process_ev(&ev)?;

        match ev {
            MetaInspectEvent::Back => Some(InspectionTransition::Back {
                path: self.get_path().to_owned(),
            }),
            MetaInspectEvent::ListEvent(ev) => {
                if let MetaInspectScene::Done(state) = self {
                    state.handle_list_event(ev, terminal)
                }
                None
            }
            MetaInspectEvent::Rescroll(_, term_height) => {
                if let MetaInspectScene::Done(state) = self {
                    let page_height = term_height.saturating_sub(2);
                    state.rescroll(page_height);
                }
                None
            }
            MetaInspectEvent::Quit => Some(InspectionTransition::Quit),
        }
    }

    pub(in crate::tui) fn handle_response(
        &mut self,
        response: BackendResponse,
        tx: &mpsc::Sender<BackendRequest>,
    ) {
        let state = match self {
            Self::Loading(state) => state,
            _ => return,
        };

        let metadata = match response {
            BackendResponse::MetadataFetch { result, request_id }
                if request_id == state.request_id =>
            {
                self.handle_response_fetch(result, tx);
                return;
            }
            BackendResponse::TuiPresentMetadata {
                metadata,
                request_id,
            } if request_id == state.request_id => metadata,
            _ => return,
        };

        let meta_len = metadata.0.len();
        let state = CompleteState {
            path: self.take_path(),
            metadata,
            selection_idx: 0,
            scrollbar_state: RefCell::new(ScrollbarState::new(meta_len)),
        };

        *self = Self::Done(state);
    }

    /// Handles a [`BackedResponse::MetadataFetch`] response.
    fn handle_response_fetch(
        &mut self,
        result: Result<GameReplayMetadata, ParseOrIoError>,
        tx: &mpsc::Sender<BackendRequest>,
    ) {
        let state = match self {
            Self::Loading(state) => state,
            _ => return,
        };

        match result {
            Ok(metadata) => {
                let request_id = state.request_id;
                self.request_presentable(metadata, tx, request_id);
            }
            Err(error) => {
                self.set_failed(error);
            }
        }
    }

    /// Request the presentable form from the backend.
    fn request_presentable(
        &mut self,
        metadata: GameReplayMetadata,
        tx: &mpsc::Sender<BackendRequest>,
        request_id: u64,
    ) {
        let request = BackendRequest::TuiPresentMetadata {
            metadata,
            request_id,
        };

        if tx.send(request).is_err() {
            let err =
                io::Error::new(io::ErrorKind::BrokenPipe, EMSG_BACKEND_CONNECTION_BROKE).into();
            self.set_failed(err);
        }
    }

    /// Go to the [`Self::Failed`] state.
    fn set_failed(&mut self, error: ParseOrIoError) {
        let processed_for = match self {
            Self::Loading(state) => Instant::now()
                .checked_duration_since(state.start_time)
                .unwrap_or(Duration::ZERO),
            Self::Done(..) => Duration::ZERO,
            Self::Failed { processed_for, .. } => *processed_for,
        };

        let path = self.take_path();

        *self = Self::Failed {
            path,
            error,
            processed_for,
        };
    }

    fn get_path(&self) -> &Path {
        match self {
            MetaInspectScene::Loading(state) => &state.path,
            MetaInspectScene::Done(state) => &state.path,
            MetaInspectScene::Failed { path, .. } => path,
        }
    }

    fn take_path(&mut self) -> PathBuf {
        match self {
            Self::Loading(state) => core::mem::take(&mut state.path),
            Self::Done(state) => core::mem::take(&mut state.path),
            Self::Failed { path, .. } => core::mem::take(path),
        }
    }
}

impl CompleteState {
    fn handle_list_event(&mut self, ev: VerticalListEvent, terminal: &DefaultTerminal) {
        let term_height = terminal.size().map(|s| s.height).unwrap_or(0);
        let page_height = term_height.saturating_sub(2);

        match ev {
            VerticalListEvent::Prev => self.prev(page_height),
            VerticalListEvent::Next => self.next(page_height),
            VerticalListEvent::Home => self.first(page_height),
            VerticalListEvent::End => self.last(page_height),
            VerticalListEvent::PageUp => self.prev_multi(page_height as usize, page_height),
            VerticalListEvent::PageDown => self.next_multi(page_height as usize, page_height),
            VerticalListEvent::Activate => (),
        }
    }

    /// Move the cursor to the previous entry.
    ///
    /// # Page Height
    /// Page height is the height of the contents of the primary block. This is
    /// currently equal to terminal height minus two.
    fn prev(&mut self, page_height: u16) {
        self.prev_multi(1, page_height);
    }

    /// Move the cursor to the nth previous entry.
    ///
    /// # Page Height
    /// Page height is the height of the contents of the primary block. This is
    /// currently equal to terminal height minus two.
    fn prev_multi(&mut self, amount: usize, page_height: u16) {
        self.selection_idx = self.selection_idx.saturating_sub(amount);
        self.rescroll(page_height);
    }

    /// Move the cursor to the next entry.
    ///
    /// # Page Height
    /// Page height is the height of the contents of the primary block. This is
    /// currently equal to terminal height minus two.
    fn next(&mut self, page_height: u16) {
        self.next_multi(1, page_height);
    }

    /// Move the cursor to the nth next entry.
    ///
    /// # Page Height
    /// Page height is the height of the contents of the primary block. This is
    /// currently equal to terminal height minus two.
    fn next_multi(&mut self, amount: usize, page_height: u16) {
        let upper_bound = self.metadata.0.len().saturating_sub(1);
        self.selection_idx = self.selection_idx.saturating_add(amount).min(upper_bound);
        self.rescroll(page_height);
    }

    /// Move the cursor to the first entry.
    ///
    /// # Page Height
    /// Page height is the height of the contents of the primary block. This is
    /// currently equal to terminal height minus two.
    fn first(&mut self, page_height: u16) {
        self.selection_idx = 0;
        self.rescroll(page_height);
    }

    /// Move the cursor to the last entry.
    ///
    /// # Page Height
    /// Page height is the height of the contents of the primary block. This is
    /// currently equal to terminal height minus two.
    fn last(&mut self, page_height: u16) {
        self.selection_idx = self.metadata.0.len().saturating_sub(1);
        self.rescroll(page_height);
    }

    /// Rescrolls the screen such that the currently-selected entry remains
    /// on-screen.
    ///
    /// # Page Height
    /// Page height is the height of the contents of the primary block. This is
    /// currently equal to terminal height minus two.
    fn rescroll(&mut self, page_height: u16) {
        let Some(page_height_minus_one) = page_height.checked_sub(1) else {
            // Terminal is too small to render anything!
            return;
        };

        let page_height_minus_one = page_height_minus_one as usize;

        let first_displayed = self.scrollbar_state.borrow().get_position();
        let last_displayed = first_displayed.saturating_add(page_height_minus_one);

        let selected_idx = self.selection_idx;

        if selected_idx < first_displayed {
            // Selected is above viewport
            let mut state = self.scrollbar_state.borrow_mut();
            *state = state.position(selected_idx);
        } else if selected_idx > last_displayed {
            // Selected is below viewport
            let mut state = self.scrollbar_state.borrow_mut();
            *state = state.position(selected_idx.saturating_sub(page_height_minus_one));
        }
    }

    fn render(&self, frame: &mut Frame) {
        let block = inspect_outer_block(&self.path, frame.area());
        let inner_area = block.inner(frame.area());

        frame.render_widget(block, frame.area());

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);

        frame.render_stateful_widget(
            scrollbar,
            frame.area(),
            &mut self.scrollbar_state.borrow_mut(),
        );

        let scroll_offset = self.scrollbar_state.borrow().get_position();

        for (row_idx, row_rect) in inner_area.rows().enumerate() {
            let idx = row_idx.saturating_add(scroll_offset);

            let Some(entry) = self.metadata.0.get(idx) else {
                break;
            };

            let selected = idx == self.selection_idx;

            let line = entry.as_line(selected);
            frame.render_widget(line, row_rect);
        }
    }
}

impl TuiPresentableMetaEntry {
    fn style_key(selected: bool) -> Style {
        if selected {
            OP_METINSP_STYLE_KEY_SEL
        } else {
            OP_METINSP_STYLE_KEY_UNSEL
        }
    }

    fn key_as_span(&self, selected: bool) -> Span<'static> {
        let style = Self::style_key(selected);
        let key = format!("{:<16}: ", self.key);

        Span::raw(key).style(style)
    }

    fn style_value(&self, selected: bool) -> Style {
        use crate::backend::presentation::tui::TuiPresentableMetaEntryKind as EntryKind;

        match (self.json_type, selected) {
            (EntryKind::Null, true) => OP_METINSP_STYLE_NULL_SEL,
            (EntryKind::Null, false) => OP_METINSP_STYLE_NULL_UNSEL,
            (EntryKind::Bool, true) => OP_METINSP_STYLE_BOOL_SEL,
            (EntryKind::Bool, false) => OP_METINSP_STYLE_BOOL_UNSEL,
            (EntryKind::Number, true) => OP_METINSP_STYLE_NUM_SEL,
            (EntryKind::Number, false) => OP_METINSP_STYLE_NUM_UNSEL,
            (EntryKind::String, true) => OP_METINSP_STYLE_STR_SEL,
            (EntryKind::String, false) => OP_METINSP_STYLE_STR_UNSEL,
            (EntryKind::Array, true) => OP_METINSP_STYLE_ARR_SEL,
            (EntryKind::Array, false) => OP_METINSP_STYLE_ARR_UNSEL,
        }
    }

    fn value_as_span(&self, selected: bool) -> Span<'_> {
        let style = self.style_value(selected);
        Span::raw(&*self.value).style(style)
    }

    fn as_line(&self, selected: bool) -> Line<'_> {
        let style = if selected {
            ENTRY_STYLE_LABEL_SEL
        } else {
            ENTRY_STYLE_LABEL_UNSEL
        };

        Line {
            style,
            alignment: None,
            spans: vec![self.key_as_span(selected), self.value_as_span(selected)],
        }
    }
}
