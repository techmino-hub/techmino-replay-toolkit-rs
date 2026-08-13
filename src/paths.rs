//! Module for file paths, especially those made by Techmino.
//!
//! # Terminology
//! - Data directory: The data directory as designated by the `dirs` crate.
//! - Save directory: Techmino's save directory, containing the `replay` folder
//!   etc.
//! - Replay directory: The `replay` folder inside Techmino's save directory.

use std::{borrow::Cow, path::PathBuf};

/// The path to Techmino's save directory **relative to the data directory**.
const SAVE_RELPATH: Option<&str> = {
    if cfg!(windows) {
        Some("Techmino")
    } else if cfg!(target_os = "macos") {
        Some("LOVE/Techmino")
    } else if cfg!(target_os = "linux") {
        Some("love/Techmino")
    } else {
        None
    }
};

/// The alternate path to Techmino's save directory **relative to the data
/// directory**, if there is an alternate path.
const ALT_SAVE_RELPATH: Option<&str> = if cfg!(windows) {
    Some(r"LOVE\Techmino")
} else {
    None
};

/// The path to Techmino's replay directory **relative to the save directory**.
const REPLAY_RELPATH: &str = "replay";

/// An abstraction over Techmino's save directory.
#[derive(Clone, Debug)]
pub(crate) struct TechminoSaveDir {
    path: PathBuf,
}

impl TechminoSaveDir {
    /// Try to get this save directory if it exists.
    ///
    /// Returns `None` if this save directory doesn't exist in any of the
    /// expected/checked paths.
    pub fn new() -> Option<Self> {
        let save_relpath = SAVE_RELPATH?;
        let data_dir = dirs::data_dir()?;

        let main_path = data_dir.join(save_relpath);

        if main_path.exists() {
            return Some(Self { path: main_path });
        }

        let alt_relpath = ALT_SAVE_RELPATH?;
        let alt_path = data_dir.join(alt_relpath);

        if alt_path.exists() {
            return Some(Self { path: main_path });
        }

        None
    }

    /// Try to get Techmino's replay directory if it exists.
    ///
    /// Returns `None` if it doesn't.
    pub fn get_replay_dir(&self) -> Option<TechminoReplayDir> {
        let path = self.path.join(REPLAY_RELPATH);

        if path.exists() {
            return Some(TechminoReplayDir { path });
        }

        None
    }
}

/// An abstraction over Techmino's replay directory.
#[derive(Clone, Debug)]
pub(crate) struct TechminoReplayDir {
    path: PathBuf,
}

/// Gets the initial path to start the TUI in, if not overridden.
pub fn get_initial_path() -> PathBuf {
    let Some(dir) = TechminoSaveDir::new() else {
        return get_fallback_start_path();
    };

    dir.get_replay_dir().map(|d| d.path).unwrap_or(dir.path)
}

/// Gets a fallback initial path for when the Techmino replay directory doesn't
/// exist or is inaccessible.
fn get_fallback_start_path() -> PathBuf {
    if let Ok(dir) = std::env::current_dir() {
        return dir;
    }

    if let Some(dir) = dirs::home_dir()
        && dir.exists()
    {
        return dir;
    }

    PathBuf::from("/")
}

/// Trims away the start of a path of a given folder to a given maximum length,
/// prioritizing path separator characters as points for splitting.
#[cfg(feature = "tui")]
pub(crate) fn truncate_folder_path<'a>(path: &'a str, max_len: usize) -> Cow<'a, str> {
    const SEPARATOR: char = std::path::MAIN_SEPARATOR;

    truncate_folder_path_inner::<SEPARATOR>(path, max_len)
}

/// Internal version of the [`trim_folder_path`] function for tests.
fn truncate_folder_path_inner<'a, const SEPARATOR: char>(
    path: &'a str,
    max_len: usize,
) -> Cow<'a, str> {
    /// The length of the `prefix` constant.
    const PREFIX_LEN: usize = 4;
    let prefix = const {
        assert!(
            SEPARATOR as u32 <= u8::MAX as u32,
            "non-ascii path separators are not supported"
        );

        let buffer: &'static [u8] = &const {
            let mut buffer = *b".../";
            buffer[buffer.len() - 1] = SEPARATOR as u8;
            buffer
        };

        assert!(
            buffer.len() == PREFIX_LEN,
            "constant mismatch, update PREFIX_LEN"
        );

        match core::str::from_utf8(buffer) {
            Ok(s) => s,
            Err(_) => panic!("buffer is non-utf8"),
        }
    };

    // Case 1: The path is already short enough
    if path.len() <= max_len {
        return Cow::Borrowed(path);
    }

    // Case 2: Cut off one or more parts, replacing with `...<SEP>`
    let mut path = path;

    while !is_single_part::<SEPARATOR>(path) {
        let Some(next_slash) = path.find(SEPARATOR) else {
            break;
        };

        path = &path[next_slash + 1..];

        let total_len = PREFIX_LEN + path.len();

        if total_len <= max_len {
            let mut string = String::with_capacity(total_len);

            string.push_str(prefix);
            string.push_str(path);

            return Cow::Owned(string);
        }
    }

    // Case 3: Only one part, cut off the start
    let start_idx = path.len().saturating_sub(max_len);
    Cow::Borrowed(&path[start_idx..])
}

