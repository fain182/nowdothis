/* history_log.rs
 *
 * Copyright 2026 Pietro Campagnano
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gtk::glib;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// On the list when the history began, rather than added at that moment.
    Initial,
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
        let mut entries = 0;

        for line in text.lines() {
            if let Some(timestamp) = line.strip_prefix("## ") {
                entries += 1;
                at = glib::DateTime::from_iso8601(timestamp, None)
                    .and_then(|at| at.to_local())
                    .ok();
                continue;
            }

            let change = match line.get(..2) {
                Some("+ ") if entries == 1 => Change::Initial,
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

    /// A task cut and pasted elsewhere in the list is written as deleted and
    /// then added again; within one day that is a move, and both are dropped.
    pub fn without_moves(&self) -> Vec<&Event> {
        let mut moved = vec![false; self.events.len()];

        for (index, event) in self.events.iter().enumerate() {
            if event.change != Change::Deleted || moved[index] {
                continue;
            }
            let pasted = self.events[index + 1..]
                .iter()
                .enumerate()
                .map(|(offset, later)| (index + 1 + offset, later))
                .find(|(later_index, later)| {
                    !moved[*later_index]
                        && later.change == Change::Added
                        && later.task == event.task
                        && later.at.ymd() == event.at.ymd()
                });
            if let Some((pasted_index, _)) = pasted {
                moved[index] = true;
                moved[pasted_index] = true;
            }
        }

        self.events
            .iter()
            .zip(moved)
            .filter(|(_, moved)| !moved)
            .map(|(event, _)| event)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{Change, HistoryLog};

    fn changes(log: &HistoryLog) -> Vec<(Change, &str)> {
        log.events
            .iter()
            .map(|event| (event.change, event.task.as_str()))
            .collect()
    }

    fn shown(log: &HistoryLog) -> Vec<(Change, &str)> {
        log.without_moves()
            .into_iter()
            .map(|event| (event.change, event.task.as_str()))
            .collect()
    }

    const START: &str = "## 2026-10-10T08:00:00+02:00\n+ on the list\n";

    #[test]
    fn the_first_entry_is_the_list_as_found() {
        let log = HistoryLog::from_text(START);
        assert_eq!(changes(&log), [(Change::Initial, "on the list")]);
    }

    #[test]
    fn each_mark_is_read_as_its_change() {
        let log = HistoryLog::from_text(&format!(
            "{START}## 2026-10-10T11:48:05+02:00\n+ buy bread\nx call Luca\n- try",
        ));
        assert_eq!(
            changes(&log)[1..],
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
        let at = log.events[0].at.to_utc().unwrap();
        assert_eq!((at.hour(), at.minute()), (9, 48));
    }

    #[test]
    fn lines_without_a_valid_time_are_skipped() {
        let log = HistoryLog::from_text(
            "+ before any entry\n## not a time\n+ under a bad time\n## 2026-10-10T11:48:05+02:00\n- kept\n",
        );
        assert_eq!(changes(&log), [(Change::Deleted, "kept")]);
    }

    #[test]
    fn a_task_may_start_like_a_mark() {
        let log = HistoryLog::from_text("## 2026-10-10T11:48:05+02:00\nx x-ray\n");
        assert_eq!(changes(&log), [(Change::Completed, "x-ray")]);
    }

    #[test]
    fn a_task_cut_and_pasted_the_same_day_is_not_shown() {
        let log = HistoryLog::from_text(&format!(
            "{START}## 2026-10-10T11:07:00+02:00\n- on the list\n## 2026-10-10T11:08:00+02:00\n+ on the list\n",
        ));
        assert_eq!(shown(&log), [(Change::Initial, "on the list")]);
    }

    #[test]
    fn a_task_deleted_and_added_on_different_days_is_shown() {
        let log = HistoryLog::from_text(&format!(
            "{START}## 2026-10-10T11:07:00+02:00\n- on the list\n## 2026-10-11T09:00:00+02:00\n+ on the list\n",
        ));
        assert_eq!(
            shown(&log)[1..],
            [(Change::Deleted, "on the list"), (Change::Added, "on the list")]
        );
    }

    #[test]
    fn a_task_added_then_deleted_is_shown() {
        let log = HistoryLog::from_text(&format!(
            "{START}## 2026-10-10T11:07:00+02:00\n+ try\n## 2026-10-10T11:08:00+02:00\n- try\n",
        ));
        assert_eq!(
            shown(&log)[1..],
            [(Change::Added, "try"), (Change::Deleted, "try")]
        );
    }
}
