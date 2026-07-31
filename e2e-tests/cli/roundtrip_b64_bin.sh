#!/usr/bin/env bash

script_dir=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

. "$script_dir/utils/_setup.sh"

cargo b $cargo_build_verbosity_arg --bin techmino-replay-toolkit

for b64_file in "$testcases_dir/"*".b64.rep"; do
    tmpfile="$tmpdir/$(basename "$b64_file").tmp"
    cargo r --bin techmino-replay-toolkit $cargo_verbosity_arg -- cli binaryify -i "$b64_file" | cargo r --bin techmino-replay-toolkit $cargo_verbosity_arg -- cli base64ify -o "$tmpfile"

    if diff "$b64_file" "$tmpfile"; then
        # Same file
        if [[ "$verbosity" -ge 2 ]]; then
            echo "b64->bin->b64: ok ($b64_file)"
        elif [[ "$verbosity" -eq 1 ]]; then
            echo -n ","
        fi
        continue
    else
        # Different files, something went wrong
        echo "b64->bin->b64: fail ($b64_file != $tmpfile)"
        exit 1
    fi

    rm --preserve-root "$tmpfile"
done

for bin_file in "$testcases_dir/"*".bin.rep"; do
    tmpfile="$tmpdir/$(basename "$bin_file").tmp"
    cargo r --bin techmino-replay-toolkit $cargo_verbosity_arg -- cli base64ify -i "$bin_file" | cargo r --bin techmino-replay-toolkit $cargo_verbosity_arg -- cli binaryify -o "$tmpfile"

    if diff "$bin_file" "$tmpfile"; then
        # Same file
        if [[ "$verbosity" -ge 2 ]]; then
            echo "bin->b64->bin: ok ($bin_file)"
        elif [[ "$verbosity" -eq 1 ]]; then
            echo -n "."
        fi
        continue
    else
        # Different files, something went wrong
        echo "bin->b64->bin: fail ($bin_file != $tmpfile)"
        exit 1
    fi

    rm --preserve-root "$tmpfile"
done

rm -r --preserve-root "$tmpdir"
popd >/dev/null
