//! IBANs: a registered country, its exact length, and mod-97 equal to 1.
use super::{Pattern, compile, digit_value, group_span, not_ascii_alnum, scan};
use crate::text::char_before;
use crate::unicode::{UnicodeVersion, Versioned, digit_class};

/// The SWIFT IBAN registry's length per country.
const LENGTHS: &[(&str, usize)] = &[
    ("AD", 24),
    ("AE", 23),
    ("AL", 28),
    ("AT", 20),
    ("AZ", 28),
    ("BA", 20),
    ("BE", 16),
    ("BG", 22),
    ("BH", 22),
    ("BI", 27),
    ("BR", 29),
    ("BY", 28),
    ("CH", 21),
    ("CR", 22),
    ("CY", 28),
    ("CZ", 24),
    ("DE", 22),
    ("DJ", 27),
    ("DK", 18),
    ("DO", 28),
    ("EE", 20),
    ("EG", 29),
    ("ES", 24),
    ("FI", 18),
    ("FK", 18),
    ("FO", 18),
    ("FR", 27),
    ("GB", 22),
    ("GE", 22),
    ("GI", 23),
    ("GL", 18),
    ("GR", 27),
    ("GT", 28),
    ("HR", 21),
    ("HU", 28),
    ("IE", 22),
    ("IL", 23),
    ("IQ", 23),
    ("IS", 26),
    ("IT", 27),
    ("JO", 30),
    ("KW", 30),
    ("KZ", 20),
    ("LB", 28),
    ("LC", 32),
    ("LI", 21),
    ("LT", 20),
    ("LU", 20),
    ("LV", 21),
    ("LY", 25),
    ("MC", 27),
    ("MD", 24),
    ("ME", 22),
    ("MK", 19),
    ("MN", 20),
    ("MR", 27),
    ("MT", 31),
    ("MU", 30),
    ("NI", 28),
    ("NL", 18),
    ("NO", 15),
    ("OM", 23),
    ("PK", 24),
    ("PL", 28),
    ("PS", 29),
    ("PT", 25),
    ("QA", 29),
    ("RO", 24),
    ("RS", 22),
    ("RU", 33),
    ("SA", 24),
    ("SC", 31),
    ("SD", 18),
    ("SE", 24),
    ("SI", 19),
    ("SK", 24),
    ("SM", 27),
    ("SO", 23),
    ("ST", 25),
    ("SV", 28),
    ("TL", 23),
    ("TN", 24),
    ("TR", 26),
    ("UA", 29),
    ("VA", 22),
    ("VG", 24),
    ("XK", 20),
    ("YE", 30),
];

/// Spaced in fours or compact; a candidate may run on into a following word, which the
/// registered length then trims off. Then a character that is not an ASCII letter or digit (or
/// the end), looked at and left for the next search (`docs/patterns.md`, P21).
static CANDIDATE: Pattern = Versioned::new(|version| {
    let d = digit_class(version);
    compile(&format!(
        "([A-Z]{{2}}{d}{{2}}(?: ?[A-Z0-9]{{4}}){{2,7}}(?: ?[A-Z0-9]{{1,4}})?)(?:\\z|[^A-Za-z0-9])"
    ))
});

/// Whether the compact `iban` passes mod-97: the first four characters moved to the end, every
/// letter read as the two-digit number 10 to 35, and the whole number equal to 1 mod 97.
#[must_use]
pub fn iban_mod97_ok(iban: &str, version: UnicodeVersion) -> bool {
    let remainder = iban
        .chars()
        .skip(4)
        .chain(iban.chars().take(4))
        .fold(0, |remainder, c| {
            let value = c.to_digit(36).unwrap_or_else(|| digit_value(c, version));
            let shift = if value < 10 { 10 } else { 100 };
            (remainder * shift + value) % 97
        });
    remainder == 1
}

/// Where the IBAN in `raw`, a candidate, ends (a byte offset), if it has one: the country is
/// registered, the registered length closes a group, and mod-97 holds.
fn iban_end(raw: &str, version: UnicodeVersion) -> Option<usize> {
    raw.get(..2)
        .and_then(|country| LENGTHS.iter().find(|&&(code, _)| code == country))
        .and_then(|&(_, length)| {
            raw.char_indices()
                .filter(|&(_, c)| c != ' ')
                .nth(length - 1)
        })
        .map(|(index, c)| index + c.len_utf8())
        .filter(|&end| {
            raw.get(end..)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
        })
        .filter(|&end| {
            raw.get(..end)
                .is_some_and(|iban| iban_mod97_ok(&iban.replace(' ', ""), version))
        })
}

/// IBANs: a registered country, its exact length, and mod-97 == 1.
#[must_use]
pub fn find_ibans(text: &str, version: UnicodeVersion) -> Vec<(usize, usize)> {
    let candidates = scan(CANDIDATE.get(version), text, |start, caps| {
        group_span(caps, 1).filter(|_| not_ascii_alnum(char_before(text, start)))
    });
    candidates
        .into_iter()
        .filter_map(|(from, to)| {
            text.get(from..to)
                .and_then(|raw| iban_end(raw, version))
                .map(|end| (from, from + end))
        })
        .collect()
}
