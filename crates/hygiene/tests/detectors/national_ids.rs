//! `find_national_ids` (US SSN), `find_uk_ninos` and `find_in_pans`.
use crate::reference::{find_in_pans, find_national_ids, find_uk_ninos};

use super::{assert_finds, assert_misses};

/// The Unicode fullwidth form of ASCII digits `text`.
fn wide(text: &str) -> String {
    text.chars()
        .map(|c| {
            c.to_digit(10)
                .map_or(c, |d| char::from_u32(0xFF10 + d).unwrap())
        })
        .collect()
}

#[test]
fn an_ssn_is_found_exactly() {
    assert_finds(
        find_national_ids,
        "dashes",
        "SSN {} on record",
        "123-45-6789",
    );
    assert_finds(find_national_ids, "dashes", "ssn: {}", "078-05-1120");
    assert_finds(find_national_ids, "start and end", "{}", "123-45-6789");
}

#[test]
fn an_ssn_with_an_unissued_area_group_or_serial_is_not_one() {
    for (name, text) in [
        ("area 000", "invalid ssn 000-12-3456"),
        ("area 666", "666-12-3456"),
        ("area 900", "900-12-3456"),
        ("area 999", "999-12-3456"),
        ("group 00", "123-00-4567"),
        ("serial 0000", "123-45-0000"),
        ("undashed", "ssn 123456789 unformatted"),
        ("one dash", "123-456789"),
        ("short serial", "123-45-678"),
    ] {
        assert_misses(find_national_ids, name, text);
    }
}

#[test]
fn the_edges_of_the_unissued_ranges_are_issued() {
    for (name, value) in [
        ("area 001", "001-12-3456"),
        ("area 665", "665-12-3456"),
        ("area 667", "667-12-3456"),
        ("area 899", "899-12-3456"),
        ("group 01", "123-01-4567"),
        ("group 10", "123-10-4567"),
        ("serial 0001", "123-45-0001"),
        ("serial 1000", "123-45-1000"),
        ("area 100", "100-12-3456"),
        ("area 006", "006-12-3456"),
        ("area 060", "060-12-3456"),
    ] {
        assert_finds(find_national_ids, name, "x {} y", value);
    }
}

#[test]
fn the_rules_compare_ascii_characters_so_another_zero_passes() {
    // `000`, `00` and `0000` are the ASCII text, as in the reference's lookaheads.
    assert_finds(find_national_ids, "area", "{}", &wide("000-12-3456"));
    assert_finds(find_national_ids, "group", "{}", &wide("123-00-4567"));
    assert_finds(find_national_ids, "serial", "{}", &wide("123-45-0000"));
    assert_finds(find_national_ids, "nine", "{}", &wide("923-45-6789"));
    assert_finds(find_national_ids, "666", "{}", &wide("666-45-6789"));
}

#[test]
fn a_digit_or_hyphen_beside_an_ssn_hides_it() {
    for (name, text) in [
        ("digit before", "1123-45-6789"),
        ("hyphen before", "-123-45-6789"),
        ("digit after", "123-45-67890"),
        ("hyphen after", "123-45-6789-"),
        ("wide digit after", "123-45-6789\u{ff11}"),
    ] {
        assert_misses(find_national_ids, name, text);
    }
    assert_finds(find_national_ids, "letters", "a{}b", "123-45-6789");
}

#[test]
fn a_hidden_start_is_retried_one_character_later() {
    let text = "1\u{ff11}23-45-6789 123-45-6789";
    assert_eq!(find_national_ids(text), vec![(15, 26)]);
}

#[test]
fn several_ssns_are_found_in_order() {
    assert_eq!(
        find_national_ids("123-45-6789,078-05-1120"),
        vec![(0, 11), (12, 23)]
    );
}

#[test]
fn a_nino_is_found_exactly() {
    assert_finds(find_uk_ninos, "compact", "NI {} on file", "AB123456C");
    assert_finds(find_uk_ninos, "spaced", "NI {} on file", "JG 10 37 64 A");
    assert_finds(find_uk_ninos, "some spaces", "NI {} on file", "AB 123456 C");
    assert_finds(find_uk_ninos, "suffix d", "{}", "AB123456D");
    assert_finds(
        find_uk_ninos,
        "wide digits",
        "{}",
        &format!("AB{}C", wide("123456")),
    );
}

