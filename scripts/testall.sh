#!/usr/bin/env bash

# A script to thoroughly test the entire workspace.

echo "checking dependencies..."

deps_met=1

if ! command -v cargo; then
    echo "cargo: command not found"
    deps_met=0
fi

if ! cargo msrv --version >/dev/null; then
    echo "cargo-msrv is not installed"
    deps_met=0
fi

if ! cargo deny --version >/dev/null; then
    echo "cargo-deny is not installed"
    deps_met=0
fi

if [ $deps_met == 0 ]; then
    echo "////////// one or more dependencies missing //////////"
    exit 1
fi

script_dir=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
repo_dir=$(dirname "$script_dir")

echo "[][][][][] Testing libtechmino-vlq [][][][][]"
bash "$repo_dir/vlq/scripts/long-test.sh"

exitcode="$?"
if [ "$exitcode" != 0 ]; then
    echo "////////// script exited with code $exitcode //////////"
    exit "$exitcode"
fi

echo "[][][][][] Testing libtechmino-replay [][][][][]"
bash "$repo_dir/lib/scripts/long-test.sh"

exitcode="$?"
if [ "$exitcode" != 0 ]; then
    echo "////////// script exited with code $exitcode //////////"
    exit "$exitcode"
fi

echo "[][][][][] Testing techmino-replay-toolkit [][][][][]"
bash "$script_dir/long-test.sh"

exitcode="$?"
if [ "$exitcode" != 0 ]; then
    echo "////////// script exited with code $exitcode //////////"
    exit "$exitcode"
fi

echo "[][][][][] cargo-deny [][][][][]"
cargo deny --workspace check

exitcode="$?"
if [ "$exitcode" != 0 ]; then
    echo "////////// deny exited with code $exitcode //////////"
    exit "$exitcode"
fi

echo "[][][][][] all good! [][][][][]"
