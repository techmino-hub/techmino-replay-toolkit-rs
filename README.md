# Techmino Replay Toolkit

This is a toolkit for serializing and deserializing Techmino replays, written in Rust.

Includes:
- `lib/`: Library crate `libtechmino-replay` for all your replay parsing and serializing needs
- `vlq/`: Library crate `libtechmino-vlq`, used by `libtechmino-replay` for handling VLQs
- `src/`: Binary CLI/TUI crate `techmino-replay-toolkit` as a user-friendly frontend

## A note on stability

The `techmino-replay-toolkit` codebase in `src/` should be treated as **unstable**; that is,
internal code may change at any time! **If you want to make your own utility for Techmino
replays, please use `libtechmino-replay` instead!**

## Download the Libraries

If you're a developer, you can get the `libtechmino-replay` library here!

<https://crates.io/crates/libtechmino-replay>

## Download the Binaries

For regular users, the Techmino Replay Toolkit is available as an executable **[in the Releases page](releases)**. **It is primarily a terminal app; make sure to open it in a terminal (CMD/powershell/bash)!** Nothing will happen if you open it outside a terminal.

Get started:
```
# If on Linux
./techmino-replay-toolkit --help

# If on Windows
techmino-replay-toolkit.exe --help
```
