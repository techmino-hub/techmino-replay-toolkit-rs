#!/usr/bin/env bash

if ! cargo --version >/dev/null; then
    echo "cargo: command not found"
    exit 1
fi

# e.g. ~/Code/techmino-replay-toolkit-rs
scripts_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

tmpdir="${TMPDIR:-/tmp/}"

# Start many scripts asynchronously
echo "running tests..."

pids=()

(
    fmt_result="$(cargo fmt --all -- --check 2>&1)"
    if [ $? != 0 ]; then
        echo -e "===== Fmt failed =====\n$fmt_result"
        exit 1
    fi
) &

pids[0]=$!

while read featureset; do
    (
        clippy_result="$(cargo clippy --lib -p libtechmino-replay --no-default-features --features "$featureset" --keep-going -- -D warnings 2>&1)";
        exitcode="$?"
        if [ "$exitcode" != 0 ]; then
            echo -e "===== Clippy failure with featureset $featureset exited with code $exitcode =====\n$clippy_result"
            exit 1
        fi

        # Tests assume std, so skip if we use alloc instead
        if [[ "$featureset" != alloc* ]]; then
            test_result="$(cargo test --lib -p libtechmino-replay --no-default-features --features "$featureset" --no-fail-fast 2>&1)";
            exitcode="$?"
            if [ "$exitcode" != 0 ]; then
                echo -e "===== Test failure with featureset $featureset exited with code $exitcode =====\n$test_result"
                exit 1
            fi
        fi

        export RUSTDOCFLAGS="$RUSTDOCFLAGS -D warnings"
        doc_result="$(cargo doc --lib -p libtechmino-replay --no-default-features --features "$featureset" --no-deps --document-private-items --keep-going 2>&1)";
        exitcode="$?"
        if [ "$exitcode" != 0 ]; then
            echo -e "===== Doc failure with featureset $featureset exited with code $exitcode =====\n$doc_result"
            exit 1
        fi
    ) &

    pids[${#pids[@]}]="$!"
done << FEATURES_END
std
std,arbitrary
std,strum
std,strum,preserve_metadata_order,float_roundtrip
std,arbitrary,strum,preserve_metadata_order,float_roundtrip
alloc
alloc,strum
alloc,strum,preserve_metadata_order,float_roundtrip
FEATURES_END

echo "all tests launched, awaiting completion..."

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
