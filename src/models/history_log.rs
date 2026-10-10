/* history_log.rs
 *
 * Copyright 2026 Pietro Campagnano
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gtk::glib;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Added,
    Completed,
    Deleted,
}

#[derive(Debug)]
pub struct Event {
    pub change: Change,
    pub task: String,
    pub at: glib::DateTime,
}

/// The history file read back as events, oldest first.
#[derive(Debug, Default)]
pub struct HistoryLog {
    events: Vec<Event>,
}

impl HistoryLog {
    /// Lines that cannot be placed in time or have no known mark are skipped,
    /// so a damaged entry costs only itself.
    pub fn from_text(text: &str) -> Self {
        let mut events = Vec::new();
        let mut at = None;

        for line in text.lines() {
            if let Some(timestamp) = line.strip_prefix("## ") {
                at = glib::DateTime::from_iso8601(timestamp, None)
                    .and_then(|at| at.to_local())
                    .ok();
                continue;
            }

            let change = match line.get(..2) {
                Some("+ ") => Change::Added,
                Some("x ") => Change::Completed,
                Some("- ") => Change::Deleted,
                _ => continue,
            };
            if let Some(at) = &at {
                events.push(Event {
                    change,
                    task: line[2..].to_string(),
                    at: at.clone(),
                });
            }
        }

        Self { events }
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use super::{Change, HistoryLog};

    fn changes(log: &HistoryLog) -> Vec<(Change, &str)> {
        log.events()
            .iter()
            .map(|event| (event.change, event.task.as_str()))
            .collect()
    }

    #[test]
    fn each_mark_is_read_as_its_change() {
        let log = HistoryLog::from_text(
            "## 2026-10-10T11:48:05+02:00\n+ buy bread\nx call Luca\n- try",
        );
        assert_eq!(
            changes(&log),
            [
                (Change::Added, "buy bread"),
                (Change::Completed, "call Luca"),
                (Change::Deleted, "try"),
            ]
        );
    }

    #[test]
    fn a_task_keeps_the_time_of_its_entry() {
        let log = HistoryLog::from_text("## 2026-10-10T11:48:05+02:00\n+ buy bread\n");
        let at = log.events()[0].at.to_utc().unwrap();
        assert_eq!((at.hour(), at.minute()), (9, 48));
    }

    #[test]
    fn lines_without_a_valid_time_are_skipped() {
        let log = HistoryLog::from_text(
            "+ before any entry\n## not a time\n+ under a bad time\n## 2026-10-10T11:48:05+02:00\n+ kept\n",
        );
        assert_eq!(changes(&log), [(Change::Added, "kept")]);
    }

    #[test]
    fn a_task_may_start_like_a_mark() {
        let log = HistoryLog::from_text("## 2026-10-10T11:48:05+02:00\nx x-ray\n");
        assert_eq!(changes(&log), [(Change::Completed, "x-ray")]);
    }
}
