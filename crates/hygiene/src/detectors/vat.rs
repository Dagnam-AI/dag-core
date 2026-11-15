//! EU VAT numbers: a member state's prefix and its format (no per-state checksum).
use std::sync::LazyLock;

use super::{Pattern, compile, group_span, not_ascii_alnum, scan};
use crate::text::{DIGIT_CLASS, char_before};

/// Each member state's format after its prefix (`EL` is Greece), in the reference's order. `D`
/// stands for a decimal digit of any script, as the reference's `\d` reads.
const FORMATS: [(&str, &str); 27] = [
    ("AT", r"UD{8}"),
    ("BE", r"[01]D{9}"),
    ("BG", r"D{9,10}"),
    ("CY", r"D{8}[A-Z]"),
    ("CZ", r"D{8,10}"),
    ("DE", r"D{9}"),
    ("DK", r"D{8}"),
    ("EE", r"D{9}"),
    ("EL", r"D{9}"),
    ("ES", r"[A-Z0-9]D{7}[A-Z0-9]"),
    ("FI", r"D{8}"),
    ("FR", r"[A-HJ-NP-Z0-9]{2}D{9}"),
    ("HR", r"D{11}"),
    ("HU", r"D{8}"),
    ("IE", r"D{7}[A-W][A-I]?|D[A-Z+*]D{5}[A-W]"),
    ("IT", r"D{11}"),
    ("LT", r"D{9}|D{12}"),
    ("LU", r"D{8}"),
    ("LV", r"D{11}"),
    ("MT", r"D{8}"),
    ("NL", r"D{9}BD{2}"),
    ("PL", r"D{10}"),
    ("PT", r"D{9}"),
    // Romania's CUI runs 2-10 digits; under 6 it is too short to tell from a code.
    ("RO", r"[1-9]D{5,9}"),
    ("SE", r"D{10}01"),
    ("SI", r"D{8}"),
    ("SK", r"D{10}"),
];

/// One alternative per state: the prefix, an optional space and the format; then a character that
/// is not an ASCII letter or digit (or the end), looked at and left for the next search, so the
/// shorter of two formats (`LT`) gives way to the longer (`docs/patterns.md`, P30).
static EU_VAT: Pattern = LazyLock::new(|| {
    let digit = DIGIT_CLASS.as_str();
    let states: Vec<String> = FORMATS
        .iter()
        .map(|(state, format)| {
            // The format has a bare `D` (a digit) and `B` (a letter) only in the places a
            // digit of any script or the letter belongs: one replacement of `D` is safe.
            format!("{state} ?(?:{})", format.replace('D', digit))
        })
        .collect();
    compile(&format!("((?:{}))(?:\\z|[^A-Za-z0-9])", states.join("|")))
});

/// EU VAT numbers, by the member state's format.
#[must_use]
pub fn find_eu_vat(text: &str) -> Vec<(usize, usize)> {
    scan(&EU_VAT, text, |start, caps| {
        group_span(caps, 1).filter(|_| not_ascii_alnum(char_before(text, start)))
    })
}
