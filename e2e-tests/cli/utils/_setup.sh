#!/usr/bin/env bash

# Used for setting up other testcases.

shopt -s nullglob
set -euo pipefail

repo_dir=$( git -C "$script_dir" rev-parse --show-toplevel )
testcases_dir="$repo_dir/testcases"
target_dir="$repo_dir/target"
mkdir -p "$target_dir"
tmpdir="$target_dir/e2e-tests-tmp"
mkdir -p "$tmpdir"
tmpdir=$(mktemp -d -p "$tmpdir")
verbosity=2


while [[ $# -gt 0 ]]; do
    case $1 in
        --quiet)
            verbosity=1
            shift
            ;;
        --verbose)
            verbosity=3
            shift
            ;;
    esac
done


cargo_verbosity_arg=""

pushd "$repo_dir" >/dev/null

if [[ "$verbosity" -gt 2 ]]; then
    echo "(setup) Setting up the test"
    echo "(setup) pushd to $repo_dir"
    cargo_verbosity_arg="--verbose"
elif [[ "$verbosity" -lt 2 ]]; then
    cargo_verbosity_arg="--quiet"
fi
