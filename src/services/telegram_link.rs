/* telegram_link.rs
 *
 * Copyright 2026 Pietro Campagnano
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gettextrs::gettext;
use gtk::prelude::*;
use gtk::{gio, glib};

use crate::models::pinned_list::pinned_text;
use crate::models::task_list::TaskList;
use crate::models::telegram_inbox::Note;
use crate::services::telegram::Bot;

const RETRY_SECONDS: u32 = 30;
/// Typing saves on every keystroke; the pinned message waits for a pause.
const MIRROR_SECONDS: u32 = 3;

/// Ties the list to the user's own Telegram bot while the app is open: what is
/// sent to the bot is added to the list, and the list is kept in a pinned
/// message. The first private chat to write to the bot is the only one heard.
#[derive(Clone)]
pub struct TelegramLink {
    inner: Rc<Inner>,
}

struct Inner {
    settings: gio::Settings,
    on_tasks: Box<dyn Fn(Vec<String>)>,
    /// Present from start until the link stops.
    bot: RefCell<Option<Bot>>,
    polling: RefCell<Option<glib::JoinHandle<()>>>,
    /// The list as it should read in the chat, once the pause is over.
    wanted: RefCell<String>,
    pending_mirror: RefCell<Option<glib::SourceId>>,
    mirroring: Cell<bool>,
    mirror_again: Cell<bool>,
}

impl std::fmt::Debug for TelegramLink {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TelegramLink")
            .field("paired", &self.is_paired())
            .finish_non_exhaustive()
    }
}

impl TelegramLink {
    pub fn new(on_tasks: impl Fn(Vec<String>) + 'static) -> Self {
        Self {
            inner: Rc::new(Inner {
                settings: gio::Settings::new("camp.pietro.NowDoThis"),
                on_tasks: Box::new(on_tasks),
                bot: RefCell::default(),
                polling: RefCell::default(),
                wanted: RefCell::default(),
                pending_mirror: RefCell::default(),
                mirroring: Cell::default(),
                mirror_again: Cell::default(),
            }),
        }
    }

    pub fn bot_name(&self) -> String {
        self.inner.settings.string("telegram-bot-name").into()
    }

    pub fn is_set_up(&self) -> bool {
        !self.token().is_empty()
    }

    pub fn is_paired(&self) -> bool {
        self.chat_id() != 0
    }

    pub fn connect_paired<F: Fn() + 'static>(&self, callback: F) -> glib::SignalHandlerId {
        self.inner
            .settings
            .connect_changed(Some("telegram-chat-id"), move |_, _| callback())
    }

    pub fn disconnect(&self, handler: glib::SignalHandlerId) {
        self.inner.settings.disconnect(handler);
    }

    /// Forgets any previous bot and starts listening for the first chat.
    pub fn set_up(&self, token: &str, bot_name: &str) {
        self.forget();
        let settings = &self.inner.settings;
        settings
            .set_string("telegram-token", token.trim())
            .expect("the key exists");
        settings
            .set_string("telegram-bot-name", bot_name)
            .expect("the key exists");
        self.start();
    }

    pub fn forget(&self) {
        self.stop();
        for key in [
            "telegram-token",
            "telegram-bot-name",
            "telegram-chat-id",
            "telegram-list-message-id",
            "telegram-offset",
        ] {
            self.inner.settings.reset(key);
        }
    }

    pub fn start(&self) {
        if !self.is_set_up() || self.inner.polling.borrow().is_some() {
            return;
        }

        let bot = Bot::new(&self.token());
        self.inner.bot.replace(Some(bot.clone()));
        let link = self.clone();
        let handle = glib::spawn_future_local(async move { link.poll(bot).await });
        self.inner.polling.replace(Some(handle));
    }

    /// Dropping the poll cancels the request in flight.
    fn stop(&self) {
        if let Some(handle) = self.inner.polling.take() {
            handle.abort();
        }
        self.inner.bot.take();
        if let Some(source) = self.inner.pending_mirror.take() {
            source.remove();
        }
    }

    /// Brings the pinned message in line with the list, after a pause.
    pub fn mirror(&self, tasks: &TaskList) {
        self.inner
            .wanted
            .replace(pinned_text(tasks, &gettext("Nothing left to do")));

        if let Some(source) = self.inner.pending_mirror.take() {
            source.remove();
        }
        let link = self.clone();
        let source = glib::timeout_add_seconds_local_once(MIRROR_SECONDS, move || {
            // The source is gone once it has fired.
            link.inner.pending_mirror.take();
            link.mirror_now();
        });
        self.inner.pending_mirror.replace(Some(source));
    }

    async fn poll(&self, bot: Bot) {
        loop {
            let offset = self.inner.settings.int64("telegram-offset");
            match bot.updates(offset).await {
                Ok(inbox) => {
                    for note in &inbox.notes {
                        self.receive(&bot, note).await;
                    }
                    if let Some(offset) = inbox.next_offset() {
                        self.inner
                            .settings
                            .set_int64("telegram-offset", offset)
                            .expect("the key exists");
                    }
                }
                Err(error) if error.is_bad_token() => {
                    glib::g_warning!("nowdothis", "Telegram refused the bot token: {}", error);
                    self.stop();
                    return;
                }
                Err(error) => {
                    glib::g_debug!("nowdothis", "Could not reach Telegram: {}", error);
                    glib::timeout_future_seconds(RETRY_SECONDS).await;
                }
            }
        }
    }

    async fn receive(&self, bot: &Bot, note: &Note) {
        if !self.is_paired() {
            self.inner
                .settings
                .set_int64("telegram-chat-id", note.chat_id)
                .expect("the key exists");
            let welcome = gettext("Connected. Anything you send here is added to the end of your list.");
            if let Err(error) = bot.send(note.chat_id, &welcome).await {
                glib::g_warning!("nowdothis", "Could not greet the Telegram chat: {}", error);
            }
            self.mirror_now();
            return;
        }

        if note.chat_id != self.chat_id() {
            return;
        }

        let tasks = note.tasks();
        if tasks.is_empty() {
            return;
        }
        (self.inner.on_tasks)(tasks);
        // The phone's only sign that the message arrived.
        if let Err(error) = bot.react(note.chat_id, note.message_id, "👍").await {
            glib::g_debug!("nowdothis", "Could not acknowledge the message: {}", error);
        }
    }

    /// One update at a time, so two quick changes cannot both find no message
    /// and send two lists.
    fn mirror_now(&self) {
        let Some(bot) = self.inner.bot.borrow().clone() else {
            return;
        };
        if !self.is_paired() || self.inner.wanted.borrow().is_empty() {
            return;
        }
        if self.inner.mirroring.get() {
            self.inner.mirror_again.set(true);
            return;
        }

        self.inner.mirroring.set(true);
        let link = self.clone();
        glib::spawn_future_local(async move {
            loop {
                link.inner.mirror_again.set(false);
                link.update_pinned(&bot).await;
                if !link.inner.mirror_again.get() {
                    break;
                }
            }
            link.inner.mirroring.set(false);
        });
    }

    async fn update_pinned(&self, bot: &Bot) {
        let chat_id = self.chat_id();
        let text = self.inner.wanted.borrow().clone();
        let message_id = self.inner.settings.int64("telegram-list-message-id");

        if message_id != 0 {
            match bot.edit(chat_id, message_id, &text).await {
                Ok(()) => return,
                Err(error) if error.is_not_modified() => return,
                Err(error) if error.is_message_gone() => {}
                Err(error) => {
                    glib::g_debug!("nowdothis", "Could not update the pinned list: {}", error);
                    return;
                }
            }
        }

        let message_id = match bot.send(chat_id, &text).await {
            Ok(message_id) => message_id,
            Err(error) => {
                glib::g_debug!("nowdothis", "Could not send the list: {}", error);
                return;
            }
        };
        self.inner
            .settings
            .set_int64("telegram-list-message-id", message_id)
            .expect("the key exists");
        if let Err(error) = bot.pin(chat_id, message_id).await {
            glib::g_warning!("nowdothis", "Could not pin the list: {}", error);
        }
    }

    fn token(&self) -> String {
        self.inner.settings.string("telegram-token").into()
    }

    fn chat_id(&self) -> i64 {
        self.inner.settings.int64("telegram-chat-id")
    }
}
