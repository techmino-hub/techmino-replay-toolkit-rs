use core::time::Duration;

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

/// The text for an unselected list entry's spacer.
pub(crate) static ENTRY_TEXT_SPACER_UNSEL: &str = " ";

/// The text for a selected list entry's spacer.
pub(crate) static ENTRY_TEXT_SPACER_SEL: &str = "→ ";

/// The style to give to the list entry's unselected line.
pub(crate) const ENTRY_STYLE_LINE_UNSEL: Style = Style::new();

/// The style to give to the list entry's selected line.
pub(crate) const ENTRY_STYLE_LINE_SEL: Style = Style::new().on_red().bold();

/// The style to give to the list entry's unselected spacer.
pub(crate) const ENTRY_STYLE_SPACER_UNSEL: Style = ENTRY_STYLE_LINE_UNSEL;

/// The style to give to the list entry's selected spacer.
pub(crate) const ENTRY_STYLE_SPACER_SEL: Style = ENTRY_STYLE_LINE_SEL.light_red();

/// The style to give to the list entry's unselected main text.
pub(crate) const ENTRY_STYLE_LABEL_UNSEL: Style = ENTRY_STYLE_LINE_UNSEL;

/// The style to give to the list entry's selected main text.
pub(crate) const ENTRY_STYLE_LABEL_SEL: Style = ENTRY_STYLE_LINE_SEL.light_cyan().underlined();

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

/// The text for the parent directory entry.
pub(crate) const EXP_PRIM_TEXT_PARENT: &str = "../   (Alt+↑)";

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

/// The control hint in the bottom left of the operation scenes to tell the
/// user how to go back to the explorer menu.
pub(crate) static OPS_HINT_BACK: &str = "[Esc] back";

/// The control hint in the bottom right of the operation scenes to tell the
/// user how to quit the application.
pub(crate) static OPS_HINT_QUIT: &str = "[q] quit";

/// The text to display when an operation is taking too long.
pub(crate) static OPS_HINT_TIMEOUT_TEXT: &str = "Operation taking longer than expected\n\
    Is something stuck?\n\
    For gigantic/streaming data processing consider using the `libtechmino-replay` library directly.";

/// The amount of time to wait until displaying [`OPS_HINT_TIMEOUT_TEXT`].
pub(crate) const OPS_HINT_TIMEOUT_DURATION: Duration = Duration::from_secs(10);

/// The loading message for the metadata inspection operation scene.
pub(crate) static OP_METAINSP_LOADING_MSG: &str = "Loading metadata...";

/// The loading message for the input event data inspection operation scene.
pub(crate) static OP_EVEINSP_LOADING_MSG: &str = "Loading input event data...";

/// The text for when an input event is the "press" kind.
pub(crate) static OP_EVEINSP_TEXT_PRESS: &str = "   Press ";

/// The text for when an input event is the "release" kind.
pub(crate) static OP_EVEINSP_TEXT_RELEASE: &str = " Release ";

/// The style for the "frame" section in the input event, when the entry is
/// not selected.
pub(crate) const OP_EVEINSP_STYLE_FRAME_UNSEL: Style =
    ENTRY_STYLE_LABEL_UNSEL.underlined().italic();

/// The style for the "frame" section in the input event, when the entry is
/// selected.
pub(crate) const OP_EVEINSP_STYLE_FRAME_SEL: Style = ENTRY_STYLE_LABEL_SEL.underlined().italic();

/// The style for the "kind" section in the input event, when the entry is
/// not selected.
pub(crate) const OP_EVEINSP_STYLE_KIND_UNSEL: Style = ENTRY_STYLE_LABEL_UNSEL.not_underlined();

/// The style for the "kind" section in the input event, when the entry is
/// selected.
pub(crate) const OP_EVEINSP_STYLE_KIND_SEL: Style = ENTRY_STYLE_LABEL_SEL.not_underlined();

/// The style for the "key" section in the input event, when the entry is
/// not selected.
pub(crate) const OP_EVEINSP_STYLE_KEY_UNSEL: Style =
    ENTRY_STYLE_LABEL_UNSEL.not_underlined().bold();

/// The style for the "key" section in the input event, when the entry is
/// selected.
pub(crate) const OP_EVEINSP_STYLE_KEY_SEL: Style = ENTRY_STYLE_LABEL_SEL.not_underlined().bold();

/// The style for the key of the metadata entry, when the entry is
/// selected.
pub(crate) const OP_METINSP_STYLE_KEY_SEL: Style = ENTRY_STYLE_LABEL_SEL.italic();

/// The style for the key of the metadata entry, when the entry is
/// not selected.
pub(crate) const OP_METINSP_STYLE_KEY_UNSEL: Style = ENTRY_STYLE_LABEL_UNSEL.italic();

/// The generic style for the value of a metadata entry, when the entry is
/// selected.
pub(crate) const OP_METINSP_STYLE_ENTRY_SEL: Style = ENTRY_STYLE_LABEL_SEL.bold();

/// The generic style for the value of a metadata entry, when the entry is
/// not selected.
pub(crate) const OP_METINSP_STYLE_ENTRY_UNSEL: Style = ENTRY_STYLE_LABEL_UNSEL.bold();

/// The style for the null value of the metadata entry, when the entry is
/// selected.
pub(crate) const OP_METINSP_STYLE_NULL_SEL: Style = OP_METINSP_STYLE_ENTRY_SEL.dim();

/// The style for the null value of the metadata entry, when the entry is
/// not selected.
pub(crate) const OP_METINSP_STYLE_NULL_UNSEL: Style = OP_METINSP_STYLE_ENTRY_UNSEL.dim();

/// The style for the boolean value of the metadata entry, when the entry is
/// selected.
pub(crate) const OP_METINSP_STYLE_BOOL_SEL: Style = OP_METINSP_STYLE_ENTRY_SEL.light_blue();

/// The style for the boolean value of the metadata entry, when the entry is
/// not selected.
pub(crate) const OP_METINSP_STYLE_BOOL_UNSEL: Style = OP_METINSP_STYLE_ENTRY_UNSEL.light_blue();

/// The style for the number value of the metadata entry, when the entry is
/// selected.
pub(crate) const OP_METINSP_STYLE_NUM_SEL: Style = OP_METINSP_STYLE_ENTRY_SEL.light_cyan();

/// The style for the number value of the metadata entry, when the entry is
/// not selected.
pub(crate) const OP_METINSP_STYLE_NUM_UNSEL: Style = OP_METINSP_STYLE_ENTRY_UNSEL.light_cyan();

/// The style for the string value of the metadata entry, when the entry is
/// selected.
pub(crate) const OP_METINSP_STYLE_STR_SEL: Style = OP_METINSP_STYLE_ENTRY_SEL.light_green();

/// The style for the string value of the metadata entry, when the entry is
/// not selected.
pub(crate) const OP_METINSP_STYLE_STR_UNSEL: Style = OP_METINSP_STYLE_ENTRY_UNSEL.light_green();

/// The style for the array value of the metadata entry, when the entry is
/// selected.
pub(crate) const OP_METINSP_STYLE_ARR_SEL: Style = OP_METINSP_STYLE_ENTRY_SEL;

/// The style for the array value of the metadata entry, when the entry is
/// not selected.
pub(crate) const OP_METINSP_STYLE_ARR_UNSEL: Style = OP_METINSP_STYLE_ENTRY_UNSEL;
