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
        pub focus_page: TemplateChild<adw::NavigationPage>,
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
        pub focus_surface: TemplateChild<gtk::ScrolledWindow>,
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

            // Landing on the task means the next thing wanted is Done, so put
            // the keyboard on it and let Enter carry the whole run.
            self.focus_page.connect_showing(clone!(
                #[weak]
                window,
                move |_| {
                    let imp = window.imp();
                    if imp.done_button.is_visible() {
                        imp.done_button.grab_focus();
                    }
                }
            ));

            window.setup_actions();

            // A tint behind the text is the first thing someone asking for more
            // contrast wants gone.
            let style = adw::StyleManager::default();
            style.connect_high_contrast_notify(clone!(
                #[weak]
                window,
                move |style| window.set_vignette(!style.is_high_contrast())
            ));
            window.set_vignette(!style.is_high_contrast());

            // Pages declared in the template are registered with the navigation
            // view, not stacked: the focus page is the root, so an empty list
            // has to push the planning page over it.
            if self.tasks.borrow().is_empty() {
                self.navigation_view.set_animate_transitions(false);
                self.navigation_view.push(&self.plan_page.get());
                self.navigation_view.set_animate_transitions(true);
            }

            window.refresh();

            #[cfg(feature = "screenshot")]
            crate::screenshot::capture(&window);
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

/// The task is shown as a sentence spoken to the user, so it is given a capital
/// and a full stop. Both are presentation only: the list on disk keeps the
/// words exactly as they were typed.
fn as_sentence(task: &str) -> String {
    let mut sentence = capitalised(task);

    if sentence.chars().last().is_some_and(char::is_alphanumeric) {
        sentence.push('.');
    }

    sentence
}

/// Leaves a first word that carries a capital of its own alone, so names like
/// "iPhone" are not mangled into "IPhone".
fn capitalised(task: &str) -> String {
    let first_word = task.split_whitespace().next().unwrap_or_default();
    if first_word.chars().skip(1).any(char::is_uppercase) {
        return task.to_string();
    }

    let mut characters = task.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => String::new(),
    }
}

impl NowdothisWindow {
    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }



    fn set_vignette(&self, wanted: bool) {
        let surface = self.imp().focus_surface.get();
        if wanted {
            surface.add_css_class("focus-surface");
        } else {
            surface.remove_css_class("focus-surface");
        }
    }

    fn setup_actions(&self) {
        let advance = gio::ActionEntry::builder("advance")
            .activate(|window: &Self, _, _| window.advance())
            .build();
        let add_task = gio::ActionEntry::builder("add-task")
            .activate(|window: &Self, _, _| window.present_add_dialog())
            .build();

        self.add_action_entries([advance, add_task]);
    }

    /// The same keystroke moves the day forward wherever you are: it starts the
    /// list from the planning page and marks the task done from the task page.
    fn advance(&self) {
        let imp = self.imp();

        if imp.tasks.borrow().is_empty() {
            return;
        }

        let on_plan = imp
            .navigation_view
            .visible_page()
            .and_then(|page| page.tag())
            .is_some_and(|tag| tag == "plan");

        if on_plan {
            imp.navigation_view.pop();
        } else {
            self.complete_current_task();
        }
    }

    fn refresh(&self) {
        let imp = self.imp();
        let tasks = imp.tasks.borrow();

        match tasks.current() {
            Some(task) => self.show_task(&as_sentence(task)),
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

        // The task changes while focus stays on the Done button, so a screen
        // reader would otherwise never hear what came next.
        back.announce(text, gtk::AccessibleAnnouncementPriority::Medium);
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
    use super::as_sentence;

    #[test]
    fn a_task_is_shown_as_a_sentence() {
        assert_eq!(as_sentence("walk the dog"), "Walk the dog.");
    }

    #[test]
    fn punctuation_of_its_own_is_left_alone() {
        assert_eq!(as_sentence("call Mum?"), "Call Mum?");
        assert_eq!(as_sentence("ship it!"), "Ship it!");
        assert_eq!(as_sentence("read chapter 3."), "Read chapter 3.");
    }

    #[test]
    fn a_name_that_starts_lowercase_keeps_its_own_shape() {
        assert_eq!(as_sentence("iPhone backup"), "iPhone backup.");
        assert_eq!(as_sentence("eBay listing"), "eBay listing.");
    }

    #[test]
    fn a_task_that_starts_with_a_number_is_not_forced() {
        assert_eq!(as_sentence("3pm standup"), "3pm standup.");
    }
}
