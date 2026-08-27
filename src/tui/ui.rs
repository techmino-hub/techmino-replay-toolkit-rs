//! Helper functions for common UI elements.

use ratatui::prelude::{Line, Span};

use crate::consts::tui::{
    ENTRY_STYLE_LABEL_SEL, ENTRY_STYLE_LABEL_UNSEL, ENTRY_STYLE_LINE_SEL, ENTRY_STYLE_LINE_UNSEL,
    ENTRY_STYLE_SPACER_SEL, ENTRY_STYLE_SPACER_UNSEL, ENTRY_TEXT_SPACER_SEL,
    ENTRY_TEXT_SPACER_UNSEL,
};

/// Gets the spacer [`Span`] element for a selectable list entry.
pub(in crate::tui) fn get_spacer_el(selected: bool) -> Span<'static> {
    let content = if selected {
        ENTRY_TEXT_SPACER_SEL
    } else {
        ENTRY_TEXT_SPACER_UNSEL
    };

    let style = if selected {
        ENTRY_STYLE_SPACER_SEL
    } else {
        ENTRY_STYLE_SPACER_UNSEL
    };

    Span::raw(content).style(style)
}

/// Gets the label [`Span`] element for a selectable list entry.
pub(in crate::tui) fn get_label_el<'a>(content: &'a str, selected: bool) -> Span<'a> {
    let style = if selected {
        ENTRY_STYLE_LABEL_SEL
    } else {
        ENTRY_STYLE_LABEL_UNSEL
    };

    Span::raw(content).style(style)
}

/// Gets the [`Line`] element for a selectable list entry.
pub(in crate::tui) fn get_selectable_entry_el<'a>(content: &'a str, selected: bool) -> Line<'a> {
    let spacer = get_spacer_el(selected);
    let label = get_label_el(content, selected);
    let style = if selected {
        ENTRY_STYLE_LINE_SEL
    } else {
        ENTRY_STYLE_LINE_UNSEL
    };

    Line {
        style,
        spans: vec![spacer, label],
        alignment: None,
    }
}
