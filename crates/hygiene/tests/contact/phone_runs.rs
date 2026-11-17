//! `find_phones` on runs: a number read on from an accepted one without the lookbehinds.
use crate::reference::find_phones;

use super::{assert_none, assert_reads};

/// The numbers `n` of `+1415555xxxx`, distinct and synthetic.
fn plus_numbers(n: usize) -> Vec<String> {
    (0..n)
        .map(|i| format!("+1415555{}", 1000 + i % 9000))
        .collect()
}

#[test]
fn numbers_one_space_or_hyphen_apart_are_each_found() {
    let numbers = plus_numbers(5);
    let needles: Vec<&str> = numbers.iter().map(String::as_str).collect();
    for separator in [" ", "-"] {
        let text = needles.join(separator);
        assert_reads(find_phones, &format!("{separator:?}"), &text, &needles);
    }
}

#[test]
fn other_separators_never_needed_the_chain() {
    let numbers = plus_numbers(5);
    let needles: Vec<&str> = numbers.iter().map(String::as_str).collect();
    for separator in [", ", "\n", "\t", " - "] {
        let text = needles.join(separator);
        assert_reads(find_phones, &format!("{separator:?}"), &text, &needles);
    }
}

#[test]
fn a_chain_of_any_length_is_one_finding_per_number() {
    for length in [2, 3, 5, 10, 17, 40, 1000] {
        let numbers = plus_numbers(length);
        let text = numbers.join(" ");
        let found = find_phones(&text);
        assert_eq!(found.len(), length, "chain of {length}");
        assert!(
            found.iter().all(|&(s, e)| e - s == 12),
            "each of {length} is 12 bytes"
        );
    }
}

#[test]
fn the_run_ends_where_a_number_stops_being_one() {
    assert_reads(
        find_phones,
        "ends at words",
        "+14155551000 +14155551001 and 12",
        &["+14155551000", "+14155551001"],
    );
    // Two numbers read as one run of digits too long to be a number are neither.
    assert_none(find_phones, "too long", "+14155551000 4155551001");
}

#[test]
fn a_hyphen_before_a_plus_is_between_numbers_not_part_of_one() {
    assert_reads(
        find_phones,
        "hyphen",
        "+1415555100-+1415555101",
        &["+1415555100", "+1415555101"],
    );
    assert_reads(
        find_phones,
        "space hyphen",
        "+14155551000 -+14155551001",
        &["+14155551000", "+14155551001"],
    );
    assert_reads(
        find_phones,
        "two spaces",
        "+14155551000  +14155551001",
        &["+14155551000", "+14155551001"],
    );
}

#[test]
fn a_number_after_an_accepted_one_may_have_no_plus() {
    assert_reads(
        find_phones,
        "with plus first",
        "+1 415-555-0132 +1 415-555-0133",
        &["+1 415-555-0132", "+1 415-555-0133"],
    );
    // Without a plus on the first, the second is behind a digit and a space: neither is a number.
    assert_none(find_phones, "no plus", "415-555-0132 415-555-0133");
}

#[test]
fn the_chain_stops_before_the_number_the_pattern_finds_next() {
    // The body of a number takes digits, spaces and parentheses: left alone the chain from the
    // second number would take the area code of the parenthesised one and strand the rest.
    let text = "+44 20 7946 0958 +44 20 7946 0958 (415) 555-0132";
    assert_reads(
        find_phones,
        "three",
        text,
        &["+44 20 7946 0958", "+44 20 7946 0958", "415) 555-0132"],
    );
    let text = "+44 20 7946 0958 +44 20 7946 0958\n(415) 555-0132";
    assert_eq!(find_phones(text).len(), 3, "three, a newline between");
}

#[test]
fn nothing_is_chained_from_a_number_the_filter_refused() {
    // A nine-digit id is not accepted, so no link starts from it; and the number after it is
    // behind "digit, space", which the pattern's lookbehind refuses.
    assert_none(find_phones, "refused first", "078-05-1120 +14155551001x");
}
