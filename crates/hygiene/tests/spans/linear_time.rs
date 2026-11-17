//! Time: the text `findings` reads must cost time that grows with it, not with its square. Each
//! text here is made at two sizes (`SMALL` and four times that) and the larger must take less than
//! eight times as long as the smaller (a quadratic read would take sixteen).
//!
//! This is the coarse check, on the clock. What the scan reads is pinned by count, on shapes that
//! make it do real work, in `src/spans/cost_tests.rs`: a ratio of two short timings passes for
//! reasons of its own, a count of characters read does not.
//!
//! The texts are made of shapes that put pressure on the span resolver (long runs that every
//! pattern reads, findings glued one to the next so that each hides the one after it) and of the
//! reference's own corpora repeated to size. No file holds a credential in any form: where a
//! credential is part of a text it is a case of `credentials.jsonl` assembled at run time.
use std::time::{Duration, Instant};

use crate::reference::findings;

use crate::{CredentialCase, FindingsCase, cases};

/// The characters of the smaller text: large enough that its run lasts well above `FLOOR`.
const SMALL: usize = 5_000;
/// How many times larger the larger text is.
const FACTOR: usize = 4;
/// The smaller run is taken to last at least this long, so noise in a very fast run is not a ratio.
const FLOOR: Duration = Duration::from_millis(1);
/// Runs of each text; the best counts.
const BEST_OF: usize = 3;

/// `head`, then `unit` repeated to about `size` characters, then `tail`.
fn shape(head: &str, unit: &str, tail: &str, size: usize) -> String {
    let count = size.saturating_sub(head.len() + tail.len()) / unit.len();
    format!("{head}{}{tail}", unit.repeat(count))
}

/// The longest unit `tile` takes: a unit of the size of the text would end the small text and the
/// large one at the same case, and the two would be one text.
const LONGEST_UNIT: usize = SMALL / 8;

/// The units no longer than `LONGEST_UNIT` in turn, one after the other with `between` after each,
/// to about `size` characters.
fn tile(units: &[String], between: &str, size: usize) -> String {
    let mut text = String::with_capacity(size + LONGEST_UNIT);
    for unit in units.iter().filter(|u| u.len() <= LONGEST_UNIT).cycle() {
        if text.len() >= size {
            break;
        }
        text.push_str(unit);
        text.push_str(between);
    }
    text
}

/// The best of `BEST_OF` runs of `findings` over `text`.
fn best_of(text: &str) -> Duration {
    (0..BEST_OF)
        .map(|_| {
            let started = Instant::now();
            let found = findings(text);
            let took = started.elapsed();
            std::hint::black_box(found);
            took
        })
        .min()
        .expect("at least one run")
}

/// Four times the text takes less than eight times as long, and `findings` finds something in the
/// small one exactly when `finds` says so (a shape that drifted into finding nothing would time
/// nothing). The larger text must be at least three times the smaller, so that a text that does not
/// grow with the size asked for cannot pass for linear.
fn assert_linear(name: &str, finds: bool, make: impl Fn(usize) -> String) {
    assert_linear_from(SMALL, name, finds, make);
}

/// `assert_linear` with the characters of the smaller text given.
fn assert_linear_from(size: usize, name: &str, finds: bool, make: impl Fn(usize) -> String) {
    let small = make(size);
    assert_eq!(!findings(&small).is_empty(), finds, "{name}");
    let large = make(size * FACTOR);
    assert!(
        large.chars().count() >= 3 * small.chars().count(),
        "{name}: texts of {} and {} characters are not four times apart",
        small.chars().count(),
        large.chars().count()
    );
    let (a, b) = (best_of(&small), best_of(&large));
    assert!(
        b.as_secs_f64() < 8.0 * a.max(FLOOR).as_secs_f64(),
        "{name}: {} characters took {a:?}, {} took {b:?}",
        small.chars().count(),
        large.chars().count()
    );
}

