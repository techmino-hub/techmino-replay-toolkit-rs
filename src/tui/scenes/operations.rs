use core::fmt::Display;
use std::path::PathBuf;

use ratatui::{
    crossterm,
    prelude::{Frame, Line},
    widgets::Block,
};
use strum::{EnumCount, IntoEnumIterator};

use crate::{
    backend::BackendResponse,
    consts::tui::{OPS_HINT_BACK, OPS_HINT_QUIT},
    paths::truncate_folder_path,
    tui::{
        event::{VerticalListEvent, operations::OperationsEvent},
        ui::get_selectable_entry_el,
    },
};

/// Represents the state of the operations scene.
#[derive(Debug)]
pub(in crate::tui) struct OperationsScene {
    /// The path of the replay file.
    rep_path: PathBuf,
    /// The currently-selected operation.
    selection_index: usize,
}

impl OperationsScene {
    /// Creates a new operations state based on the specified replay path.
    pub(in crate::tui) fn new(rep_path: PathBuf) -> Self {
        Self {
            rep_path,
            selection_index: 0,
        }
    }

    /// Decrements the selection index.
    fn prev(&mut self) {
        self.selection_index = self.selection_index.saturating_sub(1);
    }

    /// Increments the selection index.
    fn next(&mut self) {
        self.selection_index = self
            .selection_index
            .saturating_add(1)
            .min(const { OperationChoice::COUNT - 1 });
    }

    /// Moves the selection to the first option.
    fn first(&mut self) {
        self.selection_index = 0;
    }

    /// Moves the selection to the last option.
    fn last(&mut self) {
        self.selection_index = const { OperationChoice::COUNT - 1 };
    }

    pub(in crate::tui) fn render(&self, frame: &mut Frame) {
        let path_str = self.rep_path.to_string_lossy();
        let max_len = frame.area().width.saturating_sub(2) as usize;
        let path = truncate_folder_path(&path_str, max_len);
        let block = Block::bordered()
            .title(path)
            .title_bottom(Line::from(OPS_HINT_BACK).left_aligned())
            .title_bottom(Line::from(OPS_HINT_QUIT).right_aligned());

        let mut remaining_size = block.inner(frame.area());

        frame.render_widget(block, frame.area());

        for op in OperationChoice::iter() {
            let op_str = op.to_str();
            let selected = op.into_index() == self.selection_index;

            let entry = get_selectable_entry_el(op_str, selected);

            frame.render_widget(entry, remaining_size);

            remaining_size.y = remaining_size.y.saturating_add(1);
            remaining_size.height = remaining_size.height.saturating_sub(1);

            if remaining_size.is_empty() {
                break;
            }
        }
    }

    pub(in crate::tui) fn handle_event(
        &mut self,
        event: crossterm::event::Event,
    ) -> Option<OperationsTransition> {
        let event = OperationsEvent::process_ev(&event)?;

        match event {
            OperationsEvent::ListEvent(VerticalListEvent::Home)
            | OperationsEvent::ListEvent(VerticalListEvent::PageUp) => {
                self.first();
                None
            }
            OperationsEvent::ListEvent(VerticalListEvent::End)
            | OperationsEvent::ListEvent(VerticalListEvent::PageDown) => {
                self.last();
                None
            }
            OperationsEvent::ListEvent(VerticalListEvent::Prev) => {
                self.prev();
                None
            }
            OperationsEvent::ListEvent(VerticalListEvent::Next) => {
                self.next();
                None
            }
            OperationsEvent::ListEvent(VerticalListEvent::Activate) => {
                OperationChoice::try_from_index(self.selection_index).map(|operation| {
                    OperationsTransition::SelectOperation {
                        operation,
                        rep_path: self.rep_path.clone(),
                    }
                })
            }
            OperationsEvent::Explorer => Some(OperationsTransition::Explorer),
            OperationsEvent::Quit => Some(OperationsTransition::Quit),
        }
    }

    pub(in crate::tui) fn handle_response(&self, _reply: BackendResponse) {
        // Backend replies are ignored here
    }
}

/// An operation to perform.
#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::EnumCount, strum::EnumIter)]
#[repr(usize)]
pub(in crate::tui) enum OperationChoice {
    /// Inspect the replay's metadata.
    InspectMetadata,
    /// Inspect the replay's input event data.
    InspectEvents,
    // Remux,
}

impl OperationChoice {
    /// Gets the display string for this operation choice.
    const fn to_str(self) -> &'static str {
        match self {
            OperationChoice::InspectMetadata => "Inspect metadata",
            OperationChoice::InspectEvents => "Inspect input events",
            // OperationChoice::Remux => "Convert to another format",
        }
    }

    /// Attempts to convert an operation index to this operation choice.
    const fn try_from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::InspectMetadata),
            1 => Some(Self::InspectEvents),
            // 2 => Some(Self::Remux),
            _ => None,
        }
    }

    /// Converts this operation choice to its own index.
    const fn into_index(self) -> usize {
        self as usize
    }
}

impl Display for OperationChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

/// A struct representing an instruction to navigate to another scene, and
/// any associated/related data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::tui) enum OperationsTransition {
    /// An operation was selected.
    SelectOperation {
        /// The specific operation that was selected.
        operation: OperationChoice,
        /// The path to the replay file to perform the operation on.
        rep_path: PathBuf,
    },
    /// Go back to the explorer scene.
    Explorer,
    /// Quit the application.
    Quit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_choice_roundtrip() {
        for op in OperationChoice::iter() {
            let idx = op.into_index();
            assert_eq!(OperationChoice::try_from_index(idx), Some(op));
        }
    }
}
