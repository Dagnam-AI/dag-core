//! `find_ibans` and `iban_mod97_ok`.
use dagnam_hygiene::{find_ibans, iban_mod97_ok};

use super::{assert_finds, assert_misses};

/// The registry as the reference lists it: country code and total length.
const REGISTRY: &str = "AD24 AE23 AL28 AT20 AZ28 BA20 BE16 BG22 BH22 BI27 BR29 BY28 CH21 CR22 CY28 CZ24 \
DE22 DJ27 DK18 DO28 EE20 EG29 ES24 FI18 FK18 FO18 FR27 GB22 GE22 GI23 GL18 GR27 \
GT28 HR21 HU28 IE22 IL23 IQ23 IS26 IT27 JO30 KW30 KZ20 LB28 LC32 LI21 LT20 LU20 \
LV21 LY25 MC27 MD24 ME22 MK19 MN20 MR27 MT31 MU30 NI28 NL18 NO15 OM23 PK24 PL28 \
PS29 PT25 QA29 RO24 RS22 RU33 SA24 SC31 SD18 SE24 SI19 SK24 SM27 SO23 ST25 SV28 \
TL23 TN24 TR26 UA29 VA22 VG24 XK20 YE30";

/// An IBAN of `country` and total `length` whose check digits come from the arithmetic:
/// 98 minus the remainder mod 97 of the basic number with `00` as the check digits.
fn make(country: &str, length: usize) -> String {
    let basic: String = "A1B2C3D4E5F6G7H8J9K0L1M2N3P4Q5R6S7T8"
        .chars()
        .take(length - 4)
        .collect();
    let digits: Vec<u32> = format!("{basic}{country}00")
        .chars()
        .flat_map(|c| {
            let v = c.to_digit(36).unwrap();
            if v < 10 {
                vec![v]
            } else {
                vec![v / 10, v % 10]
            }
        })
        .collect();
    let remainder = digits.iter().fold(0, |r, d| (r * 10 + d) % 97);
    format!("{country}{:02}{basic}", 98 - remainder)
}

/// `iban` in groups of four, as it is printed.
fn spaced(iban: &str) -> String {
    iban.as_bytes()
        .chunks(4)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join(" ")
}

fn registry() -> Vec<(&'static str, usize)> {
    REGISTRY
        .split_whitespace()
        .map(|entry| (&entry[..2], entry[2..].parse().unwrap()))
        .collect()
}

#[test]
fn the_registry_holds_eighty_eight_countries() {
    assert_eq!(registry().len(), 88);
}

#[test]
fn every_registered_country_is_found_at_its_length_compact_and_spaced() {
    for (country, length) in registry() {
        let iban = make(country, length);
        assert_eq!(iban.len(), length, "{country}");
        assert!(iban_mod97_ok(&iban), "{country} generated");
        assert_finds(find_ibans, country, "to {} now", &iban);
        assert_finds(find_ibans, country, "to {} now", &spaced(&iban));
    }
}

#[test]
fn every_registered_country_refuses_one_character_more_or_less() {
    for (country, length) in registry() {
        let longer = make(country, length + 1);
        assert_misses(find_ibans, &format!("{country} longer"), &longer);
        if length > 15 {
            let shorter = make(country, length - 1);
            assert_misses(find_ibans, &format!("{country} shorter"), &shorter);
        }
    }
}

#[test]
fn the_reference_examples_are_found_exactly() {
    assert_finds(
        find_ibans,
        "de",
        "pay {} today",
        "DE89 3704 0044 0532 0130 00",
    );
    assert_finds(find_ibans, "gb", "pay {} today", "GB29NWBK60161331926819");
    assert_finds(
        find_ibans,
        "fr",
        "iban: {}.",
        "FR14 2004 1010 0505 0001 3M02 606",
    );
    assert_finds(find_ibans, "be", "to {} NOW", "BE68 5390 0754 7034");
    assert_finds(find_ibans, "nl", "({})", "NL91ABNA0417164300");
}

#[test]
fn the_reference_near_misses_are_not_found() {
    assert_misses(
        find_ibans,
        "check digits off",
        "DE89 3704 0044 0532 0130 01",
    );
    assert_misses(find_ibans, "no such country", "XX89 3704 0044 0532 0130 00");
    assert_misses(find_ibans, "one short", "GB29NWBK6016133192681");
    assert_misses(find_ibans, "not an iban", "ref AB12CD34EF56GH78 attached");
    assert_misses(find_ibans, "lowercase", "de89 3704 0044 0532 0130 00");
}

#[test]
fn a_character_glued_after_the_last_one_hides_it() {
    assert_misses(find_ibans, "glued letter", "DE89 3704 0044 0532 0130 00X");
    assert_misses(find_ibans, "before", "xDE89 3704 0044 0532 0130 00");
    assert_misses(find_ibans, "hyphen glued", "DE89 3704 0044 0532 0130 00X-");
}

#[test]
fn the_registered_length_trims_a_candidate_that_ran_on_into_a_word() {
    let (text, _) = super::fill("{} AND MORE", "NL91ABNA0417164300");
    assert_eq!(find_ibans(&text), vec![(0, 18)]);
    let (text, _) = super::fill("{} 12", "NL91ABNA0417164300");
    assert_eq!(find_ibans(&text), vec![(0, 18)]);
}

#[test]
fn a_shorter_end_is_taken_where_the_longest_meets_a_letter() {
    // The optional last group grows to `ABCD` and meets an `e`; each shorter end meets a
    // letter too, until the candidate ends after the number itself, followed by a space.
    let (text, _) = super::fill("{} ABCDe", "NL91ABNA0417164300");
    assert_eq!(find_ibans(&text), vec![(0, 18)]);
}

#[test]
fn check_digits_in_another_script_are_read_by_their_value() {
    // The pattern's `\d` takes them; Python's `int` reads them as 8 and 9.
    let (text, _) = super::fill("[{}]", "DE\u{ff18}\u{ff19}370400440532013000");
    assert_eq!(find_ibans(&text), vec![(1, 1 + 22 + 4)]);
    let (text, _) = super::fill("[{}]", "DE\u{ff18}\u{ff18}370400440532013000");
    assert_eq!(find_ibans(&text), vec![]);
}

#[test]
fn several_ibans_are_found_in_order() {
    let text = "NL91ABNA0417164300, DE89370400440532013000; BE68539007547034";
    assert_eq!(find_ibans(text), vec![(0, 18), (20, 42), (44, 60)]);
}

#[test]
fn mod_97_accepts_a_valid_number_and_refuses_it_with_one_digit_changed() {
    assert!(iban_mod97_ok("DE89370400440532013000"));
    assert!(iban_mod97_ok("GB29NWBK60161331926819"));
    assert!(!iban_mod97_ok("DE89370400440532013001"));
    assert!(!iban_mod97_ok("DE88370400440532013000"));
    assert!(!iban_mod97_ok("GB29NWBK60161331926818"));
}

#[test]
fn mod_97_refuses_the_empty_string() {
    assert!(!iban_mod97_ok(""));
}

#[test]
fn an_empty_text_has_no_ibans() {
    assert_misses(find_ibans, "empty", "");
}
