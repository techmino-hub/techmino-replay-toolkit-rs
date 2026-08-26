//! Module for processing crossterm input events into logical events.

use ratatui::crossterm::event::{
    Event, KeyEvent, MouseEvent,
    MouseEventKind::{self},
};

use crate::consts::tui::{
    KC_GEN_ACTIVATE, KC_GEN_DOWN, KC_GEN_END, KC_GEN_HOME, KC_GEN_PGDOWN, KC_GEN_PGUP, KC_GEN_UP,
};

pub(in crate::tui) mod explorer;
pub(in crate::tui) mod operations;

/// A logical event in the context of a vertical UI list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::tui) enum VerticalListEvent {
    /// Move the cursor to the previous entry in the list.
    Prev,
    /// Move the cursor to the next entry in the list.
    Next,
    /// Move the cursor to the start of the list.
    Home,
    /// Move the cursor to the end of the list.
    End,
    /// Move the cursor one page up.
    PageUp,
    /// Move the cursor one page down.
    PageDown,
    /// Select/activate the currently-highlighted entry in the list.
    Activate,
}

impl VerticalListEvent {
    /// Process the crossterm event, possibly yielding a logical list event.
    pub(in crate::tui) fn process_ev(ev: &Event) -> Option<Self> {
        match ev {
            Event::FocusGained => None,
            Event::FocusLost => None,
            Event::Resize(..) => None,
            Event::Paste(..) => None,
            Event::Mouse(ev) => Self::process_mouse_ev(ev),
            Event::Key(ev) => Self::process_key_ev(*ev),
        }
    }

    /// Process the crossterm mouse event, possibly yielding a logical list event.
    fn process_mouse_ev(ev: &MouseEvent) -> Option<Self> {
        match ev.kind {
            MouseEventKind::Up(..) => None,
            MouseEventKind::Down(..) => None,
            MouseEventKind::Drag(..) => None,
            MouseEventKind::Moved => None,
            MouseEventKind::ScrollLeft => None,
            MouseEventKind::ScrollRight => None,
            MouseEventKind::ScrollDown => Some(Self::Next),
            MouseEventKind::ScrollUp => Some(Self::Prev),
        }
    }

    /// Process the crossterm keyboard event, possibly yielding a logical list event.
    fn process_key_ev(ev: KeyEvent) -> Option<Self> {
        if ev.is_release() {
            return None;
        }

        match ev.code {
            c if KC_GEN_UP.contains(&c) => Some(Self::Prev),
            c if KC_GEN_DOWN.contains(&c) => Some(Self::Next),
            c if KC_GEN_HOME.contains(&c) => Some(Self::Home),
            c if KC_GEN_END.contains(&c) => Some(Self::End),
            c if KC_GEN_PGUP.contains(&c) => Some(Self::PageUp),
            c if KC_GEN_PGDOWN.contains(&c) => Some(Self::PageDown),
            c if KC_GEN_ACTIVATE.contains(&c) && ev.is_press() => Some(Self::Activate),
            _ => None,
        }
    }
}
