#!/bin/sh

export MESON_BUILD_ROOT="$1"
export MESON_SOURCE_ROOT="$2"
export CARGO_TARGET_DIR="$MESON_BUILD_ROOT"/target
export CARGO_HOME="$CARGO_TARGET_DIR"/cargo-home
export OUTPUT="$3"
export BUILDTYPE="$4"
export APP_BIN="$5"

if [ -z ${CARGO_BUILD_TARGET+defined} ]; then
    CARGO_OUTPUT_PATH="${CARGO_TARGET_DIR}"
else
    CARGO_OUTPUT_PATH="${CARGO_TARGET_DIR}/${CARGO_BUILD_TARGET}"
fi

if [ $BUILDTYPE = "release" ]; then
    echo "RELEASE MODE"
    cargo build --manifest-path \
        "$MESON_SOURCE_ROOT"/Cargo.toml --release && \
        cp "$CARGO_OUTPUT_PATH"/release/"$APP_BIN" "$OUTPUT"
else
    echo "DEBUG MODE"
    cargo build --manifest-path \
        "$MESON_SOURCE_ROOT"/Cargo.toml --verbose && \
        cp "$CARGO_OUTPUT_PATH"/debug/"$APP_BIN" "$OUTPUT"
fi

