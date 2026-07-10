#!/usr/bin/env bash

# A script to thoroughly test techmino-replay-toolkit (the binary crate).

echo "checking dependencies..."

deps_met=1

if ! command -v cargo; then
    echo "cargo: command not found"
    deps_met=0
fi

if [ $deps_met == 0 ]; then
    echo "one or more dependencies missing"
    exit 1
fi

# e.g. ~/Code/techmino-replay-toolkit-rs
scripts_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

# Start many scripts asynchronously
echo "running tests..."

(
    fmt_result="$(cargo fmt --all -- --check 2>&1)"
    if [ $? != 0 ]; then
        echo -e "===== Fmt failed =====\n$fmt_result"
        exit 1
    fi
) &

pids=($!)

while read featureset; do
    (
        clippy_result="$(cargo clippy -p techmino-replay-toolkit --no-default-features $featureset --keep-going -- -D warnings 2>&1)";
        exitcode="$?"
        if [ "$exitcode" != 0 ]; then
            echo -e "===== Clippy failure with featureset $featureset exited with code $exitcode =====\n$clippy_result"
            exit 1
        fi

        test_result="$(cargo test -p techmino-replay-toolkit --no-default-features $featureset --no-fail-fast 2>&1)";
        exitcode="$?"
        if [ "$exitcode" != 0 ]; then
            echo -e "===== Test failure with featureset $featureset exited with code $exitcode =====\n$test_result"
            exit 1
        fi

        export RUSTDOCFLAGS="$RUSTDOCFLAGS -D warnings"
        doc_result="$(cargo doc -p techmino-replay-toolkit --no-default-features $featureset --no-deps --document-private-items --keep-going 2>&1)";
        exitcode="$?"
        if [ "$exitcode" != 0 ]; then
            echo -e "===== Doc failure with featureset $featureset exited with code $exitcode =====\n$doc_result"
            exit 1
        fi
    ) &

    pids+=("$!")
done << FEATURES_END

FEATURES_END

echo "all tests launched, awaiting completion..."
echo "pids launched: ${pids[@]}"

success=1

for pid in "${pids[@]}"
do
    echo "waiting for pid $pid"
    wait $pid
    exitcode=$?

    if [ $exitcode != 0 ]; then
        success=0
        echo "pid $pid exited with exit code $exitcode"
    fi
done

if [ $success != 1 ]; then
    echo "long test failed"
    exit 1
else
    echo "long test succeeded"
    exit 0
fi
