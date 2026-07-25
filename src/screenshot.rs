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
/// - `NOWDOTHIS_SNAPSHOT_DELAY` — milliseconds to wait after that action, to
///   catch an animation part way through.
/// - `NOWDOTHIS_SNAPSHOT_TEXT_SCALE` — desktop text scaling factor, for
///   checking that the layout survives large text.
pub fn capture(window: &NowdothisWindow) {
    let Ok(path) = std::env::var("NOWDOTHIS_SNAPSHOT") else {
        return;
    };

    if let Ok(scale) = std::env::var("NOWDOTHIS_SNAPSHOT_TEXT_SCALE") {
        let scale: f64 = scale.parse().expect("the scale is a number");
        if let Some(settings) = gtk::Settings::default() {
            settings.set_gtk_xft_dpi((96.0 * 1024.0 * scale) as i32);
        }
    }

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
            // Adding a task takes typing and confirming, neither of which an
            // action can do on its own.
            if action == "toast" {
                WidgetExt::activate_action(&window, "win.add-task", None)
                    .expect("the action exists");
                if let Some(dialog) = window.visible_dialog() {
                    let alert = dialog
                        .downcast::<adw::AlertDialog>()
                        .expect("the add dialog is an alert");
                    alert
                        .extra_child()
                        .and_downcast::<gtk::Entry>()
                        .expect("the dialog holds an entry")
                        .set_text("post the parcel");
                    alert.emit_by_name::<()>("response", &[&"add"]);
                }
            } else {
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
        }

        // Shortened to catch an animation part way through.
        let delay = std::env::var("NOWDOTHIS_SNAPSHOT_DELAY")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(700);

        let window = window.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(delay), move || {
            if std::env::var("NOWDOTHIS_TAB_ORDER").is_ok() {
                for step in 1..=6 {
                    window.child_focus(gtk::DirectionType::TabForward);
                    let described = window.focus().map(|w| {
                        let label = w
                            .downcast_ref::<gtk::Button>()
                            .and_then(|b| b.label())
                            .map(|l| l.to_string())
                            .or_else(|| w.tooltip_text().map(|t| t.to_string()))
                            .unwrap_or_default();
                        format!("{} \"{}\"", w.type_().name(), label)
                    });
                    println!("TAB {step}: {}", described.unwrap_or("nessuno".into()));
                }
            }

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
