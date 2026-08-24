#!/usr/bin/env bash

# Packages the app and its dependencies for Windows. This is adapted from:
# https://github.com/GeopJr/Tuba/blob/main/Makefile
#
# Run this within an MSYS2 UCRT64 shell.

set -euo pipefail

APP_ID="${APP_ID:-de.johrpan.Musicus}"
EXE_NAME="${EXE_NAME:-musicus.exe}"
BUILD_DIR="${BUILD_DIR:-builddir}"
PREFIX_NAME="${PREFIX_NAME:-$(basename "$EXE_NAME" .exe)_windows_portable}"
MSYS_SYS="${MSYS_SYS:-ucrt64}"


EXTRA_DLLS="${EXTRA_DLLS:-librsvg-2-2.dll}"

ROOT_DIR="$(pwd)"
PREFIX="$ROOT_DIR/$PREFIX_NAME"

if [[ "${MSYSTEM:-}" != "UCRT64" ]]; then
    echo "ERROR: run this from an MSYS2 UCRT64 shell (current MSYSTEM=${MSYSTEM:-unset})" >&2
    exit 1
fi

echo "# Resetting $PREFIX_NAME"
rm -rf "$PREFIX"
mkdir -p "$PREFIX/lib/"

echo "# meson setup/compile"
meson setup "$BUILD_DIR" --prefix="$PREFIX" --wipe
meson compile -C "$BUILD_DIR"

echo "# meson install"
meson install -C "$BUILD_DIR"

echo "# Discovering DLLs"

ldd "$PREFIX/bin/$EXE_NAME" | grep '\/'"$MSYS_SYS"'.*\.dll' -o | xargs -I{} cp "{}" "$PREFIX/bin" || true

# The gspawn helper is allegedly spawned as a separate process by GLib's
# g_spawn_*() on Windows.
cp -f "/$MSYS_SYS/bin/gspawn-win64-helper.exe" "$PREFIX/bin" \
    && ldd "$PREFIX/bin/gspawn-win64-helper.exe" | grep '\/'"$MSYS_SYS"'.*\.dll' -o | xargs -I{} cp "{}" "$PREFIX/bin" || true

for dll in $EXTRA_DLLS; do
    if [[ -f "/$MSYS_SYS/bin/$dll" ]]; then
        cp -f "/$MSYS_SYS/bin/$dll" "$PREFIX/bin"
    fi
done

cp -r "/$MSYS_SYS/lib/gio/" "$PREFIX/lib"
cp -r "/$MSYS_SYS/lib/gdk-pixbuf-2.0" "$PREFIX/lib/gdk-pixbuf-2.0"
cp -r "/$MSYS_SYS/lib/gstreamer-1.0" "$PREFIX/lib/gstreamer-1.0"

# Indirect dependencies
ldd "$PREFIX"/lib/gio/*/*.dll 2>/dev/null | grep '\/'"$MSYS_SYS"'.*\.dll' -o | xargs -I{} cp "{}" "$PREFIX/bin" || true
ldd "$PREFIX"/lib/gstreamer-1.0/*.dll 2>/dev/null | grep '\/'"$MSYS_SYS"'.*\.dll' -o | xargs -I{} cp "{}" "$PREFIX/bin" || true
ldd "$PREFIX"/lib/gdk-pixbuf-2.0/*/loaders/*.dll 2>/dev/null | grep '\/'"$MSYS_SYS"'.*\.dll' -o | xargs -I{} cp "{}" "$PREFIX/bin" || true

# Final pass over bin/ itself
ldd "$PREFIX"/bin/*.dll 2>/dev/null | grep '\/'"$MSYS_SYS"'.*\.dll' -o | xargs -I{} cp "{}" "$PREFIX/bin" || true

# Probably required by GStreamer at runtime.
if [[ -f "/$MSYS_SYS/lib/gstreamer-1.0/gst-plugin-scanner.exe" ]]; then
    ldd "$PREFIX/lib/gstreamer-1.0/gst-plugin-scanner.exe" | grep '\/'"$MSYS_SYS"'.*\.dll' -o | xargs -I{} cp "{}" "$PREFIX/bin" || true
fi

echo "# Compiling GLib schemas"
mkdir -p "$PREFIX/share/glib-2.0/schemas/"
cp -r "/$MSYS_SYS/share/glib-2.0/schemas/"*.xml "$PREFIX/share/glib-2.0/schemas/"
glib-compile-schemas.exe "$PREFIX/share/glib-2.0/schemas/"

echo "# Copying icon themes"
cp -r "/$MSYS_SYS/share/icons/" "$PREFIX/share/"

echo "# Zipping"
( cd "$ROOT_DIR" && zip -r9q "${PREFIX_NAME}.zip" "$PREFIX_NAME/" )

echo
echo "# Done"
echo "Folder: $PREFIX"
echo "Zip: $ROOT_DIR/${PREFIX_NAME}.zip"