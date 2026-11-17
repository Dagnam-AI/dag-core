//! `find_phones`: one number at a time, the pattern's guards and the filters on top of it.
use crate::reference::find_phones;

use super::{assert_none, assert_reads};

/// Every format the pattern accepts, one number each.
const FORMATS: [&str; 13] = [
    "+44 20 7946 0958",
    "(415) 555-0132",
    "415.555.0132",
    "+1 415-555-0132",
    "+14155550132",
    "+91 98765 43210",
    "415 555 0132",
    "(415)555-0132",
    "+49 30 901820",
    "+33 1 42 68 53 00",
    "1-800-555-0199",
    "+81-3-1234-5678",
    "+1 (415) 555-0132",
];

#[test]
fn every_format_is_one_number_inside_a_sentence() {
    for (index, number) in FORMATS.iter().enumerate() {
        let text = format!("call {number} now");
        // An opening parenthesis is not the start of a number: the pattern starts at a digit.
        let found = number.trim_start_matches('(');
        assert_reads(find_phones, &format!("format {index}"), &text, &[found]);
    }
}

#[test]
fn a_number_on_its_own_is_the_whole_text() {
    assert_eq!(find_phones("+14155550132"), vec![(0, 12)]);
    assert_eq!(find_phones("415.555.0132"), vec![(0, 12)]);
}

#[test]
fn a_nine_digit_dashed_id_and_a_short_number_are_not_numbers() {
    for (index, text) in [
        "ssn 078-05-1120",
        "ext 555-0132",
        "030 901 820",
        "9 digits 415 555 013 only",
    ]
    .into_iter()
    .enumerate()
    {
        assert_none(find_phones, &format!("too short {index}"), text);
    }
}

#[test]
fn the_digit_count_floor_is_nine_with_a_plus_and_ten_without() {
    assert_reads(find_phones, "plus, 9", "+49 30 90182", &["+49 30 90182"]);
    assert_none(find_phones, "plus, 8", "+49 30 9018");
    assert_reads(find_phones, "none, 10", "030 901 8200", &["030 901 8200"]);
    assert_none(find_phones, "none, 9", "030 901 820");
}

#[test]
fn the_digit_count_ceiling_is_fifteen() {
    assert_reads(
        find_phones,
        "15",
        "+1 234 567 890 123 45",
        &["+1 234 567 890 123 45"],
    );
    assert_none(find_phones, "16", "+1 234 567 890 123 456");
    assert_reads(
        find_phones,
        "15, no plus",
        "1 234 567 890 123 45",
        &["1 234 567 890 123 45"],
    );
}

#[test]
fn a_bare_run_of_digits_needs_a_plus() {
    assert_none(find_phones, "order id", "order 4155550132 shipped");
    assert_reads(
        find_phones,
        "with plus",
        "tel +4155550132 now",
        &["+4155550132"],
    );
}

#[test]
fn any_python_space_or_the_punctuation_counts_as_a_separator() {
    for (index, text) in [
        "4155550132\u{a0}4",
        "4155550132\t4",
        "4155550132\u{2003}4",
        "415.5550132",
        "415-5550132",
        "415)5550132",
    ]
    .into_iter()
    .enumerate()
    {
        assert_reads(find_phones, &format!("separator {index}"), text, &[text]);
    }
}

#[test]
fn digits_of_other_scripts_count_as_digits() {
    let arabic = "\u{664}\u{661}\u{665}-\u{665}\u{665}\u{665}-\u{660}\u{661}\u{663}\u{662}";
    assert_reads(find_phones, "arabic-indic", arabic, &[arabic]);
    let wide = "\u{ff14}\u{ff11}\u{ff15} \u{ff15}\u{ff15}\u{ff15} \u{ff10}\u{ff11}\u{ff13}\u{ff12}";
    assert_reads(find_phones, "fullwidth", wide, &[wide]);
    // Superscripts are numbers, not decimal digits: they are neither a digit nor a separator.
    assert_none(find_phones, "superscript", "415 555 013\u{b2}");
}

#[test]
fn a_number_does_not_start_after_a_digit_or_a_plus() {
    assert_none(find_phones, "double plus", "++14155550132");
    // The number starting one digit earlier is the finding, not the shorter one inside it.
    assert_reads(
        find_phones,
        "digit before",
        "x1 415.555.0132-",
        &["1 415.555.0132"],
    );
    assert_reads(
        find_phones,
        "letter before",
        "a415.555.0132",
        &["415.555.0132"],
    );
    assert_reads(
        find_phones,
        "ten digits",
        "1415.555.0132",
        &["1415.555.0132"],
    );
}

