# Notes for Claude

Build and run instructions are in the README. This file covers what the code
does not say out loud.

## What the app is

A to-do list for one day, not a backlog. GNOME Circle is the standard every
decision is measured against — its criteria live at
`https://gitlab.gnome.org/Teams/Releng/AppOrganization/-/blob/main/AppCriteria.md`.

## Decisions that look like omissions but are not

- **No counter.** The point is the task at hand, not how many are left.
- **No way to postpone.** Deliberate; the original app worked this way.
- **No timer**, though the original had one.
- **The full stop after a task is added at display time only** (`with_full_stop`),
  so the stored list stays plain and tasks never accumulate punctuation.
- **The primary menu sits at the start of the header bar**, which departs from
  the HIG. It was chosen for consistency with the button that switches screens.
  If a reviewer objects, moving it back is a two-line change.
- **Ctrl+Enter, not Enter, starts the list**, because Enter separates tasks in
  the text view.

## Seeing the interface

Design work here is done by looking, not by reasoning about markup. The window
can render itself to a PNG under the `screenshot` Cargo feature, which is off
by default and so never reaches a released build. `src/screenshot.rs` holds it.

    ./scripts/screenshots.sh          # regenerates what the metainfo points at

For a one-off look at some other state, build with the feature and set the
variables yourself:

    cargo build --features screenshot --target-dir _build/screenshot
    XDG_DATA_HOME=/tmp/ndt NOWDOTHIS_SNAPSHOT=/tmp/shot.png \
        NOWDOTHIS_SNAPSHOT_PAGE=plan _build/screenshot/debug/nowdothis

`NOWDOTHIS_SNAPSHOT_ACTION` fires an action first, for states that only appear
on demand such as a dialog, and `NOWDOTHIS_TAB_ORDER=1` prints the focus chain,
which is how keyboard reachability gets checked after a layout change.

Pair it with `ADW_DEBUG_COLOR_SCHEME=prefer-dark` and
`ADW_DEBUG_HIGH_CONTRAST=1`. Dark mode, high contrast and text scaling are all
Circle criteria, and all three have caught real problems. Point
`XDG_DATA_HOME` at a scratch directory so real tasks are left alone.

Run `./scripts/screenshots.sh` after any interface change: a store listing
showing an older version of the app reads as a bug.

## Gotchas

- The app is single instance. If one is already running, a second launch
  activates it and exits in about 40ms, which looks exactly like a crash.
- `meson setup --reconfigure` is needed after adding a language to `po/LINGUAS`.
- `./scripts/test.sh` runs everything: the Rust unit tests and the desktop,
  appstream and schema validations. CI runs the same target inside the Flatpak
  sandbox, so if it passes here it passes there.

## Storage

The task list is a plain text file, one task per line, at
`$XDG_DATA_HOME/nowdothis/tasks.txt`. The list and its text form are the same
thing, which is why there is no serialisation layer.
