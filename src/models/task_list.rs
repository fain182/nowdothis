/* task_list.rs
 *
 * Copyright 2026 Pietro Campagnano
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

/// The day's tasks, in the order they will be done.
///
/// The list is its own text representation: one task per line. That keeps the
/// planning text view and the stored file the same thing, with no separate
/// serialisation step in between.
#[derive(Debug, Default)]
pub struct TaskList {
    tasks: Vec<String>,
}

impl TaskList {
    pub fn from_text(text: &str) -> Self {
        let tasks = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(String::from)
            .collect();

        Self { tasks }
    }

    pub fn to_text(&self) -> String {
        let mut text = self.tasks.join("\n");
        if !text.is_empty() {
            text.push('\n');
        }
        text
    }

    /// The one task the user is meant to be doing right now.
    pub fn current(&self) -> Option<&str> {
        self.tasks.first().map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn complete_current(&mut self) {
        if !self.tasks.is_empty() {
            self.tasks.remove(0);
        }
    }

    /// Appends to the end, never in front of the current task: a task added
    /// mid-execution must not change what the user is doing right now.
    pub fn append(&mut self, task: &str) {
        let task = task.trim();
        if !task.is_empty() {
            self.tasks.push(task.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_lines_do_not_become_tasks() {
        let list = TaskList::from_text("walk the dog\n\n  \npick up milk\n");
        assert_eq!(list.current(), Some("walk the dog"));
        assert_eq!(list.to_text(), "walk the dog\npick up milk\n");
    }

    #[test]
    fn completing_advances_to_the_next_task() {
        let mut list = TaskList::from_text("first\nsecond\n");
        list.complete_current();
        assert_eq!(list.current(), Some("second"));
        list.complete_current();
        assert!(list.is_empty());
        assert_eq!(list.to_text(), "");
    }

    #[test]
    fn appending_leaves_the_current_task_alone() {
        let mut list = TaskList::from_text("first\n");
        list.append("  later  ");
        assert_eq!(list.current(), Some("first"));
        assert_eq!(list.to_text(), "first\nlater\n");
    }
}
