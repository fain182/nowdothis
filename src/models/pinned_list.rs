/* pinned_list.rs
 *
 * Copyright 2026 Pietro Campagnano
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use crate::models::sentence::as_sentence;
use crate::models::task_list::TaskList;

/// Telegram refuses longer messages.
const LIMIT: usize = 4096;

/// The list as it reads in the pinned Telegram message, one sentence per line
/// in the order they will be done. `empty` stands in for an empty list,
/// because Telegram refuses an empty message.
pub fn pinned_text(tasks: &TaskList, empty: &str) -> String {
    if tasks.is_empty() {
        return empty.to_string();
    }

    let mut text = String::new();
    for task in tasks.iter() {
        let line = as_sentence(task);
        // Whole lines are dropped rather than cut, and the ellipsis says so.
        if text.chars().count() + line.chars().count() + 2 > LIMIT {
            text.push('…');
            break;
        }
        text.push_str(&line);
        text.push('\n');
    }

    text.trim_end_matches('\n').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tasks_read_as_sentences_in_order() {
        let tasks = TaskList::from_text("walk the dog\ncall Mum?\n");
        assert_eq!(pinned_text(&tasks, "none"), "Walk the dog.\nCall Mum?");
    }

    #[test]
    fn an_empty_list_is_still_a_message() {
        assert_eq!(pinned_text(&TaskList::default(), "none"), "none");
    }

    #[test]
    fn a_long_list_fits_in_one_message() {
        let tasks = TaskList::from_text(&"a task of some length\n".repeat(400));
        let text = pinned_text(&tasks, "none");
        assert!(text.chars().count() <= LIMIT);
        assert!(text.ends_with('…'));
    }
}
