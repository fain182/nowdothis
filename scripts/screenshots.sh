#!/bin/sh
# Regenerates the screenshots the metainfo and the README point at.
#
# Run it after any interface change: the store listing showing an older version
# of the app looks like a bug to anyone browsing it.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
out="$root/data/screenshots"
size=760x560

# The app reads its data from XDG_DATA_HOME, so a scratch directory keeps the
# sample tasks out of the real list.
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/nowdothis"

# meson installs the resources and generates src/config.rs, both of which the
# binary needs; the feature build then reuses them.
"$root/scripts/build.sh" >/dev/null

# The schema is installed under the prefix, where GSettings does not look.
export GSETTINGS_SCHEMA_DIR="$root/_install/share/glib-2.0/schemas"

cargo build \
    --features screenshot \
    --manifest-path "$root/Cargo.toml" \
    --target-dir "$root/_build/screenshot" >/dev/null

app="$root/_build/screenshot/debug/nowdothis"

shoot() {
    name=$1
    page=$2
    # Sample tasks are written in English: the interface and the store listing
    # are, and a mismatch reads as a mistake.
    XDG_DATA_HOME="$work" \
    LC_ALL=C \
    NOWDOTHIS_SNAPSHOT="$out/$name.png" \
    NOWDOTHIS_SNAPSHOT_SIZE="$size" \
    NOWDOTHIS_SNAPSHOT_PAGE="$page" \
        timeout 30 "$app"
    echo "  $name.png"
}

tasks() {
    printf '%s\n' "$@" > "$work/nowdothis/tasks.txt"
}

echo "Writing to $out"

tasks "reply to Sam's email" "walk the dog" "pick up milk"
shoot doing ""

# Without the final newline the cursor ends on the last task rather than on an
# empty line, so the handle for moving it shows.
printf '%s' "$(cat "$work/nowdothis/tasks.txt")" > "$work/nowdothis/tasks.txt"
shoot planning plan

# Relative to now, so the dialog reads Today and Yesterday.
at() {
    date -d "$1" +%Y-%m-%dT%H:%M:%S%:z
}
cat > "$work/nowdothis/history.txt" <<EOF
## $(at 'yesterday 08:50')
+ book the dentist
+ water the plants
+ finish the report
## $(at 'yesterday 10:15')
x book the dentist
## $(at 'yesterday 14:30')
+ call the bank
## $(at 'yesterday 16:05')
x finish the report
## $(at 'yesterday 19:40')
x water the plants
## $(at 'today 08:55')
+ reply to Sam's email
+ walk the dog
## $(at 'today 09:30')
- call the bank
## $(at 'today 09:45')
+ pick up milk
EOF
(export NOWDOTHIS_SNAPSHOT_ACTION=win.history; shoot history focus)

rm -f "$work/nowdothis/tasks.txt"
shoot all-done focus
