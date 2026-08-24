#!/bin/sh
# Runs `cargo build` and copies the resulting binary to the location Meson expects.

set -e

cargo_bin="$1"
built_binary="$2"
output="$3"
shift 3

"$cargo_bin" build "$@"
cp "$built_binary" "$output"
