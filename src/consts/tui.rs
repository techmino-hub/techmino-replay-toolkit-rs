use ratatui::{
    crossterm::event::KeyCode,
    prelude::{Constraint, Style},
    widgets::Padding,
};

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
pub(crate) static EXP_ERROR_HINT: &str = "Alt+↑ to move up; Q or Ctrl+C to quit";

/// The minimum height of the terminal in the "error" version of the explorer
/// scene that has any vertical padding.
pub(crate) const EXP_ERROR_PADDING_MIN_HEIGHT: u16 = 16;

/// The size of the vertical padding in the "error" version of the explorer
/// scene.
///
/// This assumes the minimum height requirement is met, see
/// [`EXP_ERROR_PADDING_MIN_HEIGHT`].
pub(crate) const EXP_ERROR_PADDING_CONSTRAINT: Constraint = Constraint::Percentage(20);

/// The text for the unselected entry's spacer.
pub(crate) const EXP_PRIM_TEXT_SPACER_UNSEL: &str = " ";

/// The text for the selected entry's spacer.
pub(crate) const EXP_PRIM_TEXT_SPACER_SEL: &str = "→ ";

/// The text for the parent directory entry.
pub(crate) const EXP_PRIM_TEXT_PARENT: &str = "../   (Alt+↑)";

/// The style to give to the primary block's unselected line.
pub(crate) const EXP_PRIM_STYLE_LINE_UNSEL: Style = Style::new();

/// The style to give to the primary block's selected line.
pub(crate) const EXP_PRIM_STYLE_LINE_SEL: Style = Style::new().on_red().bold();

/// The style to give to the primary block's unselected spacer.
pub(crate) const EXP_PRIM_STYLE_SPACER_UNSEL: Style = EXP_PRIM_STYLE_LINE_UNSEL;

/// The style to give to the primary block's selected spacer.
pub(crate) const EXP_PRIM_STYLE_SPACER_SEL: Style = EXP_PRIM_STYLE_LINE_SEL.light_red();

/// The style to give to the primary block's unselected filename.
pub(crate) const EXP_PRIM_STYLE_FILENAME_UNSEL: Style = EXP_PRIM_STYLE_LINE_UNSEL;

/// The style to give to the primary block's selected filename.
pub(crate) const EXP_PRIM_STYLE_FILENAME_SEL: Style =
    EXP_PRIM_STYLE_LINE_SEL.light_cyan().underlined();

/// The minimum width of the secondary block for it to contain a
/// scroll hint in the bottom right.
pub(crate) const EXP_SEC_SCROLL_HINT_MIN_BLOCK_WIDTH: u16 = 32;

/// The "content length" of the secondary block's contents.
///
/// Used for the [`ScrollbarState`][ratatui::widgets::ScrollbarState] to
/// limit scroll offsets.
pub(crate) const EXP_SEC_CONTENT_LENGTH: usize = 24;

/// The file hint in the bottom left of the secondary block if the selected file
/// is deemed to be a replay file.
pub(crate) static EXP_SEC_HINT_IS_REPLAY: &str = "replay file";

/// The file hint in the bottom left of the secondary block if the selected file
/// is deemed not to be a replay file.
pub(crate) static EXP_SEC_HINT_NOT_REPLAY: &str = "not replay";

/// The scroll hint in the bottom right of the secondary block if the terminal
/// is wide enough.
///
/// See [`EXP_SEC_SCROLL_HINT_MIN_BLOCK_WIDTH`].
pub(crate) static EXP_SEC_HINT_SCROLL: &str = "ctrl+↑↓ to scroll";

/// The padding for the secondary block.
pub(crate) const EXP_SEC_BLOCK_PADDING: Padding = Padding::horizontal(1);

/// The maximum allowed buffer lengths for metadata viewers.
pub(crate) const EXP_MAX_METADATA_LEN: usize = 131072;
