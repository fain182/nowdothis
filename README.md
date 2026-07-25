# Now Do This

A to-do app for a single day, not a general backlog.

You write the day's tasks as plain lines of text, then the app shows you one
at a time. There is no counter, no timer, and no way to postpone: the only
thing on screen is what you are meant to be doing right now. Tasks that come
up while you are working can be added without leaving that screen — they go to
the end of the list, so they never change what you are doing.

Whatever is left undone is still there tomorrow.

Inspired by [nowdothis.com](http://nowdothis.com/) by William Cotton and
Jakob Lodwick.

## Building

### GNOME Builder

Open the project and press <kbd>F5</kbd> to build and run it inside Flatpak.

### Command line

```sh
meson setup _build --prefix="$PWD/_install"
meson install -C _build
./_install/bin/nowdothis
```

Rebuild after a change with `meson install -C _build`.

### Tests

```sh
cargo test
```

## Data

The task list is a plain text file, one task per line, at
`~/.local/share/nowdothis/tasks.txt`. You can edit or back it up like any
other text file.

## License

GPL-3.0-or-later. See [COPYING](COPYING).
