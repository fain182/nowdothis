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

/// The list as it reads in the pinned Telegram message, in Telegram's HTML:
/// a bold title, so the pin is recognisable at a glance, then one sentence per
/// line with the current task pointed at. `empty` stands in for an empty list.
pub fn pinned_text(tasks: &TaskList, title: &str, empty: &str) -> String {
    let mut text = format!("<b>📋 {}</b>\n\n", escaped(title));

    if tasks.is_empty() {
        text.push_str(&format!("🎉 {}", escaped(empty)));
        return text;
    }

    for (index, task) in tasks.iter().enumerate() {
        let marker = if index == 0 { "👉" } else { "•" };
        let line = format!("{marker} {}", escaped(&as_sentence(task)));
        // Whole lines are dropped rather than cut, and the ellipsis says so.
        // Counting the markup as well keeps this on the safe side.
        if text.chars().count() + line.chars().count() + 2 > LIMIT {
            text.push('…');
            break;
        }
        text.push_str(&line);
        text.push('\n');
        // The task at hand stands apart from what comes after it.
        if index == 0 {
            text.push('\n');
        }
    }

    text.trim_end_matches('\n').to_string()
}

/// Telegram's HTML only needs these three escaped outside tags.
pub fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_current_task_is_pointed_at() {
        let tasks = TaskList::from_text("walk the dog\ncall Mum?\n");
        assert_eq!(
            pinned_text(&tasks, "Today", "none"),
            "<b>📋 Today</b>\n\n👉 Walk the dog.\n\n• Call Mum?"
        );
    }

    #[test]
    fn a_single_task_has_nothing_after_it() {
        let tasks = TaskList::from_text("walk the dog\n");
        assert_eq!(pinned_text(&tasks, "Today", "none"), "<b>📋 Today</b>\n\n👉 Walk the dog.");
    }

    #[test]
    fn an_empty_list_still_has_its_title() {
        assert_eq!(
            pinned_text(&TaskList::default(), "Today", "none"),
            "<b>📋 Today</b>\n\n🎉 none"
        );
    }

    #[test]
    fn tasks_cannot_inject_markup() {
        let tasks = TaskList::from_text("compare <b> & <i>\n");
        assert!(pinned_text(&tasks, "Today", "none").ends_with("👉 Compare &lt;b&gt; &amp; &lt;i&gt;"));
    }

    #[test]
    fn a_long_list_fits_in_one_message() {
        let tasks = TaskList::from_text(&"a task of some length\n".repeat(400));
        let text = pinned_text(&tasks, "Today", "none");
        assert!(text.chars().count() <= LIMIT);
        assert!(text.ends_with('…'));
    }
}
