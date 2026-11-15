//! National identifiers: the US SSN, the UK NINO, the Canadian SIN, the Indian Aadhaar and PAN.
use std::sync::LazyLock;

use super::{
    Pattern, compile, digit_value, digits_of, group_span, not_ascii_alnum, not_digit_or_hyphen,
    scan,
};
use crate::detectors::cards::luhn_ok;
use crate::text::{DIGIT_CLASS, SPACE_CLASS, char_at, char_before, is_py_digit};

/// `\d{3}-\d{2}-\d{4}`; the area, group and serial rules are checked on the three groups
/// (`docs/patterns.md`, P31).
static SSN: Pattern = LazyLock::new(|| {
    let d = DIGIT_CLASS.as_str();
    compile(&format!("({d}{{3}})-({d}{{2}})-({d}{{4}})"))
});

/// US SSNs: the area/group/serial rules are part of the match, since 000/666/9xx areas and 00
/// groups / 0000 serials are never issued, and excluding them is what keeps ordinary 9-digit
/// ids out of the results. The rules compare the ASCII text, so another script's zero passes.
#[must_use]
pub fn find_national_ids(text: &str) -> Vec<(usize, usize)> {
    scan(&SSN, text, |start, caps| {
        // `extract` cannot panic: the pattern is three groups, each one taking part in every match.
        let (matched, [area, group, serial]) = caps.extract();
        let end = start + matched.len();
        let issued = !matches!(area, "000" | "666")
            && !area.starts_with('9')
            && group != "00"
            && serial != "0000";
        let alone = not_digit_or_hyphen(char_before(text, start))
            && not_digit_or_hyphen(char_at(text, end));
        (issued && alone).then_some((start, end))
    })
}

/// Two letters, three pairs of digits and a suffix, each separated by at most one space
/// (`docs/patterns.md`, P26). The prefixes the first two letters may not form are checked in code.
static NINO: Pattern = LazyLock::new(|| {
    let d = DIGIT_CLASS.as_str();
    compile(&format!(
        "([A-CEGHJ-PR-TW-Z][A-CEGHJ-NPR-TW-Z] ?{d}{{2}} ?{d}{{2}} ?{d}{{2}} ?[A-D])"
    ))
});

/// The letter pairs a UK National Insurance number never starts with.
const NINO_EXCLUDED: [&str; 7] = ["BG", "GB", "KN", "NK", "NT", "TN", "ZZ"];

/// UK National Insurance numbers, by the issuing format's letter sets.
#[must_use]
pub fn find_uk_ninos(text: &str) -> Vec<(usize, usize)> {
    scan(&NINO, text, |start, caps| {
        group_span(caps, 1).filter(|&(from, to)| {
            not_ascii_alnum(char_before(text, start))
                && text
                    .get(from..from + 2)
                    .is_some_and(|prefix| !NINO_EXCLUDED.contains(&prefix))
                && not_ascii_alnum(char_at(text, to))
        })
    })
}

/// The name, an optional separator and the number of a Canadian SIN, grouped 3-3-3 with one
/// separator or bare (`docs/patterns.md`, P27; the backreference is its three spellings). The
/// long names are case-insensitive, and there `i` also reads U+0130 and U+0131.
static CA_SIN: Pattern = LazyLock::new(|| {
    let (d, s) = (DIGIT_CLASS.as_str(), SPACE_CLASS.as_str());
    let i = r"[i\u{130}\u{131}]";
    compile(&format!(
        "(?:SIN|NAS|(?i:soc{i}al{s}+{i}nsurance|num[\u{e9}e]ro{s}+d['\u{2019}]assurance{s}+soc{i}ale))\
         (?:{s}+(?i:number|no\\.?|#))?{s}*(?:[:#]{s}*)?\
         ([1-79]{d}{{2}}(?:{d}{{3}}{d}{{3}}| {d}{{3}} {d}{{3}}|-{d}{{3}}-{d}{{3}}))"
    ))
});

/// Canadian SINs: always beside their name, grouped 3-3-3 or bare, and Luhn-valid. A grouped
/// 9-digit number is also French thousands or an order id, and passes Luhn one time in ten.
#[must_use]
pub fn find_ca_sins(text: &str) -> Vec<(usize, usize)> {
    let candidates = scan(&CA_SIN, text, |start, caps| {
        group_span(caps, 1).filter(|&(_, to)| {
            !char_before(text, start).is_some_and(|c| c.is_ascii_alphabetic())
                && not_digit_or_hyphen(char_at(text, to))
        })
    });
    candidates
        .into_iter()
        .filter(|&(from, to)| {
            text.get(from..to)
                .is_some_and(|number| luhn_ok(&digits_of(number)))
        })
        .collect()
}

