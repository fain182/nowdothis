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
shoot planning plan

rm -f "$work/nowdothis/tasks.txt"
shoot all-done focus
