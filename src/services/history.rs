/* history.rs
 *
 * Copyright 2026 Pietro Campagnano
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

use gtk::glib;

/// Every task that enters or leaves the list, in a file that is only ever
/// appended to, so text removed by mistake can always be found again.
#[derive(Debug)]
pub struct History {
    path: PathBuf,
}

impl History {
    pub fn new() -> Self {
        Self {
            path: glib::user_data_dir().join("nowdothis").join("history.txt"),
        }
    }

    /// Appends what changed between two versions of the list; writes nothing
    /// if no task was added or removed.
    pub fn record(&self, before: &str, after: &str) -> io::Result<()> {
        let now = glib::DateTime::now_local()
            .and_then(|now| now.format("%Y-%m-%dT%H:%M:%S%:z"))
            .map_err(io::Error::other)?;
        let Some(entry) = entry(before, after, &now) else {
            return Ok(());
        };

        if let Some(directory) = self.path.parent() {
            fs::create_dir_all(directory)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(entry.as_bytes())?;
        file.sync_data()
    }
}

/// Compares the lists as collections of lines: moving a task loses no text,
/// so a change of order alone is not worth an entry.
fn entry(before: &str, after: &str, timestamp: &str) -> Option<String> {
    let mut added: Vec<&str> = tasks(after).collect();
    let mut removed = Vec::new();
    for task in tasks(before) {
        match added.iter().position(|candidate| *candidate == task) {
            Some(index) => {
                added.remove(index);
            }
            None => removed.push(task),
        }
    }

    if added.is_empty() && removed.is_empty() {
        return None;
    }

    let mut entry = format!("## {timestamp}\n");
    for task in removed {
        entry.push_str(&format!("- {task}\n"));
    }
    for task in added {
        entry.push_str(&format!("+ {task}\n"));
    }
    Some(entry)
}

fn tasks(text: &str) -> impl Iterator<Item = &str> {
    text.lines().map(str::trim).filter(|line| !line.is_empty())
}

#[cfg(test)]
mod tests {
    use super::entry;

    const AT: &str = "2026-10-09T17:30:00+02";

    #[test]
    fn an_unchanged_list_writes_nothing() {
        assert_eq!(entry("bread\nmilk\n", "bread\nmilk\n", AT), None);
    }

    #[test]
    fn a_new_order_writes_nothing() {
        assert_eq!(entry("bread\nmilk\n", "milk\nbread\n", AT), None);
    }

    #[test]
    fn added_tasks_are_marked_with_a_plus() {
        assert_eq!(
            entry("bread\n", "bread\nmilk\n", AT).as_deref(),
            Some("## 2026-10-09T17:30:00+02\n+ milk\n")
        );
    }

    #[test]
    fn removed_tasks_keep_their_text() {
        assert_eq!(
            entry("bread\nmilk\n", "milk\n", AT).as_deref(),
            Some("## 2026-10-09T17:30:00+02\n- bread\n")
        );
    }

    #[test]
    fn an_edit_shows_the_old_text_and_the_new() {
        assert_eq!(
            entry("call Luca\n", "call Luca and Marta\n", AT).as_deref(),
            Some("## 2026-10-09T17:30:00+02\n- call Luca\n+ call Luca and Marta\n")
        );
    }

    #[test]
    fn removing_one_of_two_identical_tasks_is_recorded() {
        assert_eq!(
            entry("stretch\nstretch\n", "stretch\n", AT).as_deref(),
            Some("## 2026-10-09T17:30:00+02\n- stretch\n")
        );
    }
}
