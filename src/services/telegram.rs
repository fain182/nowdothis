/* telegram.rs
 *
 * Copyright 2026 Pietro Campagnano
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::fmt;

use gtk::glib;
use serde_json::{json, Value};
use soup::prelude::*;

use crate::models::telegram_inbox::Inbox;

/// How long Telegram holds a getUpdates call open when nothing arrives.
const POLL_SECONDS: u32 = 50;

#[derive(Debug)]
pub enum BotError {
    /// The request never got an answer: no network, or Telegram unreachable.
    Network(glib::Error),
    /// Telegram answered, and said no.
    Refused { code: i64, description: String },
}

impl BotError {
    pub fn is_bad_token(&self) -> bool {
        matches!(self, Self::Refused { code: 401 | 404, .. })
    }

    /// Editing a message to the text it already has is refused, but leaves it
    /// exactly as wanted.
    pub fn is_not_modified(&self) -> bool {
        self.description_contains("message is not modified")
    }

    /// The user deleted the message, so there is nothing left to edit.
    pub fn is_message_gone(&self) -> bool {
        self.description_contains("message to edit not found")
    }

    fn description_contains(&self, words: &str) -> bool {
        matches!(self, Self::Refused { description, .. } if description.contains(words))
    }
}

impl fmt::Display for BotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(error) => write!(formatter, "{error}"),
            Self::Refused { code, description } => write!(formatter, "{code}: {description}"),
        }
    }
}

/// Telegram's Bot API, reached with the token BotFather gave for one bot.
#[derive(Debug, Clone)]
pub struct Bot {
    token: String,
    session: soup::Session,
}

impl Bot {
    pub fn new(token: &str) -> Self {
        let session = soup::Session::new();
        // Longer than the long poll, so an idle wait is not taken for a
        // dropped connection.
        session.set_timeout(POLL_SECONDS + 10);

        Self {
            token: token.trim().to_string(),
            session,
        }
    }

    /// The bot's username, which is also how the token is checked.
    pub async fn username(&self) -> Result<String, BotError> {
        let me = self.call("getMe", json!({})).await?;
        Ok(me["username"].as_str().unwrap_or_default().to_string())
    }

    /// Waits for messages to arrive. Passing an offset confirms every update
    /// before it, so Telegram stops sending them.
    pub async fn updates(&self, offset: i64) -> Result<Inbox, BotError> {
        let updates = self
            .call(
                "getUpdates",
                json!({
                    "offset": offset,
                    "timeout": POLL_SECONDS,
                    "allowed_updates": ["message"],
                }),
            )
            .await?;
        Ok(Inbox::from_updates(&updates))
    }

    /// Returns the id of the message sent.
    pub async fn send(&self, chat_id: i64, text: &str) -> Result<i64, BotError> {
        let message = self
            .call("sendMessage", json!({ "chat_id": chat_id, "text": text }))
            .await?;
        Ok(message["message_id"].as_i64().unwrap_or_default())
    }

    pub async fn edit(&self, chat_id: i64, message_id: i64, text: &str) -> Result<(), BotError> {
        self.call(
            "editMessageText",
            json!({ "chat_id": chat_id, "message_id": message_id, "text": text }),
        )
        .await
        .map(drop)
    }

    pub async fn pin(&self, chat_id: i64, message_id: i64) -> Result<(), BotError> {
        self.call(
            "pinChatMessage",
            json!({
                "chat_id": chat_id,
                "message_id": message_id,
                "disable_notification": true,
            }),
        )
        .await
        .map(drop)
    }

    pub async fn react(&self, chat_id: i64, message_id: i64, emoji: &str) -> Result<(), BotError> {
        self.call(
            "setMessageReaction",
            json!({
                "chat_id": chat_id,
                "message_id": message_id,
                "reaction": [{ "type": "emoji", "emoji": emoji }],
            }),
        )
        .await
        .map(drop)
    }

    async fn call(&self, method: &str, parameters: Value) -> Result<Value, BotError> {
        let uri = format!("https://api.telegram.org/bot{}/{method}", self.token);
        // A token with characters a URI cannot hold is one Telegram would not
        // have issued.
        let message = soup::Message::new("POST", &uri).map_err(|_| BotError::Refused {
            code: 401,
            description: "Malformed token".to_string(),
        })?;
        let body = glib::Bytes::from_owned(parameters.to_string().into_bytes());
        message.set_request_body_from_bytes(Some("application/json"), Some(&body));

        let reply = self
            .session
            .send_and_read_future(&message, glib::Priority::DEFAULT)
            .await
            .map_err(BotError::Network)?;

        read_reply(&reply)
    }
}

/// Every answer, refusals included, is a JSON object saying whether it worked.
fn read_reply(reply: &[u8]) -> Result<Value, BotError> {
    let mut reply: Value = serde_json::from_slice(reply).map_err(|error| BotError::Refused {
        code: 0,
        description: error.to_string(),
    })?;

    if reply["ok"].as_bool() == Some(true) {
        return Ok(reply["result"].take());
    }

    Err(BotError::Refused {
        code: reply["error_code"].as_i64().unwrap_or_default(),
        description: reply["description"].as_str().unwrap_or_default().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_successful_reply_yields_its_result() {
        let result = read_reply(br#"{"ok":true,"result":{"username":"my_bot"}}"#).unwrap();
        assert_eq!(result["username"], "my_bot");
    }

    #[test]
    fn a_wrong_token_is_recognised() {
        let error = read_reply(br#"{"ok":false,"error_code":401,"description":"Unauthorized"}"#)
            .unwrap_err();
        assert!(error.is_bad_token());
    }

    #[test]
    fn an_unchanged_edit_is_recognised() {
        let error = read_reply(
            br#"{"ok":false,"error_code":400,"description":"Bad Request: message is not modified: specified new message content and reply markup are exactly the same"}"#,
        )
        .unwrap_err();
        assert!(error.is_not_modified());
        assert!(!error.is_bad_token());
    }

    #[test]
    fn a_deleted_message_is_recognised() {
        let error = read_reply(
            br#"{"ok":false,"error_code":400,"description":"Bad Request: message to edit not found"}"#,
        )
        .unwrap_err();
        assert!(error.is_message_gone());
    }
}
