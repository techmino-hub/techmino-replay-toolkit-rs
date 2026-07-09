# `libtechmino-replay`

The Rust library behind `techmino-replay-toolkit`.

- Used for parsing and serializing Techmino replays
- Has support for streaming data
- `#![no_std]` support (`--no-default-features --features alloc`)
  - However, we require `alloc`
- Based on `miniz_oxide`

# Minimum Supported Rust Version
**Current MSRV: `1.87.0`**

MSRV bumps are out of scope of semver changes. This means it may be changed across minor or patch updates.
