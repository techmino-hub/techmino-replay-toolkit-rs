use std::{
    ffi::{OsStr, OsString},
    fs::{self, Metadata},
    io,
    path::{self, Path, PathBuf},
};

use libtechmino_replay::GameReplayMetadata;
use ratatui::prelude::Frame;

use crate::tui::ParseOrIoError;

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

    /// Returns the (cached) file list to show, or an I/O error if a fatal error
    /// occurred.
    pub(in crate::tui) fn file_list(&self) -> &io::Result<FileList> {
        &self.file_list
    }

    /// Returns the current path being shown.
    pub(in crate::tui) fn folder(&self) -> &Path {
        &self.folder
    }

    /// Explore one layer up.
    pub(in crate::tui) fn up(&mut self) {
        let Some(new_folder) = self.folder().parent() else {
            return;
        };

        self.folder = new_folder.to_owned();

        self.file_list = FileList::new(self.folder());
    }

    /// Move the cursor to the previous item.
    pub(in crate::tui) fn prev(&mut self) {
        let Ok(file_list) = &mut self.file_list else {
            return;
        };

        let index = &mut file_list.selected.index;

        *index = index.saturating_sub(1);

        file_list
            .selected
            .update_metadata(&self.folder, &file_list.entries);
    }

    /// Move the cursor to the next item.
    pub(in crate::tui) fn next(&mut self) {
        let Ok(file_list) = &mut self.file_list else {
            return;
        };

        let len = file_list.entries.len();
        let index = &mut file_list.selected.index;

        *index = index.saturating_add(1).min(len);

        file_list
            .selected
            .update_metadata(&self.folder, &file_list.entries);
    }

    /// Move the cursor to the first item.
    pub(in crate::tui) fn first(&mut self) {
        let Ok(file_list) = &mut self.file_list else {
            return;
        };

        file_list.selected.index = 0;

        file_list
            .selected
            .update_metadata(&self.folder, &file_list.entries);
    }

    /// Move the cursor to the last item.
    pub(in crate::tui) fn last(&mut self) {
        let Ok(file_list) = &mut self.file_list else {
            return;
        };

        file_list.selected.index = file_list.entries().len() - 1;

        file_list
            .selected
            .update_metadata(&self.folder, &file_list.entries);
    }

    /// Selects the currently-highlighted item.
    ///
    /// # Returns
    /// Returns instructions on the next UI state after this selection.
    pub(in crate::tui) fn select(mut self) -> SelectionResult {
        let Ok(file_list) = &self.file_list else {
            return SelectionResult::Explorer(self);
        };

        let Some(entry) = file_list.entries.get(file_list.selected.index) else {
            return SelectionResult::Explorer(self);
        };

        let path = entry.resolve(&self.folder);

        if path.is_file() {
            let instr = GoToOperations {
                ret_path: self.folder,
                file_path: path,
            };
            return SelectionResult::Operations(instr);
        }

        SelectionResult::Explorer(Self::new(&path))
    }

    pub(in crate::tui::scenes) fn render(&self, frame: &mut Frame) {
        frame.render_widget("TODO: Explorer rendering\n{self:?}", frame.area());
    }
}

/// Represents a cache of the list of files shown in the interface.
#[derive(Debug)]
pub(in crate::tui) struct FileList {
    /// The list of file entries shown in the current directory.
    entries: Vec<UiDirEntry>,
    /// Information about the currently-selected file.
    selected: SelectedFile,
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
            if let Ok(ftype) = entry.file_type()
                && ftype.is_dir()
            {
                name.push(path::MAIN_SEPARATOR_STR);
            }

            let metadata = entry.metadata().ok();

            let entry = UiDirEntry::Regular { name, metadata };

            entries.push(entry);
        }

        let selected = SelectedFile::new(&folder, &entries);

        Ok(Self {
            entries,
            selected,
            non_fatal_errors,
        })
    }

    /// Gets the list of file entries shown in the current directory.
    pub(in crate::tui) fn entries(&self) -> &[UiDirEntry] {
        &self.entries
    }

    /// Information about the currently-selected file.
    pub(in crate::tui) fn selected(&self) -> &SelectedFile {
        &self.selected
    }

    /// Returns the number of non-fatal I/O errors that occurred while
    /// traversing the directory.
    pub(in crate::tui) fn non_fatal_errors(&self) -> usize {
        self.non_fatal_errors
    }
}

/// Contains internal data about the selected file.
#[derive(Debug)]
pub(in crate::tui) struct SelectedFile {
    /// The index of the file which is selected.
    index: usize,
    /// The replay-specific metadata of that file if it parses correctly.
    rep_metadata: Result<GameReplayMetadata, ParseOrIoError>,
}

impl SelectedFile {
    /// Create a new selected file state pointing to the first entry.
    ///
    /// # Parameters
    /// - `folder`: The directory containing the given directory entries.
    /// - `entries`: The list of directory entries in the `folder` argument.
    pub(in crate::tui::scenes) fn new(folder: &Path, entries: &[UiDirEntry]) -> Self {
        let mut this = Self {
            index: 0,
            rep_metadata: Err(ParseOrIoError::Io(io::Error::other(""))),
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
        name: OsString,
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
    pub(in crate::tui) fn name(&self) -> &OsStr {
        match self {
            UiDirEntry::Regular { name, metadata: _ } => name,
            UiDirEntry::ParentDir { metadata: _ } => OsStr::new("../"),
        }
    }

    /// Resolves this entry into a full path given a certain parent directory.
    pub(in crate::tui) fn resolve(&self, folder: &Path) -> PathBuf {
        match self {
            UiDirEntry::Regular { name, metadata: _ } => folder.join(name),
            UiDirEntry::ParentDir { metadata: _ } => folder.parent().unwrap_or(folder).to_owned(),
        }
    }
}

/// Represents the next UI state as a result of selecting a directory entry.
#[must_use]
#[derive(Debug)]
pub(in crate::tui) enum SelectionResult {
    /// Go to another explorer state.
    Explorer(ExplorerScene),
    /// Move to the operations scene.
    Operations(GoToOperations),
}

/// A struct representing an instruction to navigate to the operations scene, and
/// any associated/related data.
#[derive(Debug)]
pub(in crate::tui) struct GoToOperations {
    /// The path to return to later when revisiting the explorer scene.
    ret_path: PathBuf,
    /// The full path of the selected file to give to the operations scene.
    file_path: PathBuf,
}

impl GoToOperations {
    /// Returns the path to return to later when revisiting the explorer scene.
    ///
    /// This should be saved for later.
    pub(in crate::tui) fn ret_path(&self) -> &Path {
        &self.ret_path
    }

    /// Returns the full path of the selected file to give to the operations scene.
    pub(in crate::tui) fn file_path(&self) -> &Path {
        &self.file_path
    }
}
