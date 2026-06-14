use crate::GameReplayData;
use serde::{Deserialize, Serialize};
use std::{boxed::Box, collections::HashMap, fs::DirEntry, string::String, vec::Vec};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoredReplay {
    Base64(String),
    Binary(Box<[u8]>),
}

#[derive(Debug, PartialEq, Default)]
pub struct TestCase {
    pub serialized: Option<StoredReplay>,
    pub data: Option<GameReplayData>,
}

pub const TESTCASE_PATH: &str = "./src/test_utils/cases";

const EXTENSION_B64_REPLAY: &str = "b64.rep";
const EXTENSION_BIN_REPLAY: &str = "bin.rep";
const EXTENSION_RON: &str = "ron";

#[must_use]
pub fn get_test_cases() -> HashMap<String, TestCase> {
    let files: Vec<_> = std::fs::read_dir(TESTCASE_PATH)
        .unwrap()
        .flatten()
        .collect();

    let mut map: HashMap<String, TestCase> = HashMap::new();

    for file in files {
        process_testcase_file(&file, &mut map);
    }

    map
}

fn process_testcase_file(file: &DirEntry, map: &mut HashMap<String, TestCase>) {
    let name = file
        .file_name()
        .into_string()
        .expect("Invalid Unicode in test case file name");
    let (basename, ext) = name
        .split_once('.')
        .expect("Invalid test case file name (no extension)");

    let contents = std::fs::read(file.path())
        .unwrap_or_else(|_| panic!("Error reading test case {basename} content"));

    match ext.to_ascii_lowercase().as_str() {
        EXTENSION_B64_REPLAY => process_b64_file(basename, contents, map),
        EXTENSION_BIN_REPLAY => process_bin_file(basename, contents, map),
        EXTENSION_RON => process_ron_file(basename, contents, map),
        _ => panic!("Unknown file extension .{ext} for test {basename}"),
    }
}

fn process_b64_file(basename: &str, contents: Vec<u8>, map: &mut HashMap<String, TestCase>) {
    let contents = String::from_utf8(contents)
        .unwrap_or_else(|_| panic!("Invalid Unicode in test case {basename} contents"));

    let stored = StoredReplay::Base64(contents);

    map.entry(basename.to_owned()).or_default().serialized = Some(stored);
}

fn process_bin_file(basename: &str, contents: Vec<u8>, map: &mut HashMap<String, TestCase>) {
    let stored = StoredReplay::Binary(contents.into_boxed_slice());

    map.entry(basename.to_owned()).or_default().serialized = Some(stored);
}

fn process_ron_file(basename: &str, contents: Vec<u8>, map: &mut HashMap<String, TestCase>) {
    // let case = map.get_mut(basename);

    let contents = String::from_utf8(contents)
        .unwrap_or_else(|_| panic!("Invalid Unicode in test case {basename} data"));

    let data: GameReplayData = ron::from_str(&contents)
        .unwrap_or_else(|_| panic!("Invalid RON in test case {basename} data"));

    map.entry(basename.to_owned()).or_default().data = Some(data);
}
