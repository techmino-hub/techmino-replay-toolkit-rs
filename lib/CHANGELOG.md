## v0.2.0
A massive refactoring:
- BREAKING: Metadata `set_*()` methods now return a new `OwnedTypeError` type as the error variant instead of `serde_json::Value`.
- BREAKING: Metadata `set_*()` methods now accept `T` directly instead of `Option<T>`, for removing values use the new `remove_*()` methods instead.
- BREAKING: Streaming encoding using `ReplayEncoder` reworked
  - Specifically, the constructor was reworked:
    - `.new()` + `.feed_metadata()` is now combined to just `.new()`
      - Therefore `InvalidOperation` error is no longer possible (yay!)
    - `.with_config()` + the NEW `EncoderConfig` used for encoder config instead of many optional parameters
    - Or you can `EncoderConfig::build()`
- BREAKING: `GameInputEvent`'s `serde::Serialize/Deserialize` implementations changed
  - Previously it returned an opaque number e.g. `GameInputEvent(72057594037927937)`
    - This relied on the unstable packed internal representation of game input events.
  - Now it is more descriptive e.g. `GameInputEvent(action: InputAction(kind: Press, key: MoveLeft), frame: 1)`
  - Old `GameInputEvent` serializations are now no longer valid
- DEPRECATION: Deprecated all crate-level type shortcuts. Things are more structured now.
- DEPRECATION: Deprecated the `format` module as its contents has moved elsewhere.
- DEPRECATION: Deprecated the `action` module as it was moved to `replay::action`
- NEW: Metadata `remove_*()` methods.
- NEW: `OwnedTypeError`: variant of `TypeError` that owns the `serde_json::Value` instead of just having a reference.
  - You can use `.inner()` to get its inner `serde_json::Value`
- NEW: Utility methods in `ReplayBufferKind`: `is_binary()`, `is_binary_compressed()`, `is_binary_uncompressed()`, `is_base64()`, and `infer_from_first_byte()`.
- NEW: `Piece` enum to help index into per-piece metadata (e.g. skin color, face)
  - Includes a bit of metadata e.g. amount of minos.
- NEW: `PieceColor` enum to help interpret piece skin color data
  - Get from player settings data using `.get_skin_enum()` instead of `.get_skin()`
  - Default skin color array from the game: `consts::DEFAULT_SKIN_COLORS`
- NEW: `chrono` feature to add `chrono::NaiveDateTime` metadata getters/setters
  - For getting/setting the date the replay was created.
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
  - This new feature flag also enables some other things coupled with the preprocessors in one way or another
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
