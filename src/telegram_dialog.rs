/* telegram_dialog.rs
 *
 * Copyright 2026 Pietro Campagnano
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use gettextrs::gettext;
use gtk::glib::clone;
use gtk::{gio, glib};

use crate::services::telegram::Bot;
use crate::services::telegram_link::TelegramLink;

/// Walks someone through making a bot of their own and connecting it, or,
/// once that is done, says which bot it is and offers to let go of it.
pub fn build(link: &TelegramLink) -> adw::Dialog {
    let view = adw::NavigationView::new();
    let dialog = adw::Dialog::builder()
        .title(gettext("Telegram"))
        .content_width(460)
        .content_height(560)
        .child(&view)
        .build();

    let root = if link.is_paired() {
        connected_page(&dialog, link, false)
    } else if link.is_set_up() {
        hello_page(&dialog, &view, link)
    } else {
        intro_page(&dialog, &view, link)
    };
    view.add(&root);

    dialog
}

fn intro_page(dialog: &adw::Dialog, view: &adw::NavigationView, link: &TelegramLink) -> adw::NavigationPage {
    let start = pill(&gettext("Get Started"), "suggested-action");
    start.connect_clicked(clone!(
        #[weak]
        dialog,
        #[weak]
        view,
        #[strong]
        link,
        move |_| view.push(&token_page(&dialog, &view, &link))
    ));

    let status = adw::StatusPage::builder()
        .icon_name("phone-symbolic")
        .title(gettext("Add Tasks From Your Phone"))
        .description(gettext(
            "Make a Telegram bot of your own and anything you send it is added to the end of your list. The list stays pinned in the chat, so you can see it anywhere.\n\nMessages pass through Telegram, and are picked up while Now Do This is open.",
        ))
        .child(&start)
        .build();

    page(&gettext("Telegram"), &status)
}

fn token_page(dialog: &adw::Dialog, view: &adw::NavigationView, link: &TelegramLink) -> adw::NavigationPage {
    let steps = adw::PreferencesGroup::new();

    let botfather = step_row(
        "1",
        &gettext("Open @BotFather in Telegram"),
        &gettext("Telegram's own bot for making bots"),
    );
    botfather.set_activatable(true);
    botfather.add_suffix(&gtk::Image::from_icon_name("adw-external-link-symbolic"));
    botfather.connect_activated(|row| open(row, "https://t.me/BotFather"));
    steps.add(&botfather);
    steps.add(&step_row(
        "2",
        &gettext("Send it /newbot"),
        &gettext("Then choose a name and a username for your bot"),
    ));
    steps.add(&step_row(
        "3",
        &gettext("Paste the token it gives you"),
        &gettext("A long code such as 123456789:AAH…"),
    ));

    let token = adw::PasswordEntryRow::builder()
        .title(gettext("Token"))
        .build();
    let entry_group = adw::PreferencesGroup::new();
    entry_group.add(&token);

    let error = gtk::Label::builder()
        .label(gettext("Telegram did not accept this token"))
        .wrap(true)
        .xalign(0.0)
        .visible(false)
        .css_classes(["error"])
        .build();

    let spinner = adw::Spinner::builder().visible(false).build();
    let label = gtk::Label::new(Some(&gettext("Continue")));
    let content = gtk::Box::builder().spacing(12).halign(gtk::Align::Center).build();
    content.append(&spinner);
    content.append(&label);
    let next = gtk::Button::builder()
        .child(&content)
        .halign(gtk::Align::Center)
        .sensitive(false)
        .css_classes(["pill", "suggested-action"])
        .build();

    token.connect_changed(clone!(
        #[weak]
        next,
        #[weak]
        error,
        move |token| {
            next.set_sensitive(!token.text().trim().is_empty());
            error.set_visible(false);
        }
    ));

    let check = clone!(
        #[weak]
        dialog,
        #[weak]
        view,
        #[weak]
        token,
        #[weak]
        next,
        #[weak]
        spinner,
        #[weak]
        error,
        #[strong]
        link,
        move || {
            if !next.is_sensitive() {
                return;
            }
            next.set_sensitive(false);
            spinner.set_visible(true);
            error.set_visible(false);

            let text = token.text().trim().to_string();
            glib::spawn_future_local(clone!(
                #[strong]
                link,
                async move {
                    let answer = Bot::new(&text).username().await;
                    spinner.set_visible(false);
                    next.set_sensitive(true);
                    match answer {
                        Ok(name) => {
                            link.set_up(&text, &name);
                            view.push(&hello_page(&dialog, &view, &link));
                        }
                        Err(problem) => {
                            glib::g_debug!("nowdothis", "The token was not accepted: {}", problem);
                            error.set_label(&if problem.is_bad_token() {
                                gettext("Telegram did not accept this token")
                            } else {
                                gettext("Could not reach Telegram. Check your connection.")
                            });
                            error.set_visible(true);
                        }
                    }
                }
            ));
        }
    );
    next.connect_clicked(clone!(
        #[strong]
        check,
        move |_| check()
    ));
    token.connect_entry_activated(move |_| check());

    let content = column();
    content.append(&steps);
    content.append(&entry_group);
    content.append(&error);
    content.append(&next);

    page(&gettext("Create a Bot"), &scrolled(&content))
}

fn hello_page(dialog: &adw::Dialog, view: &adw::NavigationView, link: &TelegramLink) -> adw::NavigationPage {
    let name = link.bot_name();

    let open_chat = pill(&gettext("Open Chat"), "suggested-action");
    let address = format!("https://t.me/{name}");
    open_chat.connect_clicked(move |button| open(button, &address));

    // A wrong token or bot can only be fixed by starting over.
    let other_bot = pill(&gettext("Use Another Bot"), "flat");
    other_bot.connect_clicked(clone!(
        #[weak]
        dialog,
        #[weak]
        view,
        #[strong]
        link,
        move |_| {
            link.forget();
            view.replace(&[
                intro_page(&dialog, &view, &link),
                token_page(&dialog, &view, &link),
            ]);
        }
    ));

    let buttons = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .halign(gtk::Align::Center)
        .build();
    buttons.append(&open_chat);
    buttons.append(&other_bot);

    let status = adw::StatusPage::builder()
        .paintable(&adw::SpinnerPaintable::new(None::<&gtk::Widget>))
        .title(gettext("Say Hello"))
        // Translators: {} is the bot's username, such as @my_tasks_bot.
        .description(gettext("Send any message to @{}. The first chat to write to it is the only one it will listen to.").replace("{}", &name))
        .child(&buttons)
        .build();
    // The spinner only animates once it knows the widget it is drawn in.
    if let Some(paintable) = status.paintable().and_downcast::<adw::SpinnerPaintable>() {
        paintable.set_widget(Some(&status));
    }

    let handler = link.connect_paired(clone!(
        #[weak]
        dialog,
        #[weak]
        view,
        #[strong]
        link,
        move || {
            if link.is_paired() {
                view.replace(&[connected_page(&dialog, &link, true)]);
            }
        }
    ));
    // The settings outlive the dialog, so the handler must not.
    let handler = std::cell::Cell::new(Some(handler));
    dialog.connect_closed(clone!(
        #[strong]
        link,
        move |_| {
            if let Some(handler) = handler.take() {
                link.disconnect(handler);
            }
        }
    ));

    page(&gettext("Telegram"), &status)
}

/// `just_now` is the end of the walk-through, where the next thing wanted is
/// to close the dialog; later visits are more likely about letting go.
fn connected_page(dialog: &adw::Dialog, link: &TelegramLink, just_now: bool) -> adw::NavigationPage {
    let name = link.bot_name();

    let buttons = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .halign(gtk::Align::Center)
        .build();

    if just_now {
        let done = pill(&gettext("Done"), "suggested-action");
        done.connect_clicked(clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.close();
            }
        ));
        buttons.append(&done);
    } else {
        let open_chat = pill(&gettext("Open Chat"), "suggested-action");
        let address = format!("https://t.me/{name}");
        open_chat.connect_clicked(move |button| open(button, &address));
        buttons.append(&open_chat);

        let forget = pill(&gettext("Disconnect"), "destructive-action");
        forget.connect_clicked(clone!(
            #[weak]
            dialog,
            #[strong]
            link,
            move |_| confirm_forget(&dialog, &link)
        ));
        buttons.append(&forget);
    }

    let status = adw::StatusPage::builder()
        .icon_name("object-select-symbolic")
        // Translators: {} is the bot's username, such as @my_tasks_bot.
        .title(gettext("Connected to @{}").replace("{}", &name))
        .description(gettext(
            "Anything you send to the bot is added to the end of your list, one task per line. The list is pinned at the top of the chat.",
        ))
        .child(&buttons)
        .build();

    page(&gettext("Telegram"), &status)
}

fn confirm_forget(dialog: &adw::Dialog, link: &TelegramLink) {
    let alert = adw::AlertDialog::new(
        Some(&gettext("Disconnect From Telegram?")),
        Some(&gettext(
            "Messages sent to the bot will no longer reach your list. Connecting again takes the bot's token, which @BotFather can show you.",
        )),
    );
    alert.add_response("cancel", &gettext("Cancel"));
    alert.add_response("forget", &gettext("Disconnect"));
    alert.set_response_appearance("forget", adw::ResponseAppearance::Destructive);
    alert.set_close_response("cancel");

    alert.connect_response(
        Some("forget"),
        clone!(
            #[weak]
            dialog,
            #[strong]
            link,
            move |_, _| {
                link.forget();
                dialog.close();
            }
        ),
    );
    alert.present(Some(dialog));
}

fn page(title: &str, content: &impl IsA<gtk::Widget>) -> adw::NavigationPage {
    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    view.set_content(Some(content));
    adw::NavigationPage::new(&view, title)
}

fn pill(label: &str, style: &str) -> gtk::Button {
    gtk::Button::builder()
        .label(label)
        .halign(gtk::Align::Center)
        .css_classes(["pill", style])
        .build()
}

fn step_row(number: &str, title: &str, subtitle: &str) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(subtitle)
        .build();
    row.add_prefix(
        &gtk::Label::builder()
            .label(number)
            .width_chars(2)
            .css_classes(["title-3", "accent"])
            .build(),
    );
    row
}

fn column() -> gtk::Box {
    gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(18)
        .margin_top(12)
        .margin_bottom(24)
        .margin_start(12)
        .margin_end(12)
        .build()
}

fn scrolled(content: &gtk::Box) -> gtk::ScrolledWindow {
    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&adw::Clamp::builder().child(content).build())
        .build()
}

fn open(widget: &impl IsA<gtk::Widget>, uri: &str) {
    let window = widget.root().and_downcast::<gtk::Window>();
    gtk::UriLauncher::new(uri).launch(window.as_ref(), None::<&gio::Cancellable>, |result| {
        if let Err(error) = result {
            glib::g_warning!("nowdothis", "Could not open the link: {}", error);
        }
    });
}
