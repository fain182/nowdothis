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

Design work here is done by looking, not by reasoning about markup. Temporarily
add this at the end of `constructed()` in `src/window.rs`, run, look at the PNG,
then remove it before committing:

```rust
if let Ok(path) = std::env::var("NOWDOTHIS_SNAPSHOT") {
    let target = window.clone();
    glib::timeout_add_local_once(std::time::Duration::from_millis(900), move || {
        let paintable = gtk::WidgetPaintable::new(Some(&target));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(&snapshot, target.width() as f64, target.height() as f64);
        if let (Some(node), Some(renderer)) =
            (snapshot.to_node(), target.native().and_then(|n| n.renderer()))
        {
            renderer.render_texture(&node, None).save_to_png(&path).unwrap();
        }
        target.close();
    });
}
```

Pair it with `ADW_DEBUG_COLOR_SCHEME=prefer-dark`, `ADW_DEBUG_HIGH_CONTRAST=1`,
and `gtk::Settings::set_gtk_xft_dpi` for large text — dark mode, high contrast
and text scaling are all Circle criteria, and all three have caught real
problems. Point `XDG_DATA_HOME` at a scratch directory so real tasks are left
alone.

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
