use ratatui::crossterm::event::{Event, KeyEvent, KeyModifiers};

use crate::tui::event::VerticalListEvent;

/// Represents a logical event in the metadata inspection scene.
#[must_use = "Process the logical event"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::tui) enum MetaInspectEvent {
    /// An event relating to the event list.
    ListEvent(VerticalListEvent),
    /// Adjust the scroll state to make the currently-selected item visible.
    Rescroll(u16, u16),
    /// Go back to the operations selection menu.
    Back,
    /// Quit the application.
    Quit,
}

impl MetaInspectEvent {
    /// Process the crossterm event, possibly yielding a logical event.
    #[must_use = "Process the explorer event"]
    pub(in crate::tui) fn process_ev(ev: &Event) -> Option<Self> {
        if let Some(ev) = ev.as_key_event().and_then(Self::process_key_ev) {
            return Some(ev);
        }

        if let Some(list_ev) = VerticalListEvent::process_ev(ev) {
            return Some(Self::ListEvent(list_ev));
        }

        if let Some((cols, rows)) = ev.as_resize_event() {
            return Some(Self::Rescroll(cols, rows));
        }

        None
    }

    fn process_key_ev(ev: KeyEvent) -> Option<Self> {
        if ev.is_release() {
            return None;
        }

        let code = ev.code;

        if code.is_char('c') && ev.modifiers.contains(KeyModifiers::CONTROL) {
            return Some(Self::Quit);
        }

        if code.is_char('q') {
            return Some(Self::Quit);
        }

        if code.is_esc() {
            return Some(Self::Back);
        }

        None
    }
}
