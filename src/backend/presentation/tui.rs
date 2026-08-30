//! Turns replay data structures into a more presentable form, specifically
//! tailored for the TUI.

use std::{borrow::Cow, collections::vec_deque::VecDeque};

use libtechmino_replay::replay::GameReplayMetadata;
use serde_json::{Map, Number, Value as JsonValue};

/// A more presentable, slightly-flattened form of [`GameReplayMetadata`].
#[must_use = "Present this meta"]
#[derive(Debug)]
pub(crate) struct TuiPresentableMeta(pub(crate) Vec<TuiPresentableMetaEntry>);

/// An entry in a [`TuiPresentableMeta`].
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TuiPresentableMetaEntry {
    pub(crate) key: String,
    pub(crate) value: Cow<'static, str>,
    pub(crate) json_type: TuiPresentableMetaEntryKind,
}

/// One of the possible JSON types to end up in a [`TuiPresentableMetaEntry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TuiPresentableMetaEntryKind {
    Null,
    Bool,
    Number,
    String,
    Array,
}

impl TuiPresentableMetaEntry {
    fn null(key: String) -> Self {
        TuiPresentableMetaEntry {
            key,
            value: "null".into(),
            json_type: TuiPresentableMetaEntryKind::Null,
        }
    }

    fn bool(key: String, bool: bool) -> Self {
        let text = if bool { "true" } else { "false" };

        TuiPresentableMetaEntry {
            key,
            value: text.into(),
            json_type: TuiPresentableMetaEntryKind::Bool,
        }
    }

    fn number(key: String, number: &Number) -> Self {
        TuiPresentableMetaEntry {
            key,
            value: format!("{number}").into(),
            json_type: TuiPresentableMetaEntryKind::Number,
        }
    }

    fn string(key: String, string: &str) -> Self {
        TuiPresentableMetaEntry {
            key,
            value: format!("\"{string}\"").into(),
            json_type: TuiPresentableMetaEntryKind::String,
        }
    }

    fn array(key: String, values: &[JsonValue]) -> Self {
        let text = match serde_json::to_string(values) {
            Ok(s) => s,
            Err(e) => format!("<!> Error converting array to string: {e}"),
        };

        TuiPresentableMetaEntry {
            key,
            value: text.into(),
            json_type: TuiPresentableMetaEntryKind::Array,
        }
    }
}

impl From<&GameReplayMetadata> for TuiPresentableMeta {
    fn from(value: &GameReplayMetadata) -> Self {
        let mut to_visit: VecDeque<(String, &JsonValue)> =
            value.map.iter().map(|(k, v)| (k.to_owned(), v)).collect();
        let mut presentable: Vec<TuiPresentableMetaEntry> = Vec::new();

        while let Some((key, value)) = to_visit.pop_front() {
            match value {
                JsonValue::Null => {
                    presentable.push(TuiPresentableMetaEntry::null(key));
                }
                JsonValue::Bool(bool) => {
                    presentable.push(TuiPresentableMetaEntry::bool(key, *bool));
                }
                JsonValue::Number(number) => {
                    presentable.push(TuiPresentableMetaEntry::number(key, number));
                }
                JsonValue::String(string) => {
                    presentable.push(TuiPresentableMetaEntry::string(key, string));
                }
                JsonValue::Array(values) => {
                    presentable.push(TuiPresentableMetaEntry::array(key, values));
                }
                JsonValue::Object(map) => {
                    push_visits(&mut to_visit, &key, map);
                }
            };
        }

        Self(presentable)
    }
}

/// Append an object's members to `to_visit`.
fn push_visits<'a>(
    to_visit: &mut VecDeque<(String, &'a JsonValue)>,
    prefix: &str,
    map: &'a Map<String, JsonValue>,
) {
    for (suffix, value) in map {
        let key = format!("{prefix}.{suffix}");
        to_visit.push_back((key, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentable_meta() {
        let mut metadata = GameReplayMetadata::new();
        metadata.set_private(serde_json::json!({
            "hello": "world",
            "numbers": [1, 2, 3],
            "letters": ['a', 'b', 'c', 'd'],
            "delete": 'b',
            "isNull": null,
            "whoa": [],
            "": "???",
            "...": "ruh roh"
        }));
        metadata.set_tas_used(true);
        metadata.set_seed(u64::MAX);

        let meta = TuiPresentableMeta::from(&metadata);

        let entries = [
            TuiPresentableMetaEntry {
                key: "tasUsed".into(),
                value: "true".into(),
                json_type: TuiPresentableMetaEntryKind::Bool,
            },
            TuiPresentableMetaEntry {
                key: "seed".into(),
                value: "18446744073709551615".into(),
                json_type: TuiPresentableMetaEntryKind::Number,
            },
            TuiPresentableMetaEntry {
                key: "private.hello".into(),
                value: "\"world\"".into(),
                json_type: TuiPresentableMetaEntryKind::String,
            },
            TuiPresentableMetaEntry {
                key: "private.numbers".into(),
                value: "[1,2,3]".into(),
                json_type: TuiPresentableMetaEntryKind::Array,
            },
            TuiPresentableMetaEntry {
                key: "private.letters".into(),
                value: "[\"a\",\"b\",\"c\",\"d\"]".into(),
                json_type: TuiPresentableMetaEntryKind::Array,
            },
            TuiPresentableMetaEntry {
                key: "private.delete".into(),
                value: "\"b\"".into(),
                json_type: TuiPresentableMetaEntryKind::String,
            },
            TuiPresentableMetaEntry {
                key: "private.isNull".into(),
                value: "null".into(),
                json_type: TuiPresentableMetaEntryKind::Null,
            },
            TuiPresentableMetaEntry {
                key: "private.whoa".into(),
                value: "[]".into(),
                json_type: TuiPresentableMetaEntryKind::Array,
            },
            TuiPresentableMetaEntry {
                key: "private.".into(),
                value: "\"???\"".into(),
                json_type: TuiPresentableMetaEntryKind::String,
            },
            TuiPresentableMetaEntry {
                key: "private....".into(),
                value: "\"ruh roh\"".into(),
                json_type: TuiPresentableMetaEntryKind::String,
            },
        ];

        for entry in &entries {
            assert!(meta.0.contains(entry))
        }

        for entry in &meta.0 {
            assert!(entries.contains(entry))
        }
    }
}
