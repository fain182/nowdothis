#!/bin/sh
# Builds the app and runs it against your real task list.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)

"$root/scripts/build.sh"

exec "$root/_install/bin/nowdothis" "$@"
