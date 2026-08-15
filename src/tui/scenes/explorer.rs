use core::{cell::RefCell, cmp::Ordering, fmt::Display, num::NonZeroUsize};
use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    fs::{self, Metadata},
    io::{self},
    path::{self, Path, PathBuf},
};

use libtechmino_replay::GameReplayMetadata;
use ratatui::{
    crossterm,
    prelude::{
        Constraint, Frame, HorizontalAlignment, Layout, Line, Rect, Span, StatefulWidget, Widget,
    },
    widgets::{Block, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
};

use crate::{
    backend::BackendReply,
    consts::tui::{
        EXP_ERROR_HEADING, EXP_ERROR_HINT, EXP_ERROR_PADDING_CONSTRAINT,
        EXP_ERROR_PADDING_MIN_HEIGHT, EXP_PRIM_STYLE_FILENAME_SEL, EXP_PRIM_STYLE_FILENAME_UNSEL,
        EXP_PRIM_STYLE_LINE_SEL, EXP_PRIM_STYLE_LINE_UNSEL, EXP_PRIM_STYLE_SPACER_SEL,
        EXP_PRIM_STYLE_SPACER_UNSEL, EXP_PRIM_TEXT_PARENT, EXP_PRIM_TEXT_SPACER_SEL,
        EXP_PRIM_TEXT_SPACER_UNSEL, EXP_SEC_BLOCK_PADDING, EXP_SEC_CONTENT_LENGTH,
        EXP_SEC_HINT_IS_REPLAY, EXP_SEC_HINT_NOT_REPLAY, EXP_SEC_HINT_SCROLL,
        EXP_SEC_SCROLL_HINT_MIN_BLOCK_WIDTH,
    },
    paths::truncate_folder_path,
    tui::{
        ParseOrIoError,
        event::{VerticalListEvent, explorer::ExplorerEvent},
    },
};

/// Represents the state of the explorer scene.
#[derive(Debug)]
pub(in crate::tui) struct ExplorerScene {
    /// The current folder being explored.
    folder: PathBuf,
    /// The (cached) file list currently being shown, or an I/O error if a
    /// fatal error occurred.
    file_list: io::Result<FileList>,
}

impl ExplorerScene {
    /// Creates a new explorer state based on the specified directory path.
    pub(in crate::tui) fn new(folder: &Path) -> Self {
        let folder = folder.canonicalize().unwrap_or_else(|_| folder.to_owned());

        let file_list = FileList::new(&folder);

        Self { folder, file_list }
    }

    /// Refreshes the file list.
    pub(in crate::tui) fn refresh(&mut self) {
        self.file_list = FileList::new(&self.folder);
    }

    /// Returns the current path being shown.
    pub(in crate::tui) fn folder(&self) -> &Path {
        &self.folder
    }

    /// Explore one layer up.
    fn up(&mut self) {
        let Some(new_folder) = self.folder().parent() else {
            return;
        };

        self.folder = new_folder.to_owned();

        self.file_list = FileList::new(self.folder());
    }

    /// Move the cursor to a certain new index.
    ///
    /// # Page Height
    /// Page height is the height of the contents of the primary block. This is
    /// currently equal to terminal height minus two.
    fn move_cursor<I>(&mut self, index_cb: I, page_height: u16)
    where
        I: FnOnce(&FileList) -> usize,
    {
        let Ok(file_list) = &mut self.file_list else {
            return;
        };

        let new_idx = index_cb(file_list);

        if file_list.selected.index == new_idx {
            return;
        }

        file_list.selected.index = new_idx;

        file_list
            .selected
            .update_metadata(&self.folder, &file_list.entries);

        file_list.rescroll(page_height);
    }

    /// Move the cursor to the previous item.
    ///
    /// # Page Height
    /// Page height is the height of the contents of the primary block. This is
    /// currently equal to terminal height minus two.
    fn prev(&mut self, page_height: u16) {
        self.prev_multi(1, page_height);
    }

    /// Move the cursor to the `amount`-th previous item.
    ///
    /// # Page Height
    /// Page height is the height of the contents of the primary block. This is
    /// currently equal to terminal height minus two.
    fn prev_multi(&mut self, amount: usize, page_height: u16) {
        self.move_cursor(
            |list| list.selected.index.saturating_sub(amount),
            page_height,
        );
    }

    /// Move the cursor to the next item.
    ///
    /// # Page Height
    /// Page height is the height of the contents of the primary block. This is
    /// currently equal to terminal height minus two.
    fn next(&mut self, page_height: u16) {
        self.next_multi(1, page_height);
    }

    /// Move the cursor to the `amount`-th next item.
    ///
    /// # Page Height
    /// Page height is the height of the contents of the primary block. This is
    /// currently equal to terminal height minus two.
    fn next_multi(&mut self, amount: usize, page_height: u16) {
        let index_cb = |list: &FileList| -> usize {
            let len = list.entries.len();
            let index = list.selected.index;

            index.saturating_add(amount).min(len.saturating_sub(1))
        };

        self.move_cursor(index_cb, page_height);
    }

    /// Move the cursor to the first item.
    ///
    /// # Page Height
    /// Page height is the height of the contents of the primary block. This is
    /// currently equal to terminal height minus two.
    fn first(&mut self, page_height: u16) {
        self.move_cursor(|_| 0, page_height);
    }

    /// Move the cursor to the last item.
    ///
    /// # Page Height
    /// Page height is the height of the contents of the primary block. This is
    /// currently equal to terminal height minus two.
    fn last(&mut self, page_height: u16) {
        self.move_cursor(|list| list.entries.len() - 1, page_height);
    }

    /// Selects the currently-highlighted item.
    ///
    /// # Returns
    /// If `Some`, then the scene should switch to the operations scene, using
    /// the encapsulated parameters.
    ///
    /// If `None`, then the scene should stay in this explorer scene.
    #[must_use = "Use the returned value to switch to the operations scene"]
    fn select(&mut self, ret_path: &mut PathBuf) -> Option<ExplorerTransition> {
        let Ok(file_list) = &self.file_list else {
            return None;
        };

        let entry = file_list.entries.get(file_list.selected.index)?;

        let path = entry.resolve(&self.folder);

        if path.is_file() {
            ret_path.clone_from(&self.folder);
            let instr = ExplorerTransition::OperationsScene { file_path: path };
            return Some(instr);
        }

        self.folder = path;
        self.refresh();

        None
    }

    /// Gets the primary block of this scene.
    fn primary_block(&self) -> ExplorerPrimaryBlock<'_> {
        let path = self.folder().as_os_str().to_string_lossy();
        let error_counter = self
            .file_list
            .as_ref()
            .map_or(ErrorCounter::Fatal, |l| l.error_counter());

        ExplorerPrimaryBlock {
            path,
            error_counter,
        }
    }

    /// Renders this scene to the given frame.
    pub(in crate::tui::scenes) fn render(&self, frame: &mut Frame) {
        match self.file_list {
            Ok(ref list) => {
                let block = self.primary_block().to_owned();
                Self::render_normal(block, list, frame);
            }
            Err(ref error) => {
                let block = self.primary_block();
                Self::render_error(block, error, frame);
            }
        }
    }

    /// Rendering when the directory was read successfully
    fn render_normal(prim_block: ExplorerPrimaryBlock<'_>, list: &FileList, frame: &mut Frame) {
        // TODO: Proper rendering

        let [prim_area, sec_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(frame.area());

        let block = prim_block.instantiate(prim_area);
        let inner_prim_area = block.inner(prim_area);
        frame.render_widget(block, prim_area);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        frame.render_stateful_widget(scrollbar, prim_area, &mut list.primary_vscroll.borrow_mut());

        frame.render_widget(list.as_primary_widget(), inner_prim_area);
        frame.render_widget(list.as_secondary_widget(), sec_area);
    }

    /// Rendering when the file list can't be retrieved (i.e., a fatal error occurred)
    fn render_error(block: ExplorerPrimaryBlock<'_>, error: &io::Error, frame: &mut Frame) {
        let area = frame.area();
        let block = block.instantiate(area);
        frame.render_widget(&block, area);

        let area = block.inner(area);

        let text = format!(
            "{heading}\n{error}\n{hint}",
            heading = EXP_ERROR_HEADING,
            hint = EXP_ERROR_HINT
        );
        let text = Paragraph::new(text).wrap(Wrap { trim: true }).centered();

        let padding_constraint = (frame.area().height >= EXP_ERROR_PADDING_MIN_HEIGHT)
            .then_some(EXP_ERROR_PADDING_CONSTRAINT);
        let constraints = padding_constraint
            .into_iter()
            .chain(core::iter::once(Constraint::Fill(1)));

        let layout = Layout::vertical(constraints);
        let rects = layout.split(area);
        let allocation = rects
            .last()
            .copied()
            .expect("constraint list should not be empty");
        drop(rects);

        frame.render_widget(text, allocation);
    }

    /// Handles a crossterm event.
    ///
    /// # Returns
    /// If `Some`, then the scene should switch to the operations scene, using
    /// the encapsulated parameters.
    ///
    /// If `None`, then the scene should stay in this explorer scene.
    #[must_use = "Use the returned value to switch to the operations scene"]
    pub(in crate::tui::scenes) fn handle_event(
        &mut self,
        event: crossterm::event::Event,
        terminal: &ratatui::DefaultTerminal,
        ret_path: &mut PathBuf,
    ) -> Option<ExplorerTransition> {
        let event = ExplorerEvent::process_ev(&event)?;

        match event {
            ExplorerEvent::Pop => {
                self.up();
                None
            }
            ExplorerEvent::Refresh => {
                self.refresh();
                None
            }
            ExplorerEvent::PrimaryListEvent(event) => {
                self.handle_primary_list_event(event, terminal, ret_path)
            }
            ExplorerEvent::SecondaryListEvent(event) => {
                self.handle_secondary_list_event(event, terminal);
                None
            }
            ExplorerEvent::Rescroll(_cols, rows) => {
                if let Ok(list) = &mut self.file_list {
                    list.rescroll(rows);
                }
                None
            }
            ExplorerEvent::Quit => Some(ExplorerTransition::Quit),
        }
    }

    /// Handles a primary list logical event.
    fn handle_primary_list_event(
        &mut self,
        event: VerticalListEvent,
        terminal: &ratatui::DefaultTerminal,
        ret_path: &mut PathBuf,
    ) -> Option<ExplorerTransition> {
        let term_height = terminal.size().map(|s| s.height).unwrap_or(0);
        let page_height = term_height.saturating_sub(2);

        match event {
            VerticalListEvent::Prev => {
                self.prev(page_height);
                None
            }
            VerticalListEvent::Next => {
                self.next(page_height);
                None
            }
            VerticalListEvent::Home => {
                self.first(page_height);
                None
            }
            VerticalListEvent::End => {
                self.last(page_height);
                None
            }
            VerticalListEvent::PageUp => {
                self.prev_multi(page_height as usize, page_height);
                None
            }
            VerticalListEvent::PageDown => {
                self.next_multi(page_height as usize, page_height);
                None
            }
            VerticalListEvent::Activate => self.select(ret_path),
        }
    }

    /// Handles a secondary list logical event.
    fn handle_secondary_list_event(
        &mut self,
        event: VerticalListEvent,
        terminal: &ratatui::DefaultTerminal,
    ) {
        let Ok(list) = &mut self.file_list else {
            return;
        };

        let term_height = terminal.size().map(|s| s.height).unwrap_or(0);
        let page_height = term_height.saturating_sub(2);

        let mut offset = list.selected.secondary_vscroll.borrow_mut();

        match event {
            VerticalListEvent::Prev => offset.prev(),
            VerticalListEvent::Next => offset.next(),
            VerticalListEvent::Home => offset.first(),
            VerticalListEvent::End => offset.last(),
            VerticalListEvent::PageUp => {
                // HACK: Looping `prev` instead of setting position manually
                // [`ScrollbarState`] doesn't have a method to retrieve content
                // length
                for _ in 0..page_height {
                    offset.prev();
                }
            }
            VerticalListEvent::PageDown => {
                // HACK: Looping `next` instead of setting position manually
                // [`ScrollbarState`] doesn't have a method to retrieve content
                // length
                for _ in 0..page_height {
                    offset.next();
                }
            }
            VerticalListEvent::Activate => (),
        }
    }

    /// Handle a reply from the backend.
    pub(in crate::tui::scenes) fn handle_reply(&mut self, reply: BackendReply) {
        todo!("Handle backend reply: {reply:?}");
    }
}

/// Represents a cache of the list of files shown in the interface.
#[derive(Debug)]
pub(in crate::tui) struct FileList {
    /// The list of file entries shown in the current directory.
    entries: Vec<UiDirEntry>,
    /// Information about the currently-selected file.
    selected: SelectedFile,
    /// The current vertical scrollbar state of the primary block list.
    primary_vscroll: RefCell<ScrollbarState>,
    /// The number of non-fatal I/O errors that occurred while traversing the
    /// directory.
    non_fatal_errors: usize,
}

impl FileList {
    /// Creates a new file list based on the specified directory path.
    pub fn new(folder: &Path) -> io::Result<FileList> {
        let folder = folder.canonicalize()?;

        let mut entries = Vec::new();
        let mut non_fatal_errors = 0;

        if let Some(parent_dir) = folder.parent() {
            let metadata = fs::metadata(parent_dir).ok();
            let entry = UiDirEntry::ParentDir { metadata };
            entries.push(entry);
        }

        for entry in fs::read_dir(&folder)? {
            let Ok(entry) = entry else {
                non_fatal_errors += 1;
                continue;
            };

            let mut name = entry.file_name();
            let mut is_dir = false;

            if let Ok(ftype) = entry.file_type()
                && ftype.is_dir()
            {
                name.push(path::MAIN_SEPARATOR_STR);
                is_dir = true;
            }

            let metadata = entry.metadata().ok();

            let entry = UiDirEntry::Regular {
                name,
                is_dir,
                metadata,
            };

            entries.push(entry);
        }

        entries.sort();

        let primary_vscroll = RefCell::new(ScrollbarState::new(entries.len()));

        let selected = SelectedFile::new(&folder, &entries);

        Ok(Self {
            entries,
            selected,
            primary_vscroll,
            non_fatal_errors,
        })
    }

    /// Adjust the primary vertical scroll offset to make the currently-selected
    /// entry visible, if it is currently off-screen.
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

        let first_displayed = self.primary_vscroll.borrow().get_position();
        let last_displayed = first_displayed.saturating_add(page_height_minus_one);

        let selected_idx = self.selected.index;

        if selected_idx < first_displayed {
            // Selected is above viewport
            let mut state = self.primary_vscroll.borrow_mut();
            *state = state.position(selected_idx);
        } else if selected_idx > last_displayed {
            // Selected is below viewport
            let mut state = self.primary_vscroll.borrow_mut();
            *state = state.position(selected_idx.saturating_sub(page_height_minus_one));
        }
    }

    /// Returns the data for the displayed error counter.
    #[must_use]
    pub(in crate::tui) fn error_counter(&self) -> ErrorCounter {
        match NonZeroUsize::new(self.non_fatal_errors) {
            Some(e) => ErrorCounter::NonFatal(e),
            None => ErrorCounter::None,
        }
    }

    /// Return a widget to display a representation of this for the primary block.
    fn as_primary_widget(&self) -> ExplorerPrimaryContents<'_> {
        ExplorerPrimaryContents(self)
    }

    /// Return a widget to display a representation of this for the secondary block.
    fn as_secondary_widget(&self) -> ExplorerSecondaryBlock<'_> {
        ExplorerSecondaryBlock(self)
    }
}