#[test]
fn the_reference_nino_near_misses_are_not_found() {
    for (name, text) in [
        ("q first", "QQ123456C"),
        ("excluded prefix", "GB123456A"),
        ("suffix past d", "AB123456E"),
        ("o second", "AO123456C"),
        ("lowercase and long", "ab123456c AB1234567C"),
    ] {
        assert_misses(find_uk_ninos, name, text);
    }
}

#[test]
fn every_letter_of_a_nino_is_judged_by_its_own_set() {
    for letter in 'A'..='Z' {
        let first = !"DFIQUV".contains(letter);
        let second = !"DFIOQUV".contains(letter);
        // `A` before it is fine for any second letter; `A` after it is fine for any first.
        let as_first = format!("{letter}A123456C");
        let as_second = format!("A{letter}123456C");
        assert_eq!(
            find_uk_ninos(&as_first).len(),
            usize::from(first),
            "first {letter}"
        );
        assert_eq!(
            find_uk_ninos(&as_second).len(),
            usize::from(second),
            "second {letter}"
        );
    }
}

#[test]
fn every_excluded_prefix_is_refused_and_its_neighbours_are_not() {
    for prefix in ["BG", "GB", "KN", "NK", "NT", "TN", "ZZ"] {
        assert_misses(find_uk_ninos, prefix, &format!("{prefix}123456C"));
    }
    for prefix in ["GN", "NG", "KK", "NN", "BN", "TT", "ZY", "YZ", "AB"] {
        assert_finds(find_uk_ninos, prefix, "{}", &format!("{prefix}123456C"));
    }
}

#[test]
fn a_letter_or_digit_beside_a_nino_hides_it() {
    assert_misses(find_uk_ninos, "letter before", "xAB123456C");
    assert_misses(find_uk_ninos, "digit before", "1AB123456C");
    assert_misses(find_uk_ninos, "letter after", "AB123456Cx");
    assert_misses(find_uk_ninos, "digit after", "AB123456C1");
    assert_finds(find_uk_ninos, "punctuation", "({}).", "AB123456C");
}

#[test]
fn a_hidden_nino_is_retried_one_character_later() {
    let text = "xAB123456C AB123456C";
    assert_eq!(find_uk_ninos(text), vec![(11, 20)]);
}

#[test]
fn a_pan_is_found_exactly() {
    assert_finds(find_in_pans, "reference", "PAN {} filed", "ABCPE1234F");
    assert_finds(
        find_in_pans,
        "wide digits",
        "{}",
        &format!("ABCPE{}F", wide("1234")),
    );
}

#[test]
fn the_fourth_letter_of_a_pan_is_the_holder_type() {
    for letter in 'A'..='Z' {
        let holder = "ABCFGHJLPT".contains(letter);
        let text = format!("ABC{letter}E1234F");
        assert_eq!(find_in_pans(&text).len(), usize::from(holder), "{letter}");
    }
}

#[test]
fn the_reference_pan_near_misses_are_not_found() {
    assert_misses(find_in_pans, "d is no holder", "ABCDE1234F");
    assert_misses(find_in_pans, "short and lowercase", "ABCPE12345 abcpe1234f");
    assert_misses(find_in_pans, "letter before", "xABCPE1234F");
    assert_misses(find_in_pans, "digit after", "ABCPE1234F1");
    assert_misses(find_in_pans, "five digits", "ABCPE12345F");
}

#[test]
fn a_hidden_pan_is_retried_one_character_later() {
    assert_eq!(find_in_pans("9ABCPE1234F ABCPE1234F"), vec![(12, 22)]);
}

#[test]
fn an_empty_text_has_no_national_ids() {
    assert_misses(find_national_ids, "ssn", "");
    assert_misses(find_uk_ninos, "nino", "");
    assert_misses(find_in_pans, "pan", "");
}
