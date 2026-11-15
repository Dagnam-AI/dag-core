//! Dates of birth: a date, but only right after a birth keyword.
use std::sync::LazyLock;

use super::{Pattern, compile, group_span, scan};
use crate::text::{DIGIT_CLASS, SPACE_CLASS, char_before};

/// Whether `c` is one of the characters Python's `[A-Za-z]` reads under `(?i)`: the ASCII
/// letters and U+0130, U+0131 (for `i`), U+017F (for `s`) and U+212A (for `k`).
pub(super) fn is_ci_ascii_letter(c: char) -> bool {
    c.is_ascii_alphabetic() || matches!(c, '\u{130}' | '\u{131}' | '\u{17f}' | '\u{212a}')
}

/// The keyword classes (date of birth, SIN, Aadhaar by name) open after anything but a
/// letter, not at a `\b`: a digit glued to the keyword is the end of the previous number.
///
/// Under `(?i)` the `i` of `birth`, `is` and `april` also reads U+0130 and U+0131 (`docs/
/// patterns.md`, P25, semantics 6), and a character after the date that is a digit, or a
/// `/`, `.` or `-` with a digit after it, ends the search at that date: the terminator is
/// looked at and left for the next search, so a shorter date end can still be taken.
static DATE_OF_BIRTH: Pattern = LazyLock::new(|| {
    let (d, s) = (DIGIT_CLASS.as_str(), SPACE_CLASS.as_str());
    let i = r"[i\u{130}\u{131}]";
    let day = format!("(?:0?[1-9]|[12]{d}|3[01])");
    let month_number = "(?:0?[1-9]|1[0-2])";
    let month = format!(
        "(?:jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:{i}l)?|may|june?|july?|aug(?:ust)?|sep(?:t(?:ember)?)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\\.?"
    );
    let year = format!("(?:19|20){d}{{2}}");
    let date = format!(
        "(?:{day}[/.\\-]{month_number}|{month_number}[/.\\-]{day})[/.\\-](?:19|20)?{d}{{2}}\
         |{year}[/.\\-]{month_number}[/.\\-]{day}\
         |{day}(?:st|nd|rd|th)?{s}+{month},?{s}+{year}\
         |{month}{s}+{day}(?:st|nd|rd|th)?,?{s}+{year}"
    );
    compile(&format!(
        "(?i)(?:d\\.?o\\.?b\\.?|date{s}+of{s}+b{i}rth|b{i}rth{s}*date|b{i}rthday|born(?:{s}+on)?)\
         [{s}:=,\\-]*(?:(?:{i}s|was){s}+)?({date})\
         (?:\\z|[^{d}/.\\-]|[/.\\-](?:\\z|[^{d}]))"
    ))
});

/// Dates of birth: the date after a keyword, the date only.
#[must_use]
pub fn find_dates_of_birth(text: &str) -> Vec<(usize, usize)> {
    scan(&DATE_OF_BIRTH, text, |start, caps| {
        group_span(caps, 1).filter(|_| !char_before(text, start).is_some_and(is_ci_ascii_letter))
    })
}
