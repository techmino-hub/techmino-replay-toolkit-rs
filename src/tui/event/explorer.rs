//! Module for processing crossterm input events into logical events in the
//! explorer scene.

use ratatui::crossterm::event::{Event, KeyEvent, KeyModifiers};

use crate::tui::event::VerticalListEvent;

/// Represents a logical event in the explorer scene.
#[must_use = "Process the explorer event"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::tui) enum ExplorerEvent {
    /// Navigate to the parent directory.
    Pop,
    /// Refresh the current directory listing.
    Refresh,
    /// An event relating to the file list.
    ListEvent(VerticalListEvent),
    /// Quit the application.
    Quit,
}

impl ExplorerEvent {
    /// Process the crossterm event, possibly yielding a logical list event.
    #[must_use = "Process the explorer event"]
    pub(in crate::tui) fn process_ev(ev: &Event) -> Option<Self> {
        if let Some(ev) = ev.as_key_event().and_then(Self::process_key_ev) {
            return Some(ev);
        }

        VerticalListEvent::process_ev(ev).map(Self::ListEvent)
    }

    /// Process the crossterm keyboard event, possibly yielding a logical list event.
    fn process_key_ev(ev: KeyEvent) -> Option<Self> {
        if ev.is_release() {
            return None;
        }

        let code = ev.code;

        if code.is_function_key(5) {
            return Some(Self::Refresh);
        }

        if code.is_char('c') && ev.modifiers.contains(KeyModifiers::CONTROL) {
            return Some(Self::Quit);
        }

        if code.is_esc() || code.is_char('q') {
            return Some(Self::Quit);
        }

        if code.is_up() && ev.modifiers.contains(KeyModifiers::ALT) {
            return Some(Self::Pop);
        }

        None
    }
}
