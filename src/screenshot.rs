/* screenshot.rs
 *
 * Copyright 2026 Pietro Campagnano
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Renders the window to a PNG so that interface work can be judged by looking
//! at it rather than by reading markup.
//!
//! Compiled only under the `screenshot` feature, which `scripts/screenshots.sh`
//! turns on, so none of this reaches a released build.

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;

use crate::window::NowdothisWindow;

/// Reads its settings from the environment:
///
/// - `NOWDOTHIS_SNAPSHOT` — where to write the PNG. Nothing happens without it.
/// - `NOWDOTHIS_SNAPSHOT_SIZE` — window size as `WIDTHxHEIGHT`.
/// - `NOWDOTHIS_SNAPSHOT_PAGE` — `plan` or `focus`, to override which page an
///   empty or full list would otherwise open on.
/// - `NOWDOTHIS_SNAPSHOT_ACTION` — an action to fire first, so states that only
///   appear on demand, such as a dialog, can be captured.
pub fn capture(window: &NowdothisWindow) {
    let Ok(path) = std::env::var("NOWDOTHIS_SNAPSHOT") else {
        return;
    };

    if let Ok(size) = std::env::var("NOWDOTHIS_SNAPSHOT_SIZE") {
        let (width, height) = size.split_once('x').expect("size reads WIDTHxHEIGHT");
        window.set_default_size(
            width.parse().expect("width is a number"),
            height.parse().expect("height is a number"),
        );
    }

    let imp = window.imp();
    let page = std::env::var("NOWDOTHIS_SNAPSHOT_PAGE").unwrap_or_default();

    imp.navigation_view.set_animate_transitions(false);
    match page.as_str() {
        "plan" => imp.navigation_view.push(&imp.plan_page.get()),
        "focus" => {
            imp.navigation_view.pop();
        }
        _ => {}
    }
    imp.navigation_view.set_animate_transitions(true);

    let window = window.clone();
    // The application is only attached to the window after construction, so the
    // action has to wait a turn of the loop.
    glib::timeout_add_local_once(std::time::Duration::from_millis(400), move || {
        if let Ok(action) = std::env::var("NOWDOTHIS_SNAPSHOT_ACTION") {
            // App actions are not reachable through a widget's action muxer.
            match action.strip_prefix("app.") {
                Some(name) => window
                    .application()
                    .expect("the window has an application")
                    .activate_action(name, None),
                None => {
                    WidgetExt::activate_action(&window, &action, None)
                        .expect("the action exists");
                }
            }
        }

        let window = window.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(700), move || {
            let paintable = gtk::WidgetPaintable::new(Some(&window));
            let snapshot = gtk::Snapshot::new();
            paintable.snapshot(&snapshot, window.width() as f64, window.height() as f64);

            match (
                snapshot.to_node(),
                window.native().and_then(|native| native.renderer()),
            ) {
                (Some(node), Some(renderer)) => renderer
                    .render_texture(&node, None)
                    .save_to_png(&path)
                    .expect("the snapshot is written"),
                _ => eprintln!("nothing to render: is the window mapped?"),
            }

            window.close();
        });
    });
}