/// Contains internal data about the selected file shown in the secondary block.
#[derive(Debug)]
pub(in crate::tui) struct SelectedFile {
    /// The index of the file which is selected.
    index: usize,
    /// The replay-specific metadata of that file if it parses correctly.
    rep_metadata: Result<GameReplayMetadata, ParseOrIoError>,
    /// The current vertical scrollbar state of the secondary block.
    secondary_vscroll: RefCell<ScrollbarState>,
}

impl SelectedFile {
    /// Create a new selected file state pointing to the first entry.
    ///
    /// # Parameters
    /// - `folder`: The directory containing the given directory entries.
    /// - `entries`: The list of directory entries in the `folder` argument.
    #[must_use]
    pub(in crate::tui::scenes) fn new(folder: &Path, entries: &[UiDirEntry]) -> Self {
        let mut this = Self {
            index: 0,
            rep_metadata: Err(ParseOrIoError::Io(io::Error::other(""))),
            secondary_vscroll: RefCell::new(ScrollbarState::new(EXP_SEC_CONTENT_LENGTH)),
        };

        this.update_metadata(folder, entries);

        this
    }

    /// Given the current folder and list of entries, update the cached metadata.
    fn update_metadata(&mut self, folder: &Path, entries: &[UiDirEntry]) {
        self.rep_metadata = entries.get(self.index).map_or_else(
            || Err(io::Error::new(io::ErrorKind::NotFound, "Entry not found"))?,
            |entry| Self::read_entry(folder, entry),
        );
        self.secondary_vscroll.borrow_mut().first();
    }

