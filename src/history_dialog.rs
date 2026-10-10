/* history_dialog.rs
 *
 * Copyright 2026 Pietro Campagnano
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use gettextrs::gettext;
use gtk::glib;

use crate::models::history_log::{Change, Event, HistoryLog};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PartOfDay {
    Morning,
    Afternoon,
    Evening,
}

impl PartOfDay {
    fn of(at: &glib::DateTime) -> Self {
        match at.hour() {
            0..=11 => Self::Morning,
            12..=17 => Self::Afternoon,
            _ => Self::Evening,
        }
    }

    fn title(self) -> String {
        match self {
            Self::Morning => gettext("Morning"),
            Self::Afternoon => gettext("Afternoon"),
            Self::Evening => gettext("Evening"),
        }
    }
}

/// Read only: the list is where tasks are changed, this only shows what
/// became of them, newest first.
pub fn build(log: &HistoryLog, now: &glib::DateTime) -> adw::Dialog {
    let header = adw::HeaderBar::new();
    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);

    if log.events().is_empty() {
        let status = adw::StatusPage::builder()
            .icon_name("document-open-recent-symbolic")
            .title(gettext("No History Yet"))
            .description(gettext("Tasks you add, finish or delete will show up here"))
            .build();
        view.set_content(Some(&status));
    } else {
        view.set_content(Some(&days(log, now)));
    }

    adw::Dialog::builder()
        .title(gettext("History"))
        .content_width(480)
        .content_height(640)
        .child(&view)
        .build()
}

fn days(log: &HistoryLog, now: &glib::DateTime) -> gtk::ScrolledWindow {
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(18)
        .margin_top(12)
        .margin_bottom(24)
        .margin_start(12)
        .margin_end(12)
        .build();

    let mut day = None;
    let mut part = None;
    let mut group = adw::PreferencesGroup::new();

    for event in log.events().iter().rev() {
        let event_day = event.at.ymd();
        if day != Some(event_day) {
            day = Some(event_day);
            part = None;
            let heading = gtk::Label::builder()
                .label(day_title(&event.at, now))
                .xalign(0.0)
                .wrap(true)
                .margin_top(12)
                .accessible_role(gtk::AccessibleRole::Heading)
                .css_classes(["title-2"])
                .build();
            content.append(&heading);
        }

        let event_part = PartOfDay::of(&event.at);
        if part != Some(event_part) {
            part = Some(event_part);
            group = adw::PreferencesGroup::builder()
                .title(event_part.title())
                .build();
            content.append(&group);
        }

        group.add(&row(event));
    }

    let clamp = adw::Clamp::builder().maximum_size(600).child(&content).build();
    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&clamp)
        .build()
}

fn day_title(at: &glib::DateTime, now: &glib::DateTime) -> String {
    let yesterday = now.add_days(-1).map(|day| day.ymd()).ok();
    let format = if at.year() == now.year() {
        // Translators: a day in the history, as in "Wednesday 8 October".
        // See https://docs.gtk.org/glib/method.DateTime.format.html
        gettext("%A %-d %B")
    } else {
        // Translators: a day in an earlier year, as in "Wednesday 8 October 2025".
        gettext("%A %-d %B %Y")
    };

    if at.ymd() == now.ymd() {
        gettext("Today")
    } else if Some(at.ymd()) == yesterday {
        gettext("Yesterday")
    } else {
        let day = at.format(&format).map(String::from).unwrap_or_default();
        // Some languages write day names in lower case, which looks wrong as
        // a heading next to "Today".
        let mut letters = day.chars();
        match letters.next() {
            Some(first) => first.to_uppercase().chain(letters).collect(),
            None => day,
        }
    }
}

fn row(event: &Event) -> adw::ActionRow {
    let (icon, colour, kind, label) = match event.change {
        Change::Added => ("list-add-symbolic", "accent", "added", gettext("Added")),
        Change::Completed => ("object-select-symbolic", "success", "completed", gettext("Done")),
        Change::Deleted => ("user-trash-symbolic", "error", "deleted", gettext("Deleted")),
    };

    let image = gtk::Image::builder()
        .icon_name(icon)
        .css_classes([colour])
        .build();
    image.update_property(&[gtk::accessible::Property::Label(&label)]);

    let row = adw::ActionRow::builder()
        // Task text is the user's own, never markup.
        .use_markup(false)
        .title(&event.task)
        .css_classes(["history", kind])
        .build();
    row.add_prefix(&image);
    row
}
