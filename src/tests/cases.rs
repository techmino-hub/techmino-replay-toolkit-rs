use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::GameReplayData;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoredReplay {
    Base64(String),
    Binary(Box<[u8]>),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TestCase {
    pub serialized: Option<StoredReplay>,
    pub data: Option<GameReplayData>
}

pub fn get_test_cases() -> HashMap<String, TestCase> {
    let files: Vec<_> = std::fs::read_dir("./src/tests/cases").unwrap().flatten().collect();

    let mut map: HashMap<String, TestCase> = HashMap::new();

    for file in files {
        let name = file.file_name().into_string()
            .expect("Invalid Unicode in test case file name");
        let (name, ext) = name.split_once('.')
            .expect("Invalid test case file name (no extension)");

        let contents = std::fs::read(file.path())
            .expect(&format!("Error reading test case {name} content"));

        match ext.to_ascii_lowercase().as_str() {
            "b64.rep" => {
                let case = map.get_mut(name);

                let contents = String::from_utf8(contents)
                    .expect(&format!("Invalid Unicode in test case {name} contents"));

                let stored = StoredReplay::Base64(contents);

                if let Some(c) = case {
                    c.serialized = Some(stored);
                } else {
                    map.insert(name.to_string(), TestCase {
                        serialized: Some(stored),
                        data: None,
                    });
                }
            },
            "bin.rep" => {
                let case = map.get_mut(name);

                let stored = StoredReplay::Binary(contents.into_boxed_slice());

                if let Some(c) = case {
                    c.serialized = Some(stored);
                } else {
                    map.insert(name.to_string(), TestCase {
                        serialized: Some(stored),
                        data: None,
                    });
                }
            },
            "ron" | "dat" | "res" => {
                let case = map.get_mut(name);

                let contents = String::from_utf8(contents)
                    .expect(&format!("Invalid Unicode in test case {name} data"));

                let data: GameReplayData = ron::from_str(&contents)
                    .expect(&format!("Invalid RON in test case {name} data"));

                if let Some(c) = case {
                    c.data = Some(data);
                } else {
                    map.insert(name.to_string(), TestCase {
                        data: Some(data),
                        serialized: None,
                    });
                }
            }
            _ => panic!("Unknown file extension .{ext} for test {name}"),
        }
    }

    map
}