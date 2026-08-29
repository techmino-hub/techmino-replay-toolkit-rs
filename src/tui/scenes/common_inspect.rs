//! Common elements shared between event data and metadata inspection scenes.

use core::time::Duration;
use std::{path::Path, time::Instant};

use ratatui::{
    prelude::{Frame, Line, Rect, symbols::merge::MergeStrategy},
    widgets::{Block, Paragraph},
};

use crate::{
    ParseOrIoError,
    consts::tui::{OPS_HINT_BACK, OPS_HINT_QUIT, OPS_HINT_TIMEOUT_DURATION, OPS_HINT_TIMEOUT_TEXT},
    paths::truncate_folder_path,
};

pub(in crate::tui::scenes) fn render_loading(
    path: &Path,
    message: &str,
    start_time: Instant,
    frame: &mut Frame,
) {
    let block = inspect_outer_block(path, frame.area());
    let inner_rect = block.inner(frame.area());
    frame.render_widget(block, frame.area());

    let duration = Instant::now()
        .checked_duration_since(start_time)
        .unwrap_or(Duration::ZERO);

    let hint = if duration > OPS_HINT_TIMEOUT_DURATION {
        OPS_HINT_TIMEOUT_TEXT
    } else {
        ""
    };

    let text = format!(
        "{message}\n{duration}\n{hint}",
        duration = display_duration(duration),
    );

    let text = Paragraph::new(text);

    frame.render_widget(text, inner_rect);
}

pub(in crate::tui::scenes) fn render_error(
    path: &Path,
    error: &ParseOrIoError,
    processed_for: Duration,
    frame: &mut Frame,
) {
    let block = inspect_outer_block(path, frame.area());
    let inner_rect = block.inner(frame.area());
    frame.render_widget(block, frame.area());

    let text = format!(
        "Error after {duration}:\n\
    {error}",
        duration = display_duration(processed_for)
    );

    let text = Paragraph::new(text);

    frame.render_widget(text, inner_rect);
}

/// Returns the displayed form of the duration.
fn display_duration(duration: Duration) -> String {
    if duration < Duration::from_secs(1) {
        format!("{} ms", duration.as_millis())
    } else if duration < Duration::from_mins(1) {
        format!("{} s {} ms", duration.as_secs(), duration.subsec_millis())
    } else {
        let secs = duration.as_secs();
        format!("{} min {} s", secs / 60, secs % 60)
    }
}

/// The outer block for the inspect scenes.
#[must_use = "render this Block"]
pub(in crate::tui::scenes) fn inspect_outer_block(path: &Path, rect: Rect) -> Block<'static> {
    let path = path.to_string_lossy();
    let path = truncate_folder_path(&path, rect.width.saturating_sub(2) as usize).to_string();
    Block::bordered()
        .title(path)
        .title_bottom(Line::from(OPS_HINT_BACK).left_aligned())
        .title_bottom(Line::from(OPS_HINT_QUIT).right_aligned())
        .merge_borders(MergeStrategy::Fuzzy)
}