#[test]
fn a_number_inside_a_longer_run_of_digits_is_not_a_number() {
    assert_none(find_phones, "card", "4111 1111 1111 1111");
    assert_none(find_phones, "23 digits", "12345678901234567890123");
    assert_none(find_phones, "run after", "+14155550132 4155550132");
}

#[test]
fn a_hyphen_or_space_then_a_digit_after_the_end_extends_it_and_nothing_else_does() {
    assert_reads(
        find_phones,
        "trailing hyphen",
        "415.555.0132-",
        &["415.555.0132"],
    );
    assert_reads(
        find_phones,
        "trailing hyphen, space",
        "415.555.0132 -",
        &["415.555.0132"],
    );
    assert_reads(
        find_phones,
        "trailing letter",
        "415.555.0132 x",
        &["415.555.0132"],
    );
    // Only a space or hyphen followed by a digit refuses an end: a tab is part of the number.
    assert_reads(
        find_phones,
        "tab digit",
        "415.555.0132\t5",
        &["415.555.0132\t5"],
    );
    assert_reads(
        find_phones,
        "hyphen digit",
        "415-555-0132-5",
        &["415-555-0132-5"],
    );
    assert_reads(
        find_phones,
        "space digit",
        "415 555 0132 9",
        &["415 555 0132 9"],
    );
}

#[test]
fn a_shorter_end_is_taken_where_the_longest_is_followed_by_a_digit() {
    // 23 characters: the longest read ends at the last but one digit, which the last digit
    // follows; the read before the final run of digits is the finding, as Python's
    // backtracking gives it.
    assert_reads(
        find_phones,
        "shorter end",
        "9-5 -29043.109.11308639",
        &["9-5 -29043.109"],
    );
    assert_reads(
        find_phones,
        "shorter end, spaced",
        "4.9 318 731873036-3.204",
        &["4.9 318 731873036-3"],
    );
}

#[test]
fn a_number_between_multibyte_characters_has_byte_offsets() {
    let text = "\u{e9}+14155550132\u{e9}";
    assert_eq!(find_phones(text), vec![(2, 14)]);
}

#[test]
fn no_text_has_no_numbers() {
    assert_none(find_phones, "empty", "");
    assert_none(find_phones, "words", "no numbers in this sentence at all");
}

#[test]
fn a_digit_of_another_script_before_a_hyphen_refuses_the_number_as_an_ascii_one_does() {
    // The lookbehind is "digit, then a space or hyphen", digit meaning Python's `\d`.
    assert_none(find_phones, "arabic-indic digit", "\u{664}-+14155550132");
    assert_none(
        find_phones,
        "arabic-indic digit, space",
        "\u{664} +14155550132",
    );
    assert_reads(find_phones, "letter", "x-+14155550132", &["+14155550132"]);
}

#[test]
fn the_file_and_next_line_controls_are_separators_as_python_reads_them() {
    // U+001C and U+0085 are `\s` to Python (Rust's `char::is_whitespace` has the first not).
    for (index, (text, found)) in [
        ("415\u{1c}555\u{85}0132", "415\u{1c}555\u{85}0132"),
        ("4155550132\u{1c}4", "4155550132\u{1c}4"),
        ("4155550132\u{85}4", "4155550132\u{85}4"),
        ("415.555.0132\u{85}4", "415.555.0132\u{85}4"),
        ("+1415555\u{1c}1000", "+1415555\u{1c}1000"),
    ]
    .into_iter()
    .enumerate()
    {
        assert_reads(find_phones, &format!("control {index}"), text, &[found]);
    }
    // They end a number like any other character that is neither a digit nor a hyphen.
    assert_none(find_phones, "control, no digit after", "4155550132\u{1c}");
    assert_reads(
        find_phones,
        "between numbers",
        "+14155551000\u{1c}+14155551001",
        &["+14155551000", "+14155551001"],
    );
    assert_reads(
        find_phones,
        "between numbers, next line",
        "+14155551000\u{85} +14155551001",
        &["+14155551000", "+14155551001"],
    );
}
