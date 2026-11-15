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
mod national;
mod vat;

pub use cards::{find_payment_cards, luhn_ok};
pub use dates::find_dates_of_birth;
pub use iban::{find_ibans, iban_mod97_ok};
pub use ip::{find_ip_addresses, is_ip_address};
pub use national::{
    find_ca_sins, find_in_aadhaar, find_in_pans, find_national_ids, find_uk_ninos, verhoeff_ok,
};
pub use vat::find_eu_vat;

/// One class of finding: its code, whether its pattern is loose, and the function that finds it.
///
/// A loose class is one whose pattern also covers other classes' spans (a phone pattern reads
/// an IP address or an Aadhaar number as digits): where a stricter class's finding overlaps one
/// of a loose class's, the stricter class names the text.
pub struct Detector {
    /// The class code, as the findings carry it (`PII_PAYMENT_CARD`).
    pub code: &'static str,
    /// Whether a stricter class wins over this one where their findings overlap.
    pub loose: bool,
    /// The byte spans of this class's findings in a text, ascending and without overlap.
    pub find: fn(&str) -> Vec<(usize, usize)>,
}

/// Every class with a finder in this module, in the reference's registry order: the order is part
/// of the ranking when two findings tie.
pub static DETECTORS: &[Detector] = &[
    Detector {
        code: "PII_PAYMENT_CARD",
        loose: false,
        find: find_payment_cards,
    },
    Detector {
        code: "PII_NATIONAL_ID",
        loose: false,
        find: find_national_ids,
    },
    Detector {
        code: "PII_IBAN",
        loose: false,
        find: find_ibans,
    },
    Detector {
        code: "PII_IP_ADDRESS",
        loose: false,
        find: find_ip_addresses,
    },
    Detector {
        code: "PII_DATE_OF_BIRTH",
        loose: false,
        find: find_dates_of_birth,
    },
    Detector {
        code: "PII_UK_NINO",
        loose: false,
        find: find_uk_ninos,
    },
    Detector {
        code: "PII_CA_SIN",
        loose: false,
        find: find_ca_sins,
    },
    Detector {
        code: "PII_IN_AADHAAR",
        loose: false,
        find: find_in_aadhaar,
    },
    Detector {
        code: "PII_IN_PAN",
        loose: false,
        find: find_in_pans,
    },
    Detector {
        code: "PII_EU_VAT",
        loose: false,
        find: find_eu_vat,
    },
];

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
