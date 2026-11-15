//! The PII detectors: one finder per class, each a port of the reference function of the
//! same name, and the registry that orders them.
//!
//! A finder takes a text and returns the byte spans of its findings, ascending and without
//! overlap. Every pattern is compiled once, from the generated classes of [`crate::text`]
//! (never the crate's own `\w`, `\s`, `\d` or `\b`), and a lookaround the crate cannot
//! express is a check in code, as `docs/patterns.md` routes it.
use std::sync::LazyLock;

use regex::{Captures, Regex};

use crate::text::is_py_digit;

mod cards;
mod dates;
mod iban;
mod ip;

pub use cards::{find_payment_cards, luhn_ok};
pub use dates::find_dates_of_birth;
pub use iban::{find_ibans, iban_mod97_ok};
pub use ip::{find_ip_addresses, is_ip_address};

/// Compiles a detector pattern.
///
/// The pattern is built from constants of this crate, so a failure is a defect in the source
/// and every test of the finder hits it; no input can reach it.
#[allow(clippy::expect_used)] // see above: a constant pattern that does not compile is a source defect
pub(crate) fn compile(pattern: &str) -> Regex {
    Regex::new(pattern).expect("a detector pattern is valid regex syntax")
}

/// Every match `re` makes on `text` that `accept` takes, as Python's `finditer` would find
/// them with the lookarounds `accept` stands in for.
///
/// `accept` gets the start of the whole match and its captures and returns the span to
/// report, or `None` when a guard fails. A failed guard means "no match at this start", so
/// the search restarts one **character** after that start (never one byte, never at the
/// match end). An accepted span ends the match: the search resumes at its end.
///
/// The loop is bounded by the text's length, which every iteration outruns (each moves the
/// search at least one byte on), so a defect in the advance fails a test instead of hanging.
pub(crate) fn scan<'t>(
    re: &Regex,
    text: &'t str,
    accept: impl Fn(usize, &Captures<'t>) -> Option<(usize, usize)>,
) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut at = 0;
    for _ in 0..=text.len() {
        let Some((start, caps)) = re.captures_at(text, at).and_then(|caps| {
            caps.get(0)
                .map(|whole| whole.start())
                .map(|start| (start, caps))
        }) else {
            break;
        };
        if let Some((from, to)) = accept(start, &caps) {
            spans.push((from, to));
            at = to;
        } else {
            at = text.ceil_char_boundary(start + 1);
        }
    }
    spans
}

/// The byte span of capture group `group`.
pub(crate) fn group_span(caps: &Captures<'_>, group: usize) -> Option<(usize, usize)> {
    caps.get(group).map(|found| (found.start(), found.end()))
}

/// The value 0 to 9 of a decimal digit of any script, as Python's `int` reads it.
///
/// Every run of decimal digits in Unicode starts at a zero and counts up in tens, so a digit's
/// value is how many digits run unbroken before it, modulo ten. `c` must satisfy
/// [`is_py_digit`]; any other character has no value.
pub(crate) fn digit_value(c: char) -> u32 {
    let code = u32::from(c);
    let before: u32 = (1..=code)
        .take_while(|back| {
            code.checked_sub(*back)
                .and_then(char::from_u32)
                .is_some_and(is_py_digit)
        })
        .map(|_| 1)
        .sum();
    before % 10
}

/// Whether `c` is not an ASCII letter or digit.
pub(crate) fn not_ascii_alnum(c: Option<char>) -> bool {
    !c.is_some_and(|c| c.is_ascii_alphanumeric())
}

/// Whether `c` is not a decimal digit of any script and not a hyphen.
pub(crate) fn not_digit_or_hyphen(c: Option<char>) -> bool {
    !c.is_some_and(|c| is_py_digit(c) || c == '-')
}

/// The digits of `text` (every decimal digit of any script), as a string for a checksum.
pub(crate) fn digits_of(text: &str) -> String {
    text.chars().filter(|c| is_py_digit(*c)).collect()
}

/// A lazily compiled pattern, for the statics of the finders.
pub(crate) type Pattern = LazyLock<Regex>;