/// Verhoeff's multiplication table.
const VERHOEFF_D: [[usize; 10]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    [1, 2, 3, 4, 0, 6, 7, 8, 9, 5],
    [2, 3, 4, 0, 1, 7, 8, 9, 5, 6],
    [3, 4, 0, 1, 2, 8, 9, 5, 6, 7],
    [4, 0, 1, 2, 3, 9, 5, 6, 7, 8],
    [5, 9, 8, 7, 6, 0, 4, 3, 2, 1],
    [6, 5, 9, 8, 7, 1, 0, 4, 3, 2],
    [7, 6, 5, 9, 8, 2, 1, 0, 4, 3],
    [8, 7, 6, 5, 9, 3, 2, 1, 0, 4],
    [9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
];

/// Verhoeff's permutation table, by position modulo 8.
const VERHOEFF_P: [[usize; 10]; 8] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    [1, 5, 7, 6, 2, 8, 3, 0, 9, 4],
    [5, 8, 0, 3, 7, 9, 6, 1, 4, 2],
    [8, 9, 1, 6, 0, 4, 3, 5, 2, 7],
    [9, 4, 5, 3, 1, 2, 6, 8, 7, 0],
    [4, 2, 8, 6, 5, 7, 3, 9, 0, 1],
    [2, 7, 9, 3, 8, 0, 6, 4, 1, 5],
    [7, 0, 4, 6, 9, 1, 3, 2, 5, 8],
];

/// Verhoeff's dihedral check over a digit string, as Aadhaar numbers carry: digits of any
/// script, read by their value.
#[must_use]
#[allow(clippy::indexing_slicing)] // `position % 8 < 8`, and a digit value, a permuted digit and `check` are below 10
pub fn verhoeff_ok(digits: &str) -> bool {
    let check = digits
        .chars()
        .rev()
        .enumerate()
        .fold(0, |check, (position, c)| {
            VERHOEFF_D[check][VERHOEFF_P[position % 8][digit_value(c) as usize]]
        });
    check == 0
}

/// An Aadhaar number, grouped 4-4-4 with one separator (group 1) or bare beside its name
/// (group 2) (`docs/patterns.md`, P28; the backreference is its two spellings).
static AADHAAR: Pattern = LazyLock::new(|| {
    let (d, s) = (DIGIT_CLASS.as_str(), SPACE_CLASS.as_str());
    compile(&format!(
        "([2-9]{d}{{3}}(?: {d}{{4}} {d}{{4}}|-{d}{{4}}-{d}{{4}}))\
         |(?i:aadhaa?r(?:{s}+(?:no\\.?|number))?){s}*(?:[:#]{s}*)?([2-9]{d}{{11}})"
    ))
});

/// Indian Aadhaar numbers: Verhoeff-valid, grouped 4-4-4 with one separator, or bare beside the
/// name; never starting with 0 or 1. The grouped form refuses a digit or hyphen on either side,
/// the bare form a letter before the name and a digit after the number.
#[must_use]
pub fn find_in_aadhaar(text: &str) -> Vec<(usize, usize)> {
    let candidates = scan(&AADHAAR, text, |start, caps| {
        let before = char_before(text, start);
        group_span(caps, 1)
            .filter(|&(_, to)| {
                not_digit_or_hyphen(before) && not_digit_or_hyphen(char_at(text, to))
            })
            .or_else(|| {
                group_span(caps, 2).filter(|&(_, to)| {
                    !before.is_some_and(|c| c.is_ascii_alphabetic())
                        && !char_at(text, to).is_some_and(is_py_digit)
                })
            })
    });
    candidates
        .into_iter()
        .filter(|&(from, to)| {
            text.get(from..to)
                .is_some_and(|number| verhoeff_ok(&digits_of(number)))
        })
        .collect()
}

/// `[A-Z]{3}`, a holder type, a letter, four digits and a letter (`docs/patterns.md`, P29).
static PAN: Pattern = LazyLock::new(|| {
    let d = DIGIT_CLASS.as_str();
    compile(&format!("([A-Z]{{3}}[ABCFGHJLPT][A-Z]{d}{{4}}[A-Z])"))
});

/// Indian PANs: the fourth letter is the holder type (P person, C company, H HUF, F firm, ...).
#[must_use]
pub fn find_in_pans(text: &str) -> Vec<(usize, usize)> {
    scan(&PAN, text, |start, caps| {
        group_span(caps, 1).filter(|&(_, to)| {
            not_ascii_alnum(char_before(text, start)) && not_ascii_alnum(char_at(text, to))
        })
    })
}
