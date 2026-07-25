#!/bin/sh
# Runs the unit tests.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

exec cargo test "$@"
