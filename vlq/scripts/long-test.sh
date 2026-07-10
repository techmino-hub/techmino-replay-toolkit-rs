#!/usr/bin/env bash

# A script to thoroughly test libtechmino-vlq.

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

if [ $deps_met == 0 ]; then
    echo "one or more dependencies missing"
    exit 1
fi

scripts_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
vlq_dir=$(dirname "$scripts_dir")

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
        clippy_result="$(cargo clippy --lib -p libtechmino-vlq --no-default-features $featureset --keep-going -- -D warnings 2>&1)";
        exitcode="$?"
        if [ "$exitcode" != 0 ]; then
            echo -e "===== Clippy failure with featureset $featureset exited with code $exitcode =====\n$clippy_result"
            exit 1
        fi

        # Tests assume std, so skip if we use alloc instead
        if [[ "$featureset" != alloc* ]]; then
            test_result="$(cargo test --lib -p libtechmino-vlq --no-default-features $featureset --no-fail-fast 2>&1)";
            exitcode="$?"
            if [ "$exitcode" != 0 ]; then
                echo -e "===== Test failure with featureset $featureset exited with code $exitcode =====\n$test_result"
                exit 1
            fi
        fi

        export RUSTDOCFLAGS="$RUSTDOCFLAGS -D warnings"
        doc_result="$(cargo doc --lib -p libtechmino-vlq --no-default-features $featureset --no-deps --document-private-items --keep-going 2>&1)";
        exitcode="$?"
        if [ "$exitcode" != 0 ]; then
            echo -e "===== Doc failure with featureset $featureset exited with code $exitcode =====\n$doc_result"
            exit 1
        fi

        msrv_result="$(cargo msrv --path "$vlq_dir" verify --no-default-features $featureset 2>&1)"
        exitcode="$?"
        if [ "$exitcode" != 0 ]; then
            echo -e "===== MSRV failure with featureset $featureset exited with code $exitcode =====\n$msrv_result"
            exit 1
        fi
    ) &

    pids+=("$!")
done << FEATURES_END

--features std
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
