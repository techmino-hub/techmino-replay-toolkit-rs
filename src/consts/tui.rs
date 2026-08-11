use ratatui::crossterm::event::KeyCode;

/// General keycode list for navigating to the previous entry in a vertical list.
pub(crate) const KC_GEN_UP: [KeyCode; 2] = [KeyCode::Up, KeyCode::Char('k')];

/// General keycode list for navigating to the next entry in a list.
pub(crate) const KC_GEN_DOWN: [KeyCode; 2] = [KeyCode::Down, KeyCode::Char('j')];

/// General keycode list for navigating up one page in a list.
pub(crate) const KC_GEN_PGUP: [KeyCode; 1] = [KeyCode::PageUp];

/// General keycode list for navigating down one page in a list.
pub(crate) const KC_GEN_PGDOWN: [KeyCode; 1] = [KeyCode::PageDown];

/// General keycode list for navigating to the start of a list.
pub(crate) const KC_GEN_HOME: [KeyCode; 1] = [KeyCode::Home];

/// General keycode list for navigating to the end of a list.
pub(crate) const KC_GEN_END: [KeyCode; 1] = [KeyCode::End];

/// General keycode list for activating a value in a list.
pub(crate) const KC_GEN_ACTIVATE: [KeyCode; 1] = [KeyCode::Enter];

/// The heading text for the "error" version of the explorer scene.
pub(crate) static EXP_ERROR_HEADING: &str = "Could not read file list";

/// The hint text for the "error" version of the explorer scene.
pub(crate) static EXP_ERROR_HINT: &str = "Alt+U to move up; Q or Ctrl+C to quit";

/// The minimum height in the "error" version of the explorer scene
/// that has any vertical padding.
pub(crate) const EXP_ERROR_PADDING_MIN_HEIGHT: u16 = 6;
