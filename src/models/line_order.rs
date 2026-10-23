/* line_order.rs
 *
 * Copyright 2026 Pietro Campagnano
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

/// The text with line `from` taken out and put back so that it ends up as line
/// `to`. Works on the lines as typed, blank ones included, so it can be applied
/// to the text view without disturbing anything else in it.
pub fn move_line(text: &str, from: usize, to: usize) -> String {
    let mut lines: Vec<&str> = text.split('\n').collect();
    if from < lines.len() && to < lines.len() {
        let line = lines.remove(from);
        lines.insert(to, line);
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_moves_down() {
        assert_eq!(move_line("a\nb\nc", 0, 2), "b\nc\na");
    }

    #[test]
    fn a_line_moves_up() {
        assert_eq!(move_line("a\nb\nc", 2, 0), "c\na\nb");
    }

    #[test]
    fn blank_lines_keep_their_place() {
        assert_eq!(move_line("a\n\nb\nc", 3, 0), "c\na\n\nb");
    }

    #[test]
    fn a_line_out_of_range_changes_nothing() {
        assert_eq!(move_line("a\nb", 0, 5), "a\nb");
        assert_eq!(move_line("a\nb", 5, 0), "a\nb");
    }
}
