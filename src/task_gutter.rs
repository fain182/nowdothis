/* task_gutter.rs
 *
 * Copyright 2026 Pietro Campagnano
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gettextrs::gettext;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, graphene};

use crate::models::line_order;

/// The whole strip up to the text, so the handle can sit right beside it and
/// the area that starts a drag is as wide as the space allows.
const WIDTH: i32 = 48;
const HANDLE_SIZE: i32 = 16;
const HANDLE_GAP: i32 = 8;

mod imp {
    use super::*;

    use std::cell::{Cell, OnceCell};

    #[derive(Debug, Default)]
    pub struct TaskGutter {
        pub view: glib::WeakRef<gtk::TextView>,
        pub moving_tag: OnceCell<gtk::TextTag>,
        pub hovered: Cell<Option<i32>>,
        pub dragging: Cell<Option<i32>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TaskGutter {
        const NAME: &'static str = "NowdothisTaskGutter";
        type Type = super::TaskGutter;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for TaskGutter {}

    impl WidgetImpl for TaskGutter {
        fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            match orientation {
                gtk::Orientation::Horizontal => (WIDTH, WIDTH, -1, -1),
                _ => (0, 0, -1, -1),
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            self.obj().draw_handle(snapshot);
        }
    }
}

glib::wrapper! {
    /// The strip left of the task list holding a handle beside one task, by
    /// which it is dragged to another place in the list. Alt+Up and Alt+Down
    /// move the task under the cursor the same way, for whoever cannot drag.
    pub struct TaskGutter(ObjectSubclass<imp::TaskGutter>)
        @extends gtk::Widget;
}

impl TaskGutter {
    pub fn attach(view: &gtk::TextView) {
        let gutter: Self = glib::Object::new();
        let imp = gutter.imp();
        imp.view.set(Some(view));

        gutter.set_tooltip_text(Some(&gettext("Drag to Move")));
        gutter.add_css_class("dim-label");

        let tag = gtk::TextTag::new(None);
        view.buffer().tag_table().add(&tag);
        imp.moving_tag.set(tag).expect("the tag is only set here");

        gutter.connect_pointer(view);
        gutter.connect_keys(view);
        gutter.connect_redraws(view);

        view.set_gutter(gtk::TextWindowType::Left, Some(&gutter));
    }

    fn connect_pointer(&self, view: &gtk::TextView) {
        let motion = gtk::EventControllerMotion::new();
        motion.connect_motion(clone!(
            #[weak(rename_to = gutter)]
            self,
            move |_, _, y| gutter.hover(Some(gutter.line_at(gtk::TextWindowType::Widget, y)))
        ));
        motion.connect_leave(clone!(
            #[weak(rename_to = gutter)]
            self,
            move |_| gutter.hover(None)
        ));
        view.add_controller(motion);

        let drag = gtk::GestureDrag::new();
        drag.connect_drag_begin(clone!(
            #[weak(rename_to = gutter)]
            self,
            move |gesture, _, y| gutter.begin_drag(gesture, y)
        ));
        drag.connect_drag_update(clone!(
            #[weak(rename_to = gutter)]
            self,
            move |gesture, _, dy| {
                if let Some((_, y)) = gesture.start_point() {
                    gutter.drag_to(y + dy);
                }
            }
        ));
        drag.connect_drag_end(clone!(
            #[weak(rename_to = gutter)]
            self,
            move |_, _, _| gutter.end_drag()
        ));
        self.add_controller(drag);
    }

    fn connect_keys(&self, view: &gtk::TextView) {
        let shortcuts = gtk::ShortcutController::new();
        for (trigger, step) in [("<Alt>Up", -1), ("<Alt>Down", 1)] {
            let action = gtk::CallbackAction::new(clone!(
                #[weak(rename_to = gutter)]
                self,
                #[upgrade_or]
                glib::Propagation::Proceed,
                move |_, _| {
                    gutter.step_current(step);
                    glib::Propagation::Stop
                }
            ));
            shortcuts.add_shortcut(gtk::Shortcut::new(
                gtk::ShortcutTrigger::parse_string(trigger),
                Some(action),
            ));
        }
        view.add_controller(shortcuts);
    }

    /// The gutter is not scrolled along with the text, so the handle has to be
    /// drawn again whenever the text or its position changes. The view itself
    /// redraws the gutter once it has laid out changed lines, which is why the
    /// handle is placed while drawing rather than as a child widget: before
    /// that, the moved lines are still measured where they used to be.
    fn connect_redraws(&self, view: &gtk::TextView) {
        let buffer = view.buffer();
        buffer.connect_changed(clone!(
            #[weak(rename_to = gutter)]
            self,
            move |_| gutter.queue_draw()
        ));
        buffer.connect_mark_set(clone!(
            #[weak(rename_to = gutter)]
            self,
            move |buffer, _, mark| {
                if *mark == buffer.get_insert() {
                    gutter.queue_draw();
                }
            }
        ));
        let follow = clone!(
            #[weak(rename_to = gutter)]
            self,
            move |view: &gtk::TextView| {
                if let Some(adjustment) = view.vadjustment() {
                    adjustment.connect_value_changed(clone!(
                        #[weak]
                        gutter,
                        move |_| gutter.queue_draw()
                    ));
                }
            }
        );
        follow(view);
        view.connect_vadjustment_notify(follow);
    }

    fn view(&self) -> gtk::TextView {
        self.imp()
            .view
            .upgrade()
            .expect("the gutter lives inside its view")
    }

    fn draw_handle(&self, snapshot: &gtk::Snapshot) {
        let imp = self.imp();
        let view = self.view();
        let buffer = view.buffer();

        let line = imp
            .dragging
            .get()
            .or(imp.hovered.get())
            .unwrap_or_else(|| buffer.iter_at_mark(&buffer.get_insert()).line());
        let Some(iter) = buffer.iter_at_line(line).filter(|_| !is_blank(&buffer, line)) else {
            return;
        };

        // Beside the first row of a wrapped task, where the task starts.
        let row = view.iter_location(&iter);
        let (_, y) = view.buffer_to_window_coords(gtk::TextWindowType::Left, 0, row.y());
        let y = y + (row.height() - HANDLE_SIZE) / 2;

        let icon = gtk::IconTheme::for_display(&self.display()).lookup_icon(
            "list-drag-handle-symbolic",
            &[],
            HANDLE_SIZE,
            self.scale_factor(),
            self.direction(),
            gtk::IconLookupFlags::empty(),
        );
        snapshot.save();
        snapshot.translate(&graphene::Point::new((WIDTH - HANDLE_SIZE - HANDLE_GAP) as f32, y as f32));
        icon.snapshot_symbolic(snapshot, HANDLE_SIZE.into(), HANDLE_SIZE.into(), &[self.color()]);
        snapshot.restore();
    }

    fn line_at(&self, window: gtk::TextWindowType, y: f64) -> i32 {
        let view = self.view();
        let (_, y) = view.window_to_buffer_coords(window, 0, y as i32);
        view.line_at_y(y).0.line()
    }

    fn hover(&self, line: Option<i32>) {
        if self.imp().hovered.replace(line) == line {
            return;
        }
        if self.imp().dragging.get().is_none() {
            let over_task = line.is_some_and(|line| !is_blank(&self.view().buffer(), line));
            self.set_cursor_from_name(over_task.then_some("grab"));
        }
        self.queue_draw();
    }

    fn begin_drag(&self, gesture: &gtk::GestureDrag, y: f64) {
        let line = self.line_at(gtk::TextWindowType::Left, y);
        let buffer = self.view().buffer();
        if is_blank(&buffer, line) {
            gesture.set_state(gtk::EventSequenceState::Denied);
            return;
        }
        gesture.set_state(gtk::EventSequenceState::Claimed);

        // One undo step for the whole drag, however many places it passes.
        buffer.begin_user_action();
        self.imp().dragging.set(Some(line));
        self.mark(line);
        self.set_cursor_from_name(Some("grabbing"));
    }

    /// Moves the task as soon as the pointer reaches where it would land, so
    /// the list itself shows the result. A task only moves once the pointer
    /// would still be over it afterwards, or tasks of different heights
    /// would swap back and forth.
    fn drag_to(&self, y: f64) {
        let Some(from) = self.imp().dragging.get() else {
            return;
        };
        let view = self.view();
        let buffer = view.buffer();
        let (_, y) = view.window_to_buffer_coords(gtk::TextWindowType::Left, 0, y as i32);
        let (Some(dragged), Some(last)) = (buffer.iter_at_line(from), last_task(&buffer)) else {
            return;
        };
        let (_, height) = view.line_yrange(&dragged);

        let mut to = view.line_at_y(y).0.line().min(last);
        if let Some(target) = buffer.iter_at_line(to) {
            let (top, target_height) = view.line_yrange(&target);
            if to > from && y < top + target_height - height {
                to -= 1;
            } else if to < from && y >= top + height {
                to += 1;
            }
        }

        if to != from {
            move_line(&buffer, from, to);
            self.imp().dragging.set(Some(to));
            self.mark(to);
        }
    }

    fn end_drag(&self) {
        if self.imp().dragging.take().is_none() {
            return;
        }
        let buffer = self.view().buffer();
        let tag = self.imp().moving_tag.get().expect("the tag is set on attach");
        buffer.remove_tag(tag, &buffer.start_iter(), &buffer.end_iter());
        buffer.end_user_action();
        self.set_cursor_from_name(Some("grab"));
        self.queue_draw();
    }

    fn mark(&self, line: i32) {
        let view = self.view();
        let buffer = view.buffer();
        let tag = self.imp().moving_tag.get().expect("the tag is set on attach");
        // The text colour, faint, reads as a highlight in every scheme, high
        // contrast included. Taken each time, since the scheme can change.
        let mut tint = view.color();
        tint.set_alpha(0.08);
        tag.set_paragraph_background_rgba(Some(&tint));
        buffer.remove_tag(tag, &buffer.start_iter(), &buffer.end_iter());
        if let Some((start, end)) = line_bounds(&buffer, line) {
            buffer.apply_tag(tag, &start, &end);
        }
    }

    fn step_current(&self, step: i32) {
        let buffer = self.view().buffer();
        let from = buffer.iter_at_mark(&buffer.get_insert()).line();
        let to = from + step;
        let Some(last) = last_task(&buffer) else {
            return;
        };
        if !is_blank(&buffer, from) && (0..=last).contains(&to) {
            buffer.begin_user_action();
            move_line(&buffer, from, to);
            buffer.end_user_action();
        }
    }
}

fn line_bounds(buffer: &gtk::TextBuffer, line: i32) -> Option<(gtk::TextIter, gtk::TextIter)> {
    let start = buffer.iter_at_line(line)?;
    let mut end = start;
    if !end.ends_line() {
        end.forward_to_line_end();
    }
    Some((start, end))
}

fn is_blank(buffer: &gtk::TextBuffer, line: i32) -> bool {
    line_bounds(buffer, line).is_none_or(|(start, end)| buffer.text(&start, &end, false).trim().is_empty())
}

/// Blank lines after the last task are where the next one gets typed, not
/// places a task can be moved to.
fn last_task(buffer: &gtk::TextBuffer) -> Option<i32> {
    (0..buffer.line_count()).rev().find(|&line| !is_blank(buffer, line))
}

/// Rewrites only the lines between the two places, keeping the cursor on the
/// task that moved.
fn move_line(buffer: &gtk::TextBuffer, from: i32, to: i32) {
    let (low, high) = (from.min(to), from.max(to));
    let (Some((mut start, _)), Some((_, mut end))) =
        (line_bounds(buffer, low), line_bounds(buffer, high))
    else {
        return;
    };
    let cursor = buffer.iter_at_mark(&buffer.get_insert());
    let offset = if cursor.line() == from { cursor.line_offset() } else { 0 };

    let span = buffer.text(&start, &end, false);
    let moved = line_order::move_line(&span, (from - low) as usize, (to - low) as usize);
    buffer.delete(&mut start, &mut end);
    buffer.insert(&mut start, &moved);

    if let Some(iter) = buffer.iter_at_line_offset(to, offset) {
        buffer.place_cursor(&iter);
    }
}
