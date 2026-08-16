## v0.2.0
A massive refactoring:
- A LOT of internal changes.
- BREAKING: Metadata `set_*()` methods now return a new `OwnedTypeError` type as the error variant instead of `serde_json::Value`.
  - This new `TypeError` contains contains not just `serde_json::Value`
- BREAKING: Metadata `set_*()` methods now accept `T` directly instead of `Option<T>`, for removing values use the new `remove_*()` methods instead.
- NEW: Metadata `remove_*()` methods.
- `TypeError` now has a reference to the `serde_json::Value`, making the metadata `get_*_or_raw()` methods a simple shortcut to `get_*().inner()`.
- Improved `TypeError` `Display` impl.
- Improved documentation on metadata getter and setter functions.
- Updated documentation on unsorted inputs
- Updated documentation on compression levels (now `1` is recommended)
- Documentation "Input Parse Mode" sections now link to `InputParseMode`
<!--
Internal Changes:
- Internal: Refactored metadata method definitions into one place instead of two.
-->

## v0.1.0
Initial release.
