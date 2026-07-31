#!/usr/bin/env bash

script_dir=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
creation_marker_key='__meta__created_with_techmino_replay_toolkit'

if ! jq --version >/dev/null; then
    echo 'required executable `jq` not found (is it in $PATH?)'
    exit 1
fi

. "$script_dir/utils/_setup.sh"

cargo b $cargo_build_verbosity_arg --bin techmino-replay-toolkit

for replay_file in "$testcases_dir/"*".rep"; do
    tmpfile="$tmpdir/$(basename "$replay_file").tmp"
    json=$(cargo r --bin techmino-replay-toolkit $cargo_verbosity_arg -- cli extract -i "$replay_file" all |\
        jq --sort-keys "del(.metadata.$creation_marker_key)")

    if [[ "$verbosity" -gt 2 ]]; then
        echo "extracted json: $json"
    fi

    roundtripped=$(echo "$json" |\
        cargo r --bin techmino-replay-toolkit $cargo_verbosity_arg -- cli create -f binary |\
        cargo r --bin techmino-replay-toolkit $cargo_verbosity_arg -- cli extract all |\
        jq --sort-keys "del(.metadata.$creation_marker_key)")

    if [[ "$verbosity" -gt 2 ]]; then
        echo "roundtripped(binary) json: $roundtripped"
    fi

    roundtripped_b64=$(echo "$json" |\
        cargo r --bin techmino-replay-toolkit $cargo_verbosity_arg -- cli create -f base64 |\
        cargo r --bin techmino-replay-toolkit $cargo_verbosity_arg -- cli extract all |\
        jq --sort-keys "del(.metadata.$creation_marker_key)")

    if [[ "$verbosity" -gt 2 ]]; then
        echo "roundtripped(b64) json: $roundtripped"
    fi

    if [[ "$roundtripped_b64" != "$roundtripped" ]]; then
        echo "rep->json->rep: fail ($replay_file): differing json between b64 and binary roundtrip"
        exit 1
    fi

    if [[ "$json" == "$roundtripped" ]]; then
        # Same json
        if [[ "$verbosity" -ge 2 ]]; then
            echo "rep->json->rep: ok ($replay_file)"
        elif [[ "$verbosity" -eq 1 ]]; then
            echo -n ","
        fi
        continue
    else
        # Different files, something went wrong
        echo "rep->json->rep: fail ($replay_file): roundtrip resulted in different json than source"
        exit 1
    fi

    rm --preserve-root "$tmpfile"
done

rm -r --preserve-root "$tmpdir"
popd >/dev/null