    /// Reads a UI directory entry to try to extract the game replay metadata.
    ///
    /// # Parameters
    /// - `folder`: The directory containing the directory entry.
    /// - `entry`: The desired entry to read from.
    fn read_entry(
        _folder: &Path,
        _entry: &UiDirEntry,
    ) -> Result<GameReplayMetadata, ParseOrIoError> {
        // TODO: Read the metadata.
        // Blockers: Stabilize preprocessors

        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Metadata preview is not implemented yet",
        ))?
    }
}

/// Represents a cache of a directory entry shown in the interface.
#[derive(Debug)]
pub(in crate::tui) enum UiDirEntry {
    /// A regular directory entry, not a special one, e.g., not `..`.
    ///
    /// Encompasses both files, folders, and symlinks.
    Regular {
        /// The displayed name of the directory entry.
        ///
        /// If this entry is a directory, then this comes with the OS-specific
        /// path separator trailing at the end.
        name: OsString,
        /// Whether or not this directory entry is itself a directory.
        is_dir: bool,
        /// The metadata of this directory entry, if retrieving it was successful.
        metadata: Option<Metadata>,
    },
    /// The `..` (parent directory) virtual directory entry.
    ParentDir {
        /// The metadata of this directory entry, if retrieving it was successful.
        metadata: Option<Metadata>,
    },
}

