# Techmino Replay Toolkit

This is a toolkit for serializing and deserializing Techmino replays, written in Rust.

Includes:
- `lib/`: Library crate `libtechmino-replay` for all your replay parsing and serializing needs
- `vlq/`: Library crate `libtechmino-vlq`, used by `libtechmino-replay` for handling VLQs
- `src/`: Binary CLI REPL crate `techmino-replay-toolkit` as a user-friendly frontend
