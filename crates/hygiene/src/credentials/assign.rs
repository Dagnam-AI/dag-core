//! Values assigned to a name like a credential: `"name": "value"` (quoted both sides) and the
//! bare forms of prose, YAML, INI and shell exports (`docs/patterns.md`, P12 to P16).
//!
//! The names are of two kinds. Unambiguous ones (`password`, `secret`, `api_key`, ...) take a
//! quoted value whatever it looks like, and a bare one of six characters with a digit or 16;
//! ambiguous ones (`token`, `authorization`, `auth`, `key`) only a value that is a credential on
//! its own (see `is_credential`).
use std::sync::LazyLock;

use super::{after, for_each_match, is_credential, is_value_stop, name_start_ok, run_end, search};
use crate::detectors::{Pattern, compile};
use crate::unicode::{UnicodeVersion, Versioned, is_py_digit, space_class};

/// The credential names, in the reference's order. `(?i:...)` reads them case-insensitively
/// and, as Python does, the `i` of a name also as U+0130 and U+0131 (the crate folds `k` and `s`
/// with their look-alikes already; `docs/patterns-semantics.md`, item 6).
static NAMES: LazyLock<String> = LazyLock::new(|| {
    let names = "api[_-]?key|access[_-]?key|private[_-]?key|client[_-]?secret\
                 |secret(?:[_-]?access)?(?:[_-]?key)?|password|passwd|authorization|token|auth|key";
    format!("(?i:{})", names.replace('i', r"[i\u{130}\u{131}]"))
});

/// A quoted name and a quoted value: groups 1 (the name), 2 (a value in double quotes) and 3 (a
/// value in single quotes). The value is the text up to the next quote of the kind that opened it.
static QUOTED: Pattern = Versioned::new(|version| {
    let space = space_class(version);
    compile(&format!(
        r#"({names})["']{space}*[:=]{space}*(?:"([^"\n]+)"|'([^'\n]+)')"#,
        names = NAMES.as_str()
    ))
});

/// A name, an optional quote, a `:` or `=` and an optional quote: group 1 is the name, the match
/// ends where the value begins.
static HEAD: Pattern = Versioned::new(|version| {
    let space = space_class(version);
    compile(&format!(
        r#"({names})["']?{space}*[:=]{space}*["']?"#,
        names = NAMES.as_str()
    ))
});

/// A `:` that opens the next assignment (two assignments in one run, a colon apart): used on the text from the
/// colon, so each value is judged on its own.
static NEXT: Pattern = Versioned::new(|version| {
    let space = space_class(version);
    compile(&format!(
        r#"\A:{space}*{names}["']?{space}*[:=]"#,
        names = NAMES.as_str()
    ))
});

/// Whether a value that may be bare is one: 16 characters, or six with a digit.
fn is_long_enough(value: &str, version: UnicodeVersion) -> bool {
    let width = value.chars().take(16).count();
    width >= 16 || (width >= 6 && value.chars().any(|c| is_py_digit(version, c)))
}

/// The first colon of `text[start..run]` that opens the next assignment.
fn next_assignment(text: &str, start: usize, run: usize, version: UnicodeVersion) -> Option<usize> {
    text.get(start..run)
        .unwrap_or_default()
        .match_indices(':')
        .map(|(offset, _)| start + offset)
        .find(|&colon| NEXT.get(version).is_match(after(text, colon)))
}

/// A quoted name with a quoted value, as a span of the value.
fn quoted_spans(text: &str, version: UnicodeVersion) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    for_each_match(
        QUOTED.get(version),
        text,
        |found| name_start_ok(text, found.start()),
        |caps| {
            let value = caps.get(2).or_else(|| caps.get(3));
            spans.extend(
                caps.get(1)
                    .zip(value)
                    .filter(|(name, value)| is_credential(name.as_str(), value.as_str(), version))
                    .map(|(_, value)| (value.start(), value.end())),
            );
            caps.get_match().end()
        },
    );
    spans
}

/// A bare value after a name, as a span of the value. The value is a run of value characters cut
/// short before a `:` that opens the next assignment, and it is a value when it has 16
/// characters, or six with a digit. It is walked in code, not as a repeated group.
fn bare_spans(text: &str, version: UnicodeVersion) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut run: Option<usize> = None; // the run of value characters containing `start`
    let mut position = 0;
    // Every head starts after the position the last one left, so the text's length bounds them.
    for _ in 0..=text.len() {
        let Some(head) = search(HEAD.get(version), text, position, |found| {
            name_start_ok(text, found.start())
        }) else {
            break;
        };
        let whole = head.get_match();
        let name = head.get(1).map(|name| name.as_str());
        let start = whole.end();
        let run_to = run
            .filter(|&end| start <= end)
            .unwrap_or_else(|| run_end(text, start, |c| is_value_stop(c, version)));
        run = Some(run_to);
        let end = next_assignment(text, start, run_to, version).unwrap_or(run_to);
        let value = text.get(start..end).unwrap_or_default();
        if is_long_enough(value, version) {
            if name.is_some_and(|name| is_credential(name, value, version)) {
                spans.push((start, end));
            }
            position = end;
        } else {
            position = super::one_on(text, whole.start());
        }
    }
    spans
}

/// Spans of every credential assigned to a name, quoted or bare.
pub(super) fn assignment_spans(text: &str, version: UnicodeVersion) -> Vec<(usize, usize)> {
    let mut spans = quoted_spans(text, version);
    spans.extend(bare_spans(text, version));
    spans
}
