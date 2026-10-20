/* sentence.rs
 *
 * Copyright 2026 Pietro Campagnano
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

/// The task is shown as a sentence spoken to the user, so it is given a capital
/// and a full stop. Both are presentation only: the list on disk keeps the
/// words exactly as they were typed.
pub fn as_sentence(task: &str) -> String {
    let mut sentence = capitalised(task);

    if sentence.chars().last().is_some_and(char::is_alphanumeric) {
        sentence.push('.');
    }

    sentence
}

/// Leaves a first word that carries a capital of its own alone, so names like
/// "iPhone" are not mangled into "IPhone".
fn capitalised(task: &str) -> String {
    let first_word = task.split_whitespace().next().unwrap_or_default();
    if first_word.chars().skip(1).any(char::is_uppercase) {
        return task.to_string();
    }

    let mut characters = task.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::as_sentence;

    #[test]
    fn a_task_is_shown_as_a_sentence() {
        assert_eq!(as_sentence("walk the dog"), "Walk the dog.");
    }

    #[test]
    fn punctuation_of_its_own_is_left_alone() {
        assert_eq!(as_sentence("call Mum?"), "Call Mum?");
        assert_eq!(as_sentence("ship it!"), "Ship it!");
        assert_eq!(as_sentence("read chapter 3."), "Read chapter 3.");
    }

    #[test]
    fn a_name_that_starts_lowercase_keeps_its_own_shape() {
        assert_eq!(as_sentence("iPhone backup"), "iPhone backup.");
        assert_eq!(as_sentence("eBay listing"), "eBay listing.");
    }

    #[test]
    fn a_task_that_starts_with_a_number_is_not_forced() {
        assert_eq!(as_sentence("3pm standup"), "3pm standup.");
    }
}