/// The text of the first corpus case with just one finding, of class `code`: a sample of that
/// class with the words around it that its pattern reads.
fn sample(code: &str) -> String {
    cases::<FindingsCase>("findings")
        .into_iter()
        .find_map(|case| match case.spans.as_slice() {
            [(_, _, c)] if c == code => Some(case.text),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no corpus case holds just one {code}"))
}

#[test]
fn a_long_run_of_one_shape_is_read_in_linear_time() {
    for (name, head, unit, tail, finds) in [
        ("digits", "", "1", "", false),
        ("digits and spaces", "", "1 ", "", false),
        ("digits and hyphens", "", "1-", "", false),
        ("digits and dots", "", "1.", "", true),
        ("digit groups", "", "12 345 ", "", false),
        ("phone-like groups", "", "+1 (415) 555 ", "", false),
        ("a word character", "", "a", "", false),
        ("short words", "", "ab ", "", false),
        ("at signs", "", "a@", "", false),
        ("an at sign between dots", "", ".@.", "", false),
        ("a local part", "", "a.", "@b.co", true),
        ("domain labels", "a@", "b.", "co", true),
        ("a long label", "a@", "b", ".co", true),
        ("dates", "", "1/1/", "", false),
        ("separators", "", "- ", "", false),
        ("colons", "", "a:", "", false),
        ("quotes", "", "a\"", "", false),
        (
            "a keyword and digits",
            "",
            "Aadhaar Number 580927525309",
            "",
            true,
        ),
        ("a keyword and a group", "", "SIN 130 692 544 ", "", true),
    ] {
        assert_linear(name, finds, |size| shape(head, unit, tail, size));
    }
}

#[test]
fn every_class_glued_to_itself_and_to_the_others_is_read_in_linear_time() {
    let samples: Vec<(String, String)> = [
        "PII_CA_SIN",
        "PII_DATE_OF_BIRTH",
        "PII_EMAIL",
        "PII_EU_VAT",
        "PII_IBAN",
        "PII_IN_AADHAAR",
        "PII_IN_PAN",
        "PII_IP_ADDRESS",
        "PII_NATIONAL_ID",
        "PII_PAYMENT_CARD",
        "PII_PHONE",
        "PII_UK_NINO",
    ]
    .map(|code| (code.to_owned(), sample(code)))
    .into();
    for (code, one) in &samples {
        for glue in ["", " "] {
            let unit = format!("{one}{glue}");
            assert_linear(&format!("{code} glued with {glue:?}"), true, |size| {
                shape("", &unit, "", size)
            });
        }
    }
    let all: String = samples.iter().map(|(_, one)| one.as_str()).collect();
    let all_spaced: String = samples
        .iter()
        .flat_map(|(_, one)| [one.as_str(), " "])
        .collect();
    assert_linear("every class glued", true, |size| shape("", &all, "", size));
    assert_linear("every class spaced", true, |size| {
        shape("", &all_spaced, "", size)
    });
}

#[test]
fn the_reference_corpora_repeated_to_size_are_read_in_linear_time() {
    let texts: Vec<String> = cases::<FindingsCase>("findings")
        .into_iter()
        .map(|case| case.text)
        .collect();
    assert_linear("the findings corpus", true, |size| tile(&texts, "\n", size));
    assert_linear("the findings corpus, glued", true, |size| {
        tile(&texts, "", size)
    });
    let credentials: Vec<String> = cases::<CredentialCase>("credentials")
        .iter()
        .map(CredentialCase::assembled)
        .collect();
    assert_linear("the credentials corpus", true, |size| {
        tile(&credentials, "\n", size)
    });
    assert_linear("the credentials corpus, glued", true, |size| {
        tile(&credentials, "", size)
    });
}

#[test]
fn numbers_one_separator_apart_are_read_in_linear_time() {
    // Each number is a finding of its own, so each is read once whole and each is read again where
    // the text after it begins.
    for separator in [" ", "-", "\t", ", ", "\n"] {
        assert_linear(
            &format!("phone numbers apart by {separator:?}"),
            true,
            |size| {
                let count = size / 12;
                let numbers: Vec<String> = (0..count)
                    .map(|i| format!("+1415555{}", 1000 + i % 9000))
                    .collect();
                numbers.join(separator)
            },
        );
    }
}

/// `case` as text, the character in the middle of its one finding repeated until the text has
/// `size` characters: a corpus finding made into a giant token, in memory only.
fn inflated(case: &CredentialCase, size: usize) -> Option<String> {
    let [(start, end, _)] = case.spans.as_slice() else {
        return None;
    };
    let chars: Vec<char> = case.assembled().chars().collect();
    let middle = (start + end) / 2;
    let repeated = *chars.get(middle)?;
    let extra = size.saturating_sub(chars.len());
    let (before, after) = chars.split_at(middle);
    Some(
        before
            .iter()
            .copied()
            .chain(std::iter::repeat_n(repeated, extra))
            .chain(after.iter().copied())
            .collect(),
    )
}

/// The characters of the smaller giant-token text. Not `SMALL`: the `regex` crate's time per
/// character steps up about 5x somewhere between 10,000 and 20,000 characters of haystack (it
/// leaves its bounded backtracker), so a pair of sizes that straddles the step reads as a
/// quadratic. Both sizes here are above it.
const GIANT: usize = 40_000;

#[test]
fn a_corpus_finding_made_into_a_giant_token_is_read_in_linear_time() {
    // Cases of the credentials corpus with one finding of the class that reads values, kept short
    // so the token is most of the text. Only those that are still a finding of that class once
    // inflated are timed, and there must be six of them: nothing is written to a file.
    let kept: Vec<CredentialCase> = cases::<CredentialCase>("credentials")
        .into_iter()
        .filter(|case| {
            matches!(case.spans.as_slice(), [(_, _, code)] if code == "PII_SECRET")
                && case.assembled().chars().count() <= 120
        })
        .filter(|case| {
            inflated(case, GIANT).is_some_and(|text| {
                findings(&text)
                    .iter()
                    .any(|&(_, _, code)| code == "PII_SECRET")
            })
        })
        .take(6)
        .collect();
    assert_eq!(kept.len(), 6);
    let lead = sample("PII_PHONE");
    for (number, case) in kept.iter().enumerate() {
        assert_linear_from(GIANT, &format!("giant token {number}"), true, |size| {
            format!("{lead}{}", inflated(case, size).unwrap_or_default())
        });
    }
}
