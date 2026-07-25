/* window.rs
 *
 * Copyright 2026 Pietro Campagnano
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::glib::clone;
use gtk::{gio, glib};

use crate::models::task_list::TaskList;
use crate::services::storage::Storage;

mod imp {
    use super::*;

    use std::cell::{Cell, OnceCell, RefCell};

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/camp/pietro/NowDoThis/window.ui")]
    pub struct NowdothisWindow {
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub plan_page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub prompt_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub task_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub task_label_a: TemplateChild<gtk::Label>,
        #[template_child]
        pub task_label_b: TemplateChild<gtk::Label>,
        #[template_child]
        pub action_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub task_view: TemplateChild<gtk::TextView>,
        #[template_child]
        pub placeholder: TemplateChild<gtk::Label>,
        #[template_child]
        pub done_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub plan_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub edit_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub add_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub start_button: TemplateChild<gtk::Button>,

        pub tasks: RefCell<TaskList>,
        pub storage: OnceCell<Storage>,
        /// Guards against reacting to buffer changes we made ourselves.
        pub updating: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NowdothisWindow {
        const NAME: &'static str = "NowdothisWindow";
        type Type = super::NowdothisWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for NowdothisWindow {
        fn constructed(&self) {
            self.parent_constructed();

            let window = self.obj().clone();

            let storage = Storage::new();
            let text = match storage.load() {
                Ok(text) => text,
                Err(error) => {
                    glib::g_warning!("nowdothis", "Could not read the task list: {}", error);
                    String::new()
                }
            };
            self.storage
                .set(storage)
                .expect("storage is only set once, during construction");
            *self.tasks.borrow_mut() = TaskList::from_text(&text);

            self.updating.set(true);
            self.task_view.buffer().set_text(&text);
            self.updating.set(false);

            self.task_view.buffer().connect_changed(clone!(
                #[weak]
                window,
                move |buffer| window.on_list_edited(buffer)
            ));

            self.done_button.connect_clicked(clone!(
                #[weak]
                window,
                move |_| window.complete_current_task()
            ));

            self.edit_button.connect_clicked(clone!(
                #[weak]
                window,
                move |_| {
                    let imp = window.imp();
                    imp.navigation_view.push(&imp.plan_page.get());
                }
            ));

            self.add_button.connect_clicked(clone!(
                #[weak]
                window,
                move |_| window.present_add_dialog()
            ));

            self.plan_button.connect_clicked(clone!(
                #[weak]
                window,
                move |_| {
                    let imp = window.imp();
                    imp.navigation_view.push(&imp.plan_page.get());
                }
            ));

            self.start_button.connect_clicked(clone!(
                #[weak]
                window,
                move |_| {
                    window.imp().navigation_view.pop();
                }
            ));

            // Landing on the planning page means the user is here to type.
            self.plan_page.connect_showing(clone!(
                #[weak]
                window,
                move |_| {
                    window.imp().task_view.grab_focus();
                }
            ));

            // Pages declared in the template are registered with the navigation
            // view, not stacked: the focus page is the root, so an empty list
            // has to push the planning page over it.
            if self.tasks.borrow().is_empty() {
                self.navigation_view.set_animate_transitions(false);
                self.navigation_view.push(&self.plan_page.get());
                self.navigation_view.set_animate_transitions(true);
            }

            window.refresh();
        }
    }

    impl WidgetImpl for NowdothisWindow {}
    impl WindowImpl for NowdothisWindow {}
    impl ApplicationWindowImpl for NowdothisWindow {}
    impl AdwApplicationWindowImpl for NowdothisWindow {}
}

glib::wrapper! {
    pub struct NowdothisWindow(ObjectSubclass<imp::NowdothisWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,        @implements gio::ActionGroup, gio::ActionMap;
}

/// The task is shown as a sentence spoken to the user, so it is given a full
/// stop unless it already ends in punctuation of its own.
fn with_full_stop(task: &str) -> String {
    match task.chars().last() {
        Some(last) if last.is_alphanumeric() => format!("{task}."),
        _ => task.to_string(),
    }
}

impl NowdothisWindow {
    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }

    fn refresh(&self) {
        let imp = self.imp();
        let tasks = imp.tasks.borrow();

        match tasks.current() {
            Some(task) => self.show_task(&with_full_stop(task)),
            None => {
                imp.task_stack.set_visible_child_name("done");
                imp.action_stack.set_visible_child_name("done");
            }
        }

        imp.prompt_revealer.set_reveal_child(!tasks.is_empty());

        imp.placeholder
            .set_visible(imp.task_view.buffer().char_count() == 0);
        // Hidden rather than insensitive: a greyed-out suggested-action button
        // reads as broken, and there is nothing to start anyway.
        imp.start_button.set_visible(!tasks.is_empty());
        // One way out at a time: the call to action replaces the header button.
        imp.edit_button.set_visible(!tasks.is_empty());
    }

    /// Writes the task into whichever of the two labels is offscreen and brings
    /// it forward, so the stack has something to slide between. Leaves the
    /// stack alone when the words have not changed, or editing the list would
    /// animate on every keystroke.
    fn show_task(&self, text: &str) {
        let imp = self.imp();
        imp.action_stack.set_visible_child_name("task");

        let showing = imp.task_stack.visible_child_name();
        let (front, back, back_name) = match showing.as_deref() {
            Some("a") => (imp.task_label_a.get(), imp.task_label_b.get(), "b"),
            _ => (imp.task_label_b.get(), imp.task_label_a.get(), "a"),
        };

        if showing.as_deref() != Some("done") && front.text() == text {
            return;
        }

        back.set_text(text);
        imp.task_stack.set_visible_child_name(back_name);
    }

    fn on_list_edited(&self, buffer: &gtk::TextBuffer) {
        if self.imp().updating.get() {
            self.imp()
                .placeholder
                .set_visible(buffer.char_count() == 0);
            return;
        }

        let text = buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), false)
            .to_string();
        *self.imp().tasks.borrow_mut() = TaskList::from_text(&text);
        self.save(&text);
        self.refresh();
    }

    fn complete_current_task(&self) {
        let imp = self.imp();

        imp.tasks.borrow_mut().complete_current();
        let text = imp.tasks.borrow().to_text();
        self.write_list(&text);
        self.refresh();
    }

    fn add_task(&self, task: &str) {
        let imp = self.imp();

        imp.tasks.borrow_mut().append(task);
        let text = imp.tasks.borrow().to_text();
        self.write_list(&text);

        imp.toast_overlay
            .add_toast(adw::Toast::new(&gettext("Added to the end of the list")));
        self.refresh();
    }

    fn present_add_dialog(&self) {
        let entry = gtk::Entry::builder()
            .placeholder_text(gettext("What came up?"))
            .activates_default(true)
            .build();

        let dialog = adw::AlertDialog::new(Some(&gettext("Add Task")), None);
        dialog.set_extra_child(Some(&entry));
        dialog.add_response("cancel", &gettext("Cancel"));
        dialog.add_response("add", &gettext("Add"));
        dialog.set_response_appearance("add", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("add"));
        dialog.set_close_response("cancel");

        let window = self.clone();
        dialog.connect_response(
            None,
            clone!(
                #[weak]
                entry,
                move |_, response| {
                    if response == "add" {
                        window.add_task(&entry.text());
                    }
                }
            ),
        );

        dialog.present(Some(self));
    }

    /// Mirrors the list into the text view and onto disk, so the two never
    /// disagree about what is left to do.
    fn write_list(&self, text: &str) {
        let imp = self.imp();

        imp.updating.set(true);
        imp.task_view.buffer().set_text(text);
        imp.updating.set(false);

        self.save(text);
    }

    fn save(&self, text: &str) {
        let imp = self.imp();
        let storage = imp
            .storage
            .get()
            .expect("storage is set during construction");

        if let Err(error) = storage.save(text) {
            glib::g_warning!("nowdothis", "Could not save the task list: {}", error);
            imp.toast_overlay
                .add_toast(adw::Toast::new(&gettext("Could not save your list")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::with_full_stop;

    #[test]
    fn a_plain_task_gains_a_full_stop() {
        assert_eq!(with_full_stop("walk the dog"), "walk the dog.");
    }

    #[test]
    fn existing_punctuation_is_left_alone() {
        assert_eq!(with_full_stop("call Mum?"), "call Mum?");
        assert_eq!(with_full_stop("ship it!"), "ship it!");
        assert_eq!(with_full_stop("read chapter 3."), "read chapter 3.");
    }
}