/// Returns whether or not a path segment only contains a single part.
///
/// A "single-part" path means that it has only one directory/file name
/// in the string, with no traversals, meaning there aren't any separator
/// characters in the middle of the string.
///
/// Paths ending or starting ("capped") with the separator (e.g. `/my/path/`)
/// behave identically to paths not "capped" with the separator (e.g. `my/path`).
fn is_single_part<const SEPARATOR: char>(path: &str) -> bool {
    let path = path.trim_matches(SEPARATOR);

    path.matches(SEPARATOR).next().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test case for the [`trim_folder_path`] function.
    #[derive(Debug)]
    struct PathTrunCase {
        path_input: &'static str,
        max_len: usize,
        expected_output: &'static str,
    }

    impl PathTrunCase {
        const fn new(
            path_input: &'static str,
            max_len: usize,
            expected_output: &'static str,
        ) -> Self {
            assert!(expected_output.len() <= max_len);

            Self {
                path_input,
                max_len,
                expected_output,
            }
        }

        const fn unix_case_list() -> impl IntoIterator<Item = Self> {
            [
                PathTrunCase::new("/var/lib/hello/world/", 21, "/var/lib/hello/world/"),
                PathTrunCase::new("/var/lib/hello/world/", 20, ".../lib/hello/world/"),
                PathTrunCase::new("/var/lib/hello/world/", 19, ".../hello/world/"),
                PathTrunCase::new("/var/lib/hello/world/", 16, ".../hello/world/"),
                PathTrunCase::new("/var/lib/hello/world/", 15, ".../world/"),
                PathTrunCase::new("/var/lib/hello/world/", 10, ".../world/"),
                PathTrunCase::new("/var/lib/hello/world/", 9, "world/"),
                PathTrunCase::new("/var/lib/hello/world/", 6, "world/"),
                PathTrunCase::new("/var/lib/hello/world/", 5, "orld/"),
                PathTrunCase::new("/var/lib/hello/world/", 4, "rld/"),
                PathTrunCase::new("/var/lib/hello/world/", 3, "ld/"),
                PathTrunCase::new("/var/lib/hello/world/", 2, "d/"),
                PathTrunCase::new("/var/lib/hello/world/", 1, "/"),
                PathTrunCase::new("/var/lib/hello/world/", 0, ""),
                PathTrunCase::new(
                    "/home/gargantuan-folder-name/",
                    29,
                    "/home/gargantuan-folder-name/",
                ),
                PathTrunCase::new(
                    "/home/gargantuan-folder-name/",
                    28,
                    ".../gargantuan-folder-name/",
                ),
                PathTrunCase::new(
                    "/home/gargantuan-folder-name/",
                    27,
                    ".../gargantuan-folder-name/",
                ),
                PathTrunCase::new(
                    "/home/gargantuan-folder-name/",
                    26,
                    "gargantuan-folder-name/",
                ),
                PathTrunCase::new(
                    "/home/gargantuan-folder-name/",
                    23,
                    "gargantuan-folder-name/",
                ),
                PathTrunCase::new(
                    "/home/gargantuan-folder-name/",
                    22,
                    "argantuan-folder-name/",
                ),
                PathTrunCase::new("/home/gargantuan-folder-name/", 21, "rgantuan-folder-name/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 20, "gantuan-folder-name/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 19, "antuan-folder-name/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 18, "ntuan-folder-name/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 17, "tuan-folder-name/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 16, "uan-folder-name/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 15, "an-folder-name/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 14, "n-folder-name/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 13, "-folder-name/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 12, "folder-name/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 11, "older-name/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 10, "lder-name/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 9, "der-name/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 8, "er-name/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 7, "r-name/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 6, "-name/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 5, "name/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 4, "ame/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 3, "me/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 2, "e/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 1, "/"),
                PathTrunCase::new("/home/gargantuan-folder-name/", 0, ""),
                PathTrunCase::new("/var/lib/hello/world", 20, "/var/lib/hello/world"),
                PathTrunCase::new("/var/lib/hello/world", 19, ".../lib/hello/world"),
                PathTrunCase::new("/var/lib/hello/world", 18, ".../hello/world"),
                PathTrunCase::new("/var/lib/hello/world", 15, ".../hello/world"),
                PathTrunCase::new("/var/lib/hello/world", 14, ".../world"),
                PathTrunCase::new("/var/lib/hello/world", 9, ".../world"),
                PathTrunCase::new("/var/lib/hello/world", 8, "world"),
                PathTrunCase::new("/var/lib/hello/world", 5, "world"),
                PathTrunCase::new("/var/lib/hello/world", 4, "orld"),
                PathTrunCase::new("/var/lib/hello/world", 3, "rld"),
                PathTrunCase::new("/var/lib/hello/world", 2, "ld"),
                PathTrunCase::new("/var/lib/hello/world", 1, "d"),
                PathTrunCase::new("/var/lib/hello/world", 0, ""),
            ]
        }

        const fn windows_case_list() -> impl IntoIterator<Item = Self> {
            [
                PathTrunCase::new(
                    r"C:\Testcases\Examples\Folder",
                    28,
                    r"C:\Testcases\Examples\Folder",
                ),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder", 27, r"...\Examples\Folder"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder", 19, r"...\Examples\Folder"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder", 18, r"...\Folder"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder", 10, r"...\Folder"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder", 9, r"Folder"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder", 6, r"Folder"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder", 5, r"older"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder", 4, r"lder"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder", 3, r"der"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder", 2, r"er"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder", 1, r"r"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder", 0, r""),
                PathTrunCase::new(
                    r"C:\Testcases\Examples\Folder\",
                    29,
                    r"C:\Testcases\Examples\Folder\",
                ),
                PathTrunCase::new(
                    r"C:\Testcases\Examples\Folder\",
                    28,
                    r"...\Examples\Folder\",
                ),
                PathTrunCase::new(
                    r"C:\Testcases\Examples\Folder\",
                    20,
                    r"...\Examples\Folder\",
                ),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder\", 19, r"...\Folder\"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder\", 11, r"...\Folder\"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder\", 10, r"Folder\"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder\", 7, r"Folder\"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder\", 6, r"older\"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder\", 5, r"lder\"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder\", 4, r"der\"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder\", 3, r"er\"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder\", 2, r"r\"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder\", 1, r"\"),
                PathTrunCase::new(r"C:\Testcases\Examples\Folder\", 0, r""),
            ]
        }
    }

    #[test]
    fn truncate_folder_path() {
        for case in PathTrunCase::unix_case_list() {
            let result = truncate_folder_path_inner::<'/'>(case.path_input, case.max_len);

            assert_eq!(
                &*result, case.expected_output,
                "mismatch in test case {case:?}"
            );
        }

        for case in PathTrunCase::windows_case_list() {
            let result = truncate_folder_path_inner::<'\\'>(case.path_input, case.max_len);

            assert_eq!(
                &*result, case.expected_output,
                "mismatch in test case {case:?}"
            );
        }
    }

    #[test]
    fn is_single_part() {
        let unix_cases = [
            ("usr/local/bin/techmino", false),
            ("/usr/local/bin/techmino", false),
            ("usr/local/bin/techmino/", false),
            ("/usr/local/bin/techmino/", false),
            ("bin/techmino", false),
            ("/bin/techmino", false),
            ("bin/techmino/", false),
            ("/bin/techmino/", false),
            ("techmino", true),
            ("/techmino", true),
            ("techmino/", true),
            ("/techmino/", true),
        ];

        let windows_cases = [
            (r"C:\Games\Techmino", false),
            (r"C:\Games\Techmino\", false),
            (r"Games\Techmino", false),
            (r"\Games\Techmino", false),
            (r"Games\Techmino\", false),
            (r"\Games\Techmino\", false),
            (r"Techmino", true),
            (r"\Techmino", true),
            (r"Techmino\", true),
            (r"\Techmino\", true),
            (r"C:\Games", false),
            (r"C:\Games\", false),
        ];

        for (path, expected) in unix_cases {
            let result = super::is_single_part::<'/'>(path);
            assert_eq!(result, expected);
        }

        for (path, expected) in windows_cases {
            let result = super::is_single_part::<'\\'>(path);
            assert_eq!(result, expected);
        }
    }
}
