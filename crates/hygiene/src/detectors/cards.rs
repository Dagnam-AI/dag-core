//! Payment cards: 13 to 19 digits that pass Luhn.
use std::sync::LazyLock;

use super::{Pattern, compile, digit_value, digits_of, group_span, not_digit_or_hyphen, scan};
use crate::text::{DIGIT_CLASS, char_before};

/// 13 to 19 digits, adjacent digits at most one space or hyphen apart, then a character that
/// is not a digit or hyphen (or the end), which is looked at and left for the next search
/// (`docs/patterns.md`, P20; the terminator lets a shorter end through where the longest meets
/// a hyphen, as Python's backtracking does).
static CANDIDATE: Pattern = LazyLock::new(|| {
    let d = DIGIT_CLASS.as_str();
    compile(&format!(r"((?:{d}[ \-]?){{12,18}}{d})(?:\z|[^{d}\-])"))
});

/// Standard Luhn check over a digit string: digits of any script, read by their value.
#[must_use]
pub fn luhn_ok(digits: &str) -> bool {
    let total: u32 = digits
        .chars()
        .rev()
        .enumerate()
        .map(|(position, c)| {
            let value = digit_value(c);
            if position % 2 == 1 {
                // The digit sum of the doubled value: 14 is 1 + 4, that is 14 - 9, for 5 and up.
                value * 2 - 9 * (value / 5)
            } else {
                value
            }
        })
        .sum();
    total.is_multiple_of(10)
}

/// Spans of 13-19 digit runs that pass Luhn.
///
/// The Luhn check is what makes this class worth shipping: without it, every order id and
/// phone number in the dataset is a false positive. The candidate holds 13 to 19 digits by its
/// shape, so there is no length test.
#[must_use]
pub fn find_payment_cards(text: &str) -> Vec<(usize, usize)> {
    let candidates = scan(&CANDIDATE, text, |start, caps| {
        group_span(caps, 1).filter(|_| not_digit_or_hyphen(char_before(text, start)))
    });
    candidates
        .into_iter()
        .filter(|&(from, to)| {
            text.get(from..to)
                .is_some_and(|candidate| luhn_ok(&digits_of(candidate)))
        })
        .collect()
}
