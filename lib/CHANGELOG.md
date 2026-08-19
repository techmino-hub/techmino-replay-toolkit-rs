## v0.2.0
A massive refactoring:
- BREAKING: Metadata `set_*()` methods now return a new `OwnedTypeError` type as the error variant instead of `serde_json::Value`.
- BREAKING: Metadata `set_*()` methods now accept `T` directly instead of `Option<T>`, for removing values use the new `remove_*()` methods instead.
- DEPRECATION: Deprecated all crate-level type shortcuts. Things are more structured now.
- DEPRECATION: Deprecated the `format` module as its contents has moved elsewhere.
- DEPRECATION: Deprecated the `action` module as it was moved to `replay::action`
- NEW: Metadata `remove_*()` methods.
- NEW: `OwnedTypeError`: variant of `TypeError` that owns the `serde_json::Value` instead of just having a reference.
  - You can use `.inner()` to get its inner `serde_json::Value`
- Restructured the codebase. Now things are much better structured:
  - `replay` for replay data structs
  - `config` for encoder/decoder config-related structs
  - `errors` for error structs/enums
- `TypeError` now has a reference to the `serde_json::Value`, making the metadata `get_*_or_raw()` methods a simple shortcut to `get_*().inner()`.
- Improved `TypeError` `Display` impl to show the expected and actual types.
- Improved `GameInputEventError`'s `Display` impl to show the frame number and its maximum
- Improved documentation on metadata getter and setter functions.
- Updated documentation on unsorted inputs
- Updated documentation on compression levels (now `1` is recommended)
- Documentation "Input Parse Mode" sections now link to `InputParseMode`
- (Unstable) new: "Preprocessors" API exposing internal structs for reading compressed/base64 streams.
- (Unstable) new: "Postprocessors" API exposing internal structs for creating compressed/base64 streams.
  - Beware, unstable means this can change at any time in between patch versions!
- A LOT of internal changes.
<!--
Internal Changes:
- Internal: Refactored metadata method definitions into one place instead of two.
- Internal: Restructured codebase to make the modules smaller
-->

## v0.1.0
Initial release.
