#!/bin/sh
# Runs every check: the Rust unit tests and the desktop, appstream and schema
# validations. This is the same target CI runs inside the Flatpak sandbox.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)

"$root/scripts/build.sh" >/dev/null

exec meson test -C "$root/_build" --print-errorlogs "$@"
