//! The credentials finder behind the `PII_SECRET` class: provider keys, JWTs, PEM private keys,
//! `Bearer` and `Basic` values, a URL's password, a connection string's `pwd`, and a value
//! assigned to a name like a credential. A port of `find_secrets` in the reference.
//!
//! Every format is syntactic and built for precision; a credential in none of them is not found.
//! Patterns are compiled once from the generated classes of [`crate::unicode`]; a lookbehind is a
//! check in code on the character before a match, and where a long run follows (a URL's
//! password, a bare value) it is walked in code, so time stays linear and memory flat however
//! long a token is. A failed check restarts the search one character after the match's start,
//! as the reference's engine would try the next position.
use regex::{Captures, Match, Regex};

use crate::text::{char_at, char_before};
use crate::unicode::{UnicodeVersion, is_py_space};

mod assign;
mod shapes;
mod url;

/// Spans (bytes) of every credential in `text`; two patterns over one secret are one span.
///
/// A span that starts before the one before it ends is merged into it; spans that only touch
/// stay apart.
#[must_use]
pub fn find_secrets(text: &str, version: UnicodeVersion) -> Vec<(usize, usize)> {
    let mut spans = shapes::shape_spans(text, version);
    spans.extend(assign::assignment_spans(text, version));
    spans.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in spans {
        match merged.last_mut() {
            Some(last) if start < last.1 => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }
    merged
}

/// Whether a value assigned to `name` is a credential: any value under an unambiguous name, and
/// under an ambiguous one only a value that is a credential on its own. The value is never empty
/// here (a quoted one has a character at least, a bare one six).
fn is_credential(name: &str, value: &str, version: UnicodeVersion) -> bool {
    !is_ambiguous(name) || looks_like_credential(value, version)
}

/// `token`, `authorization`, `auth` and `key`, read as Python's `str.lower()` reads them: the
/// Kelvin sign lower-cases to `k`, and the dotted and dotless `i` do not become an ASCII `i`
/// (so such a spelling is not one of these four, and counts as an unambiguous name).
fn is_ambiguous(name: &str) -> bool {
    let lowered: String = name
        .chars()
        .map(|c| {
            if c == '\u{212a}' {
                'k'
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect();
    matches!(lowered.as_str(), "token" | "authorization" | "auth" | "key")
}

/// A value that is a credential on its own, once a name has vouched for it.
fn looks_like_credential(value: &str, version: UnicodeVersion) -> bool {
    !shapes::shape_spans(value, version).is_empty()
        || shapes::has_vouched_key(value)
        || shapes::is_high_entropy(value)
}

/// The text of `text` from byte `at` on; empty when `at` is past the end or inside a character.
fn after(text: &str, at: usize) -> &str {
    text.get(at..).unwrap_or_default()
}

/// The end of the run that begins at `from`: where the first character `stops` reads, or the
/// end of the text.
fn run_end(text: &str, from: usize, stops: impl Fn(char) -> bool) -> usize {
    after(text, from)
        .char_indices()
        .find(|&(_, c)| stops(c))
        .map_or(text.len(), |(offset, _)| from + offset)
}

/// Where the search resumes after a match at `start` that a later check refused: one character
/// on (not one byte, which could land inside a multi-byte character), so a refused match never
/// stops the search from finding the ones that overlap it (the reference's engine would try the
/// next position). Each caller names the position as this does, so a step that did not move on
/// is pinned here, in a test of its own.
fn one_on(text: &str, start: usize) -> usize {
    text.ceil_char_boundary(start + 1)
}

/// The first match of `re` at or after byte `from` that `accept` takes, as Python's
/// `search(text, from)` finds it with a lookbehind that `accept` stands in for: a match it
/// refuses restarts the search one character after the match's start (never at its end).
///
/// The text before `from` stays visible to `accept`, which gets the whole match.
fn search<'t>(
    re: &Regex,
    text: &'t str,
    from: usize,
    accept: impl Fn(Match<'t>) -> bool,
) -> Option<Captures<'t>> {
    std::iter::successors(re.captures_at(text, from), |caps| {
        re.captures_at(text, one_on(text, caps.get_match().start()))
    })
    // Each step starts one character later and a match is not empty, so the text's length bounds
    // the candidates; the bound makes the search finish even if the step were ever wrong.
    .take(text.len())
    .find(|caps| accept(caps.get_match()))
}

/// Every match of `re` that `accept` takes, in order, as Python's `finditer` finds them with a
/// lookbehind that `accept` stands in for. `visit` reads a match and returns where the next
/// search starts: the match's end, or one character after its start when a later check fails
/// (the reference's engine would try the next position).
fn for_each_match<'t>(
    re: &Regex,
    text: &'t str,
    accept: impl Fn(Match<'t>) -> bool,
    mut visit: impl FnMut(&Captures<'t>) -> usize,
) {
    let mut position = 0;
    // Every match ends after it starts, so the text's length bounds the matches; the bound makes
    // the loop finish even if `visit` named a position that did not move on.
    for _ in 0..=text.len() {
        let Some(caps) = search(re, text, position, &accept) else {
            break;
        };
        position = visit(&caps);
    }
}

/// Where a value ends: whitespace, a quote of either kind, a backtick, a comma, a semicolon or an
/// ampersand.
fn is_value_stop(c: char, version: UnicodeVersion) -> bool {
    is_py_space(version, c) || "\"'`,;&".contains(c)
}

/// `(?<![A-Za-z0-9])` or a camelCase hump: the character before a name is not an ASCII letter
/// or digit, or it is a lower-case letter or digit and the name starts with a capital.
fn name_start_ok(text: &str, start: usize) -> bool {
    char_before(text, start).is_none_or(|before| {
        !before.is_ascii_alphanumeric()
            || ((before.is_ascii_lowercase() || before.is_ascii_digit())
                && char_at(text, start).is_some_and(|first| first.is_ascii_uppercase()))
    })
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::{for_each_match, one_on, search};

    #[test]
    fn the_resume_position_is_one_character_after_the_start() {
        assert_eq!(
            [one_on("abc", 0), one_on("abc", 1), one_on("abc", 2)],
            [1, 2, 3]
        );
        // After a character of one to four bytes the next position is the next character's first
        // byte, never a byte inside the one just passed: checked at every character of a string
        // that holds each width.
        let wide = "a\u{e9}\u{17f}\u{212a}\u{1f600}x";
        for (at, c) in wide.char_indices() {
            assert_eq!(one_on(wide, at), at + c.len_utf8(), "{at}");
        }
    }

    #[test]
    fn a_refused_match_restarts_the_search_one_character_after_its_start() {
        let re = Regex::new("a+").unwrap();
        let from_each = |from: usize, min: usize| {
            search(&re, "aaaa", from, |found| found.start() >= min)
                .map(|caps| caps.get_match().range())
        };
        assert_eq!(from_each(0, 0), Some(0..4));
        assert_eq!(from_each(0, 2), Some(2..4));
        assert_eq!(from_each(0, 4), None);
        assert_eq!(from_each(1, 0), Some(1..4));
        let wide = Regex::new("\u{e9}+a").unwrap();
        assert_eq!(
            search(&wide, "\u{e9}\u{e9}a", 0, |found| found.start() > 0)
                .map(|c| c.get_match().range()),
            Some(2..5)
        );
    }

    /// A visitor that never moves the search on still ends: the text's length bounds the calls.
    #[test]
    fn a_visitor_that_does_not_advance_cannot_loop_forever() {
        let (text, mut calls) = ("aaaa", 0);
        for_each_match(
            &Regex::new("a").unwrap(),
            text,
            |_| true,
            |_| {
                calls += 1;
                assert!(calls <= text.len() + 1, "{calls} calls");
                0
            },
        );
        assert_eq!(calls, text.len() + 1);
    }
}
