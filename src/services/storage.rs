/* storage.rs
 *
 * Copyright 2026 Pietro Campagnano
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::fs;
use std::io;
use std::path::PathBuf;

use gtk::glib;

/// Reads and writes the task list as a plain text file in the user's data
/// directory.
#[derive(Debug)]
pub struct Storage {
    path: PathBuf,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            path: glib::user_data_dir().join("nowdothis").join("tasks.txt"),
        }
    }

    /// A missing file is an empty list, not an error: it is what a first run
    /// looks like.
    pub fn load(&self) -> io::Result<String> {
        match fs::read_to_string(&self.path) {
            Ok(text) => Ok(text),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
            Err(error) => Err(error),
        }
    }

    pub fn save(&self, text: &str) -> io::Result<()> {
        if let Some(directory) = self.path.parent() {
            fs::create_dir_all(directory)?;
        }
        fs::write(&self.path, text)
    }
}
