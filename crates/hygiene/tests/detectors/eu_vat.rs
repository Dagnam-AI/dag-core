//! `find_eu_vat`.
use dagnam_hygiene::find_eu_vat;

use super::{assert_finds, assert_misses};

/// One valid body of each member state's format, and a body one character short and long.
/// Greece is `EL`. `None` means the format has no such neighbour that is also valid.
struct State {
    prefix: &'static str,
    valid: &'static [&'static str],
    invalid: &'static [&'static str],
}

const STATES: [State; 27] = [
    State {
        prefix: "AT",
        valid: &["U12345678"],
        invalid: &["U1234567", "U123456789", "A12345678", "12345678"],
    },
    State {
        prefix: "BE",
        valid: &["0123456789", "1123456789"],
        invalid: &["2123456789", "012345678", "01234567890"],
    },
    State {
        prefix: "BG",
        valid: &["123456789", "1234567890"],
        invalid: &["12345678", "12345678901"],
    },
    State {
        prefix: "CY",
        valid: &["12345678A"],
        invalid: &["12345678", "1234567A", "123456789A"],
    },
    State {
        prefix: "CZ",
        valid: &["12345678", "123456789", "1234567890"],
        invalid: &["1234567", "12345678901"],
    },
    State {
        prefix: "DE",
        valid: &["123456789"],
        invalid: &["12345678", "1234567890"],
    },
    State {
        prefix: "DK",
        valid: &["12345678"],
        invalid: &["1234567", "123456789"],
    },
    State {
        prefix: "EE",
        valid: &["123456789"],
        invalid: &["12345678", "1234567890"],
    },
    State {
        prefix: "EL",
        valid: &["123456789"],
        invalid: &["12345678", "1234567890"],
    },
    State {
        prefix: "ES",
        valid: &["A1234567B", "12345678A", "X12345678", "A12345670"],
        invalid: &["A123456B", "AB234567B", "A12345678B"],
    },
    State {
        prefix: "FI",
        valid: &["12345678"],
        invalid: &["1234567", "123456789"],
    },
    State {
        prefix: "FR",
        valid: &["12345678901", "AB123456789", "1A123456789", "Z9123456789"],
        invalid: &["IO123456789", "A1123456789I", "1234567890", "AB12345678"],
    },
    State {
        prefix: "HR",
        valid: &["12345678901"],
        invalid: &["1234567890", "123456789012"],
    },
    State {
        prefix: "HU",
        valid: &["12345678"],
        invalid: &["1234567", "123456789"],
    },
    State {
        prefix: "IE",
        valid: &[
            "1234567T",
            "1234567TA",
            "1234567WI",
            "1+23456W",
            "1A23456B",
            "1*23456A",
        ],
        invalid: &["1234567X", "1234567TJ", "1+23456X", "123456T", "1+2345W"],
    },
    State {
        prefix: "IT",
        valid: &["12345678901"],
        invalid: &["1234567890", "123456789012"],
    },
    State {
        prefix: "LT",
        valid: &["123456789", "123456789012"],
        invalid: &["12345678", "1234567890", "12345678901", "1234567890123"],
    },
    State {
        prefix: "LU",
        valid: &["12345678"],
        invalid: &["1234567", "123456789"],
    },
    State {
        prefix: "LV",
        valid: &["12345678901"],
        invalid: &["1234567890", "123456789012"],
    },
    State {
        prefix: "MT",
        valid: &["12345678"],
        invalid: &["1234567", "123456789"],
    },
    State {
        prefix: "NL",
        valid: &["123456789B01", "123456789B99"],
        invalid: &[
            "123456789C01",
            "12345678B01",
            "123456789B1",
            "123456789B012",
        ],
    },
    State {
        prefix: "PL",
        valid: &["1234567890"],
        invalid: &["123456789", "12345678901"],
    },
    State {
        prefix: "PT",
        valid: &["123456789"],
        invalid: &["12345678", "1234567890"],
    },
    State {
        prefix: "RO",
        valid: &["123456", "1234567", "12345678", "123456789", "1234567890"],
        invalid: &["12345", "12345678901", "023456", "0123456789"],
    },
    State {
        prefix: "SE",
        valid: &["123456789001"],
        invalid: &["123456789002", "12345678901", "1234567890011"],
    },
    State {
        prefix: "SI",
        valid: &["12345678"],
        invalid: &["1234567", "123456789"],
    },
    State {
        prefix: "SK",
        valid: &["1234567890"],
        invalid: &["123456789", "12345678901"],
    },
];

