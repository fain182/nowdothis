#!/bin/sh
# Builds the app and installs it into ./_install.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
build_dir="$root/_build"
prefix="$root/_install"

if [ ! -d "$build_dir" ]; then
    meson setup "$build_dir" --prefix="$prefix"
fi

# meson install compiles whatever is out of date first.
meson install -C "$build_dir"
