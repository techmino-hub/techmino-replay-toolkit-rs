# Techmino Replay Toolkit

This is a toolkit for serializing and deserializing Techmino replays, written in Rust.

Includes:
- `lib/`: Library crate `libtechmino-replay` for all your replay parsing and serializing needs
- `vlq/`: Library crate `libtechmino-vlq`, used by `libtechmino-replay` for handling VLQs
- `src/`: Binary CLI REPL crate `techmino-replay-toolkit` as a user-friendly frontend

## A note on stability

The `techmino-replay-toolkit` codebase in `src/` should be treated as **unstable**; that is,
internal code may change at any time! **If you want to make your own utility for Techmino
replays, please use `libtechmino-replay` instead!**

## Download the Libraries

If you're a developer, you can get the `libtechmino-replay` library here!

<https://crates.io/crates/libtechmino-replay>

## Download the Binaries

For regular users, there isn't a Techmino Replay Toolkit executable binary available
just yet. You'll need to wait!
