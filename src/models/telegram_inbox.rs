/* telegram_inbox.rs
 *
 * Copyright 2026 Pietro Campagnano
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use serde_json::Value;

/// A text message someone sent to the bot from a private chat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub chat_id: i64,
    pub text: String,
}

impl Note {
    /// One task per line. Lines starting with a slash are bot commands, such
    /// as the /start Telegram sends when the chat is first opened.
    pub fn tasks(&self) -> Vec<String> {
        self.text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('/'))
            .map(String::from)
            .collect()
    }
}

/// One batch of updates as returned by getUpdates.
#[derive(Debug, Default)]
pub struct Inbox {
    pub notes: Vec<Note>,
    last_update_id: Option<i64>,
}

impl Inbox {
    /// Takes the `result` array of a getUpdates response. Updates that are not
    /// text messages in a private chat are skipped, but still counted towards
    /// the offset, or Telegram would hand them back forever.
    pub fn from_updates(updates: &Value) -> Self {
        let mut inbox = Self::default();

        for update in updates.as_array().into_iter().flatten() {
            if let Some(id) = update["update_id"].as_i64() {
                inbox.last_update_id = inbox.last_update_id.max(Some(id));
            }

            let message = &update["message"];
            let chat = &message["chat"];
            if chat["type"].as_str() != Some("private") {
                continue;
            }
            if let (Some(chat_id), Some(text)) = (chat["id"].as_i64(), message["text"].as_str()) {
                inbox.notes.push(Note {
                    chat_id,
                    text: text.to_string(),
                });
            }
        }

        inbox
    }

    /// The offset that confirms this batch, or `None` when it was empty.
    pub fn next_offset(&self) -> Option<i64> {
        self.last_update_id.map(|id| id + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn text_update(update_id: i64, chat_id: i64, text: &str) -> Value {
        json!({
            "update_id": update_id,
            "message": {
                "message_id": 1,
                "chat": { "id": chat_id, "type": "private" },
                "date": 0,
                "text": text
            }
        })
    }

    #[test]
    fn each_line_of_a_message_is_a_task() {
        let note = Note {
            chat_id: 1,
            text: "buy bread\n\n  call Sam  \n".to_string(),
        };
        assert_eq!(note.tasks(), vec!["buy bread", "call Sam"]);
    }

    #[test]
    fn commands_are_not_tasks() {
        let note = Note {
            chat_id: 1,
            text: "/start".to_string(),
        };
        assert!(note.tasks().is_empty());
    }

    #[test]
    fn text_messages_are_read_in_order() {
        let inbox = Inbox::from_updates(&json!([
            text_update(7, 42, "first"),
            text_update(8, 42, "second"),
        ]));
        let texts: Vec<_> = inbox.notes.iter().map(|note| note.text.as_str()).collect();
        assert_eq!(texts, vec!["first", "second"]);
        assert_eq!(inbox.notes[0].chat_id, 42);
        assert_eq!(inbox.next_offset(), Some(9));
    }

    #[test]
    fn updates_without_text_still_move_the_offset() {
        let inbox = Inbox::from_updates(&json!([
            text_update(3, 42, "walk the dog"),
            { "update_id": 4, "message": {
                "message_id": 2,
                "chat": { "id": 42, "type": "private" },
                "date": 0,
                "photo": []
            } },
        ]));
        assert_eq!(inbox.notes.len(), 1);
        assert_eq!(inbox.next_offset(), Some(5));
    }

    #[test]
    fn messages_from_groups_are_skipped() {
        let inbox = Inbox::from_updates(&json!([{
            "update_id": 1,
            "message": {
                "message_id": 1,
                "chat": { "id": -100, "type": "group" },
                "date": 0,
                "text": "spam"
            }
        }]));
        assert!(inbox.notes.is_empty());
        assert_eq!(inbox.next_offset(), Some(2));
    }

    #[test]
    fn an_empty_batch_confirms_nothing() {
        assert_eq!(Inbox::from_updates(&json!([])).next_offset(), None);
    }
}
