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
enum Section {
    InitialList,
    Morning,
    Afternoon,
    Evening,
}

impl Section {
    fn of(event: &Event) -> Self {
        if event.change == Change::Initial {
            return Self::InitialList;
        }
        match event.at.hour() {
            0..=11 => Self::Morning,
            12..=17 => Self::Afternoon,
            _ => Self::Evening,
        }
    }

    fn title(self) -> String {
        match self {
            Self::InitialList => gettext("Initial List"),
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

    let events = log.without_moves();
    if events.is_empty() {
        let status = adw::StatusPage::builder()
            .icon_name("document-open-recent-symbolic")
            .title(gettext("No History Yet"))
            .description(gettext("Tasks you add, finish or delete will show up here"))
            .build();
        view.set_content(Some(&status));
    } else {
        view.set_content(Some(&days(&events, now)));
    }

    adw::Dialog::builder()
        .title(gettext("History"))
        .content_width(480)
        .content_height(640)
        .child(&view)
        .build()
}

fn days(events: &[&Event], now: &glib::DateTime) -> gtk::ScrolledWindow {
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .margin_top(12)
        .margin_bottom(24)
        .margin_start(12)
        .margin_end(12)
        .build();

    let mut day = None;
    let mut section = None;
    let mut list = gtk::ListBox::new();

    for event in events.iter().rev() {
        let event_day = event.at.ymd();
        if day != Some(event_day) {
            let heading = gtk::Label::builder()
                .label(day_title(&event.at, now))
                .xalign(0.0)
                .wrap(true)
                .accessible_role(gtk::AccessibleRole::Heading)
                .css_classes(["title-4"])
                .build();
            if day.is_some() {
                heading.set_margin_top(12);
            }
            content.append(&heading);

            list = gtk::ListBox::builder()
                .selection_mode(gtk::SelectionMode::None)
                .css_classes(["boxed-list"])
                .build();
            content.append(&list);

            day = Some(event_day);
            section = None;
        }

        let event_section = Section::of(event);
        if section != Some(event_section) {
            section = Some(event_section);
            list.append(&section_row(event_section));
        }

        list.append(&row(event));
    }

    let clamp = adw::Clamp::builder().maximum_size(600).child(&content).build();
    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&clamp)
        .build()
}

fn section_row(section: Section) -> gtk::ListBoxRow {
    let label = gtk::Label::builder()
        .label(section.title())
        .xalign(0.0)
        .wrap(true)
        .accessible_role(gtk::AccessibleRole::Heading)
        .css_classes(["caption-heading", "dim-label"])
        .build();

    gtk::ListBoxRow::builder()
        .activatable(false)
        .focusable(false)
        .css_classes(["history-section"])
        .child(&label)
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

fn row(event: &Event) -> gtk::ListBoxRow {
    // Only what was finished or lost is coloured: those are what a reader
    // comes here to find.
    let (icon, colours, label) = match event.change {
        Change::Initial => ("view-list-bullet-symbolic", None, gettext("On the list")),
        Change::Added => ("list-add-symbolic", None, gettext("Added")),
        Change::Completed => (
            "object-select-symbolic",
            Some(("success", "completed")),
            gettext("Done"),
        ),
        Change::Deleted => (
            "user-trash-symbolic",
            Some(("error", "deleted")),
            gettext("Deleted"),
        ),
    };

    let image = gtk::Image::from_icon_name(icon);
    image.update_property(&[gtk::accessible::Property::Label(&label)]);

    let task = gtk::Label::builder()
        .label(&event.task)
        .xalign(0.0)
        .hexpand(true)
        .wrap(true)
        .wrap_mode(gtk::pango::WrapMode::WordChar)
        // So a deleted task can be copied back into the list.
        .selectable(true)
        .build();

    let content = gtk::Box::builder().spacing(12).build();
    content.append(&image);
    content.append(&task);

    let row = gtk::ListBoxRow::builder()
        .activatable(false)
        .css_classes(["history-row"])
        .child(&content)
        .build();
    if let Some((icon_colour, tint)) = colours {
        image.add_css_class(icon_colour);
        row.add_css_class(tint);
    }
    row
}
