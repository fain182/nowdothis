<img src="data/icons/camp.pietro.NowDoThis.png" width="128" align="left" alt="">

# Now Do This

A to-do app for today.

<br clear="left">

You write the day's tasks as plain lines of text, then the app shows you one
at a time. There is no counter, no timer, and no way to postpone: the only
thing on screen is what you are meant to be doing right now. Tasks that come
up while you are working can be added without leaving that screen — they go to
the end of the list, so they never change what you are doing.

Whatever is left undone is still there tomorrow.

![The app showing a single task, with a Done button beneath it](data/screenshots/doing.png)

Inspired by [nowdothis.com](http://nowdothis.com/) by William Cotton and
Jakob Lodwick.

## Building

### GNOME Builder

Open the project and press <kbd>F5</kbd> to build and run it inside Flatpak.

### Command line

```sh
./scripts/build.sh    # build into ./_install
./scripts/run.sh      # build, then run
./scripts/test.sh     # unit tests
```

The scripts wrap Meson and Cargo; `scripts/build.sh` configures the build
directory on first use.

## Data

The task list is a plain text file, one task per line, at
`~/.local/share/nowdothis/tasks.txt`. You can edit or back it up like any
other text file.

## License

GPL-3.0-or-later. See [COPYING](COPYING).
