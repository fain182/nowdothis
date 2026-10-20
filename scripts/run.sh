#!/bin/sh
# Builds the app and runs it against your real task list.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)

"$root/scripts/build.sh"

# The schema is installed under the prefix, where GSettings does not look.
export GSETTINGS_SCHEMA_DIR="$root/_install/share/glib-2.0/schemas"

exec "$root/_install/bin/nowdothis" "$@"