impl UiDirEntry {
    /// Gets the displayed name of this directory entry.
    #[must_use]
    pub(in crate::tui) fn name(&self) -> &OsStr {
        match self {
            UiDirEntry::Regular {
                name,
                is_dir: _,
                metadata: _,
            } => name,
            UiDirEntry::ParentDir { metadata: _ } => OsStr::new(EXP_PRIM_TEXT_PARENT),
        }
    }

    /// Resolves this entry into a full path given a certain parent directory.
    #[must_use]
    pub(in crate::tui) fn resolve(&self, folder: &Path) -> PathBuf {
        match self {
            UiDirEntry::Regular {
                name,
                is_dir: _,
                metadata: _,
            } => folder.join(name),
            UiDirEntry::ParentDir { metadata: _ } => folder.parent().unwrap_or(folder).to_owned(),
        }
    }
}

impl PartialEq for UiDirEntry {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Regular {
                    name: l_name,
                    is_dir: l_is_dir,
                    metadata: _,
                },
                Self::Regular {
                    name: r_name,
                    is_dir: r_is_dir,
                    metadata: _,
                },
            ) => l_name == r_name && l_is_dir == r_is_dir,
            (Self::ParentDir { .. }, Self::ParentDir { .. }) => true,
            _ => false,
        }
    }
}

