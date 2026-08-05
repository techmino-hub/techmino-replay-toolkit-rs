use core::char::UNICODE_VERSION;
use std::{
    ffi::{OsStr, OsString},
    fs::{self, Metadata},
    io,
    path::{self, Path, PathBuf},
};

/// Represents the state of the interface.
pub(in crate::tui) enum UiState {
    /// The explorer menu, where the user can traverse directories and
    /// select a file.
    Explorer(ExplorerState),
    /// The operations menu, where the user can select an operation to perform on
    /// that file.
    Operations(core::convert::Infallible),
}

impl UiState {
    /// Initializes the UI state given a certain initialization path.
    ///
    /// If the path points to a file, initializes at the operations menu.
    /// Otherwise, initializes at the explorer menu.
    pub fn new(path: &Path) -> Self {
        if path.is_dir() {
            Self::Explorer(ExplorerState::new(path))
        } else {
            Self::Operations(todo!())
        }
    }
}

/// Represents the state of the explorer menu.
pub(in crate::tui) struct ExplorerState {
    /// The current folder being explored.
    pub(in crate::tui) folder: PathBuf,
    /// The (cached) file list currently being shown, or an I/O error
    pub(in crate::tui) file_list: io::Result<FileList>,
}

impl ExplorerState {
    /// Creates a new explorer state based on the specified directory path.
    pub fn new(folder: &Path) -> Self {
        let folder = folder.canonicalize().unwrap_or_else(|_| folder.to_owned());

        let file_list = FileList::new(&folder);

        Self { folder, file_list }
    }
}

/// Represents a cache of the list of files shown in the interface.
pub(in crate::tui) struct FileList {
    /// The list of file entries shown in the current directory.
    pub(in crate::tui) entries: Vec<UiDirEntry>,
    /// The index of the currently-highlighted file.
    pub(in crate::tui) selected_index: usize,
    /// The number of non-fatal I/O errors that occurred while traversing the
    /// directory.
    pub(in crate::tui) non_fatal_errors: usize,
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

        for entry in fs::read_dir(folder)? {
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

        Ok(Self {
            entries,
            selected_index: 0,
            non_fatal_errors,
        })
    }
}

/// Represents a cache of a directory entry shown in the interface.
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
            UiDirEntry::Regular { name, metadata: _ } => &name,
            UiDirEntry::ParentDir { metadata: _ } => &OsStr::new(".."),
        }
    }
}
