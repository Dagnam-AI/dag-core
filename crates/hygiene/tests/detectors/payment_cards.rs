//! `find_payment_cards` and `luhn_ok`.
use dagnam_hygiene::{find_payment_cards, luhn_ok};

use super::{assert_finds, assert_misses};

/// Luhn-valid test numbers the reference's own tests use.
const VALID: [&str; 3] = ["4111111111111111", "378282246310005", "5555555555554444"];

/// `prefix` followed by the check digit that makes it Luhn-valid, from the arithmetic.
fn complete(prefix: &str) -> String {
    let sum: u32 = prefix
        .chars()
        .rev()
        .enumerate()
        .map(|(i, c)| {
            let d = c.to_digit(10).unwrap();
            if i % 2 == 0 {
                [0, 2, 4, 6, 8, 1, 3, 5, 7, 9][d as usize]
            } else {
                d
            }
        })
        .sum();
    format!("{prefix}{}", (10 - sum % 10) % 10)
}

#[test]
fn luhn_accepts_a_valid_number_and_refuses_it_with_one_digit_changed() {
    for valid in VALID {
        assert!(luhn_ok(valid), "{valid} as given");
        let mut digits: Vec<char> = valid.chars().collect();
        let last = digits.len() - 1;
        digits[last] = if digits[last] == '9' { '0' } else { '9' };
        assert!(!luhn_ok(&digits.into_iter().collect::<String>()));
    }
    assert!(!luhn_ok("4111111111111112"));
    assert!(!luhn_ok("1234567890123456"));
}

#[test]
fn luhn_accepts_the_empty_string_as_python_does() {
    assert!(luhn_ok(""));
}

#[test]
fn luhn_reads_every_decimal_digit_script_by_its_value() {
    // Python's `int` reads each of these; the values are the position in the run of ten.
    for (name, zero) in [
        ("fullwidth", 0xFF10_u32),
        ("arabic-indic", 0x0660),
        ("math bold", 0x1D7CE),
        ("math double-struck", 0x1D7D8),
        ("math sans-serif", 0x1D7E2),
        ("math sans-serif bold", 0x1D7EC),
        ("math monospace", 0x1D7F6),
        ("nko", 0x07C0),
    ] {
        for number in VALID {
            let shifted: String = number
                .chars()
                .map(|c| char::from_u32(zero + c.to_digit(10).unwrap()).unwrap())
                .collect();
            assert!(luhn_ok(&shifted), "{name}");
        }
    }
}

#[test]
fn a_plain_number_is_a_card() {
    for (index, value) in VALID.iter().enumerate() {
        assert_finds(
            find_payment_cards,
            &format!("plain {index}"),
            "pay {} now",
            value,
        );
    }
}

#[test]
fn a_number_grouped_by_one_space_or_hyphen_is_a_card() {
    assert_finds(
        find_payment_cards,
        "spaces",
        "card {}.",
        "4111 1111 1111 1111",
    );
    assert_finds(
        find_payment_cards,
        "hyphens",
        "card {}.",
        "4111-1111-1111-1111",
    );
    assert_finds(find_payment_cards, "amex", "card {}.", "3782 822463 10005");
    assert_finds(
        find_payment_cards,
        "mixed",
        "card {}.",
        "5555 5555-5555 4444",
    );
}

#[test]
fn a_number_in_another_script_is_a_card_with_byte_offsets() {
    let value: String = "4111111111111111"
        .chars()
        .map(|c| char::from_u32(0xFF10 + c.to_digit(10).unwrap()).unwrap())
        .collect();
    assert_eq!(value.len(), 48);
    assert_finds(find_payment_cards, "fullwidth", "x {} y", &value);
}

#[test]
fn a_number_that_fails_luhn_is_not_a_card() {
    assert_misses(find_payment_cards, "off by one", "4111111111111112");
    assert_misses(find_payment_cards, "sequence", "1234567890123456");
}

#[test]
fn only_thirteen_to_nineteen_digits_can_be_a_card() {
    // Luhn-valid at the edges: 13 digits and 19 digits, and one short and one long.
    for (length, found) in [(12, false), (13, true), (19, true), (20, false)] {
        let number = complete(&"4".repeat(length - 1));
        assert_eq!(number.len(), length);
        assert!(luhn_ok(&number));
        assert_eq!(
            find_payment_cards(&number),
            if found { vec![(0, length)] } else { vec![] },
            "{length} digits"
        );
    }
}

#[test]
fn a_digit_or_hyphen_beside_the_number_hides_it() {
    assert_misses(find_payment_cards, "digit before", "94111111111111111");
    assert_misses(find_payment_cards, "digit after", "41111111111111111");
    assert_misses(find_payment_cards, "hyphen before", "-4111111111111111");
    assert_misses(find_payment_cards, "hyphen after", "4111111111111111-");
}

#[test]
fn other_characters_beside_the_number_do_not() {
    assert_finds(find_payment_cards, "letters", "x{}y", "4111111111111111");
    assert_finds(find_payment_cards, "text end", "{}", "4111111111111111");
    assert_finds(find_payment_cards, "newline", "{}\n", "4111111111111111");
}

#[test]
fn a_longer_run_gives_way_to_the_shorter_end_the_guard_accepts() {
    // A 17th digit joined by a space is cut at 16 where it meets a hyphen.
    let (text, _) = super::fill("{} 1-", "4111 1111 1111 1111");
    assert_eq!(find_payment_cards(&text), vec![(0, 19)]);
}

#[test]
fn a_run_longer_than_a_card_is_not_cut_into_cards() {
    assert_misses(find_payment_cards, "run", &"4111111111111111".repeat(3));
}

#[test]
fn a_hidden_start_is_retried_one_character_later() {
    // The first start (a fullwidth digit, three bytes) sits after a hyphen and is refused; the
    // search goes on one character later and finds the number past the space.
    let text = "-\u{ff14} 4111111111111111";
    assert_eq!(find_payment_cards(text), vec![(5, 21)]);
}

#[test]
fn several_cards_are_found_in_order() {
    let text = "4111111111111111, 378282246310005 and 5555555555554444";
    assert_eq!(find_payment_cards(text), vec![(0, 16), (18, 33), (38, 54)]);
}

#[test]
fn an_empty_text_has_no_cards() {
    assert_misses(find_payment_cards, "empty", "");
}