impl Eq for UiDirEntry {}

impl PartialOrd for UiDirEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for UiDirEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Parent dir, then dir, then files.

        let (lhs, rhs) = match (self, other) {
            (Self::ParentDir { .. }, Self::ParentDir { .. }) => return Ordering::Equal,
            (Self::ParentDir { .. }, Self::Regular { .. }) => return Ordering::Less,
            (Self::Regular { .. }, Self::ParentDir { .. }) => return Ordering::Greater,
            (Self::Regular { is_dir: true, .. }, Self::Regular { is_dir: false, .. }) => {
                return Ordering::Less;
            }
            (Self::Regular { is_dir: false, .. }, Self::Regular { is_dir: true, .. }) => {
                return Ordering::Greater;
            }
            (Self::Regular { name: lhs, .. }, Self::Regular { name: rhs, .. }) => (lhs, rhs),
        };

        // Then compare names logically
        alphanumeric_sort::compare_os_str(lhs, rhs)
    }
}

/// A struct representing an instruction to navigate to another scene, and
/// any associated/related data.
#[must_use]
#[derive(Debug)]
pub(in crate::tui) enum ExplorerTransition {
    /// Navigate to the operations scene.
    OperationsScene {
        /// The full path of the selected file to give to the operations scene.
        file_path: PathBuf,
    },
    /// Quit the application.
    Quit,
}

/// Represents the explorer scene's primary/file list block.
///
/// Use [`instantiate`][Self::instantiate] to convert into a [`Widget`].
#[must_use]
struct ExplorerPrimaryBlock<'a> {
    path: Cow<'a, str>,
    error_counter: ErrorCounter,
}

impl ExplorerPrimaryBlock<'_> {
    /// Instantiate this [`ExplorerPrimaryBlock`] instance into a [`Block`].
    #[must_use]
    fn instantiate(&self, area: Rect) -> Block<'_> {
        let path_max_len = area.width.saturating_sub(2);
        let path = truncate_folder_path(&self.path, path_max_len as usize);

        Block::bordered()
            .title_bottom(self.error_counter.to_str())
            .title_top(path)
    }

    fn to_owned(&self) -> Self {
        let path = self.path.to_string();
        let path = Cow::Owned(path);
        let error_counter = self.error_counter;

        Self {
            path,
            error_counter,
        }
    }
}