#[test]
fn every_state_is_found_by_its_own_format_with_or_without_a_space() {
    for state in &STATES {
        for body in state.valid {
            for sep in ["", " "] {
                let value = format!("{}{sep}{body}", state.prefix);
                assert_finds(find_eu_vat, state.prefix, "VAT {} ok", &value);
                assert_finds(find_eu_vat, state.prefix, "{}", &value);
            }
        }
    }
}

#[test]
fn every_state_refuses_a_body_its_format_does_not_allow() {
    for state in &STATES {
        for body in state.invalid {
            for sep in ["", " "] {
                let text = format!("VAT {}{sep}{body} ok", state.prefix);
                assert_misses(find_eu_vat, state.prefix, &text);
            }
        }
    }
}

#[test]
fn the_reference_examples_are_found_exactly() {
    for (index, value) in [
        "DE123456789",
        "ATU12345678",
        "FR 12345678901",
        "NL123456789B01",
        "ESB12345678",
        "IE1234567T",
        "SE123456789001",
    ]
    .into_iter()
    .enumerate()
    {
        assert_finds(
            find_eu_vat,
            &format!("positive {index}"),
            "VAT {} ok",
            value,
        );
    }
}

#[test]
fn the_reference_near_misses_are_not_found() {
    for (name, text) in [
        ("wrong length for de", "DE12345678 and DE1234567890"),
        ("not a member state", "XY123456789 GB123456789"),
        ("too short for ro", "RO12 is a room"),
        ("lowercase", "de123456789"),
    ] {
        assert_misses(find_eu_vat, name, text);
    }
}

#[test]
fn a_letter_or_digit_around_the_number_hides_it() {
    assert_misses(find_eu_vat, "letter before", "xDE123456789");
    assert_misses(find_eu_vat, "digit before", "1DE123456789");
    assert_misses(find_eu_vat, "letter after", "DE123456789x");
    assert_misses(find_eu_vat, "digit after", "DE1234567890");
    assert_misses(find_eu_vat, "two spaces", "DE  123456789");
    assert_finds(find_eu_vat, "punctuation", "({}).", "DE123456789");
    assert_finds(find_eu_vat, "hyphens", "-{}-", "DE123456789");
}

#[test]
fn the_longer_alternative_is_taken_where_the_shorter_meets_a_digit() {
    // `LT` is `\d{9}|\d{12}`: nine digits meet a tenth, so the twelve-digit form is the match.
    assert_finds(find_eu_vat, "lt", "{}", "LT123456789012");
    // Romania's body takes the whole run of up to ten digits.
    assert_finds(find_eu_vat, "ro", "{}", "RO1234567890");
}

#[test]
fn digits_of_another_script_make_a_number() {
    let wide: String = "123456789"
        .chars()
        .map(|c| char::from_u32(0xFF10 + c.to_digit(10).unwrap()).unwrap())
        .collect();
    assert_finds(find_eu_vat, "wide", "VAT {}", &format!("DE{wide}"));
    // The guard around the number names ASCII letters and digits only.
    assert_eq!(find_eu_vat("DE123456789\u{ff11}"), vec![(0, 11)]);
    assert_eq!(find_eu_vat("\u{ff11}DE123456789"), vec![(3, 14)]);
}

#[test]
fn a_hidden_start_is_retried_one_character_later() {
    assert_eq!(find_eu_vat("xDE123456789 DE123456789"), vec![(13, 24)]);
}

#[test]
fn several_numbers_are_found_in_order() {
    let text = "DE123456789, ATU12345678 and FR 12345678901";
    assert_eq!(find_eu_vat(text), vec![(0, 11), (13, 24), (29, 43)]);
}

#[test]
fn an_empty_text_has_no_numbers() {
    assert_misses(find_eu_vat, "empty", "");
}