impl Widget for ExplorerPrimaryBlock<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        self.instantiate(area).render(area, buf)
    }
}

/// A representation of the error counter display.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::tui) enum ErrorCounter {
    /// No errors.
    None,
    /// A non-zero amount of non-fatal errors.
    NonFatal(NonZeroUsize),
    /// At least one fatal error occurred.
    Fatal,
}

impl ErrorCounter {
    /// Formats this enum into a readable form.
    ///
    /// Will not allocate if not necessary.
    fn to_str(self) -> Cow<'static, str> {
        match self {
            Self::None => Cow::Borrowed("No errors"),
            Self::NonFatal(NonZeroUsize::MIN) => Cow::Borrowed("1 error"),
            Self::NonFatal(num) => Cow::Owned(format!("{num} errors")),
            Self::Fatal => Cow::Borrowed("Fatal error"),
        }
    }
}

impl Display for ErrorCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_str())
    }
}

/// A widget to render the primary block's contents based on the current
/// [`FileList`].
#[must_use = "render this widget"]
struct ExplorerPrimaryContents<'a>(&'a FileList);

impl Widget for ExplorerPrimaryContents<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let list = self.0;
        let scroll_offset = list.primary_vscroll.borrow().get_position();

        for (display_index, area) in area.rows().enumerate() {
            let Some(entry_idx) = display_index.checked_add(scroll_offset) else {
                // Overflow
                break;
            };

            let Some(entry) = list.entries.get(entry_idx) else {
                // Entry not found
                break;
            };

            let is_selected = entry_idx == list.selected.index;

            let (line_style, spacer_style, filename_style) = if is_selected {
                (
                    EXP_PRIM_STYLE_LINE_SEL,
                    EXP_PRIM_STYLE_SPACER_SEL,
                    EXP_PRIM_STYLE_FILENAME_SEL,
                )
            } else {
                (
                    EXP_PRIM_STYLE_LINE_UNSEL,
                    EXP_PRIM_STYLE_SPACER_UNSEL,
                    EXP_PRIM_STYLE_FILENAME_UNSEL,
                )
            };

            let spacer_text = if is_selected {
                EXP_PRIM_TEXT_SPACER_SEL
            } else {
                EXP_PRIM_TEXT_SPACER_UNSEL
            };

            let spacer = Span::raw(spacer_text).style(spacer_style);
            let filename = Span::raw(entry.name().to_string_lossy()).style(filename_style);

            let line = Line {
                style: line_style,
                spans: vec![spacer, filename],
                alignment: Some(HorizontalAlignment::Left),
            };

            line.render(area, buf);
            // let line = Line::raw(entry.name().to_string_lossy()).;
        }
    }
}

/// A widget to render the secondary block and its contents based on the current
/// [`FileList`].
#[must_use = "render this widget"]
struct ExplorerSecondaryBlock<'a>(&'a FileList);

impl Widget for ExplorerSecondaryBlock<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let list = self.0;

        let Some(selected_entry) = list.entries.get(list.selected.index) else {
            let msg = format!(
                "invariant breached\n\
                this is a bug!\n\
                selection index {idx} >= entry len {len}",
                idx = list.selected.index,
                len = list.entries.len()
            );
            msg.render(area, buf);
            return;
        };

        let title = selected_entry.name().to_string_lossy();
        let rep_hint = if list.selected.rep_metadata.is_ok() {
            EXP_SEC_HINT_IS_REPLAY
        } else {
            EXP_SEC_HINT_NOT_REPLAY
        };
        let rep_hint = Line::from(rep_hint).left_aligned();

        let mut block = Block::bordered()
            .title(title)
            .title_bottom(rep_hint)
            .padding(EXP_SEC_BLOCK_PADDING);

        if area.width >= EXP_SEC_SCROLL_HINT_MIN_BLOCK_WIDTH {
            let line = Line::from(EXP_SEC_HINT_SCROLL).right_aligned();
            block = block.title_bottom(line);
        }

        let inner_area = block.inner(area);

        block.render(area, buf);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        scrollbar.render(area, buf, &mut list.selected.secondary_vscroll.borrow_mut());

        // TODO: Render contents
    }
}
