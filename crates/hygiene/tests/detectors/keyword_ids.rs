//! `find_ca_sins`, `find_in_aadhaar` and the checksums `verhoeff_ok` beside `luhn_ok`.
use dagnam_hygiene::{find_ca_sins, find_in_aadhaar, luhn_ok, verhoeff_ok};

use super::{assert_finds, assert_misses};

#[test]
fn verhoeff_takes_a_valid_number_and_refuses_it_with_one_digit_changed() {
    // 236 has check digit 3 (Verhoeff's own example).
    assert!(verhoeff_ok("2363"));
    assert!(!verhoeff_ok("2364"));
    assert!(!verhoeff_ok("2373"));
    assert!(verhoeff_ok("234567890124"));
    assert!(!verhoeff_ok("234567890125"));
}

#[test]
fn verhoeff_takes_the_empty_string_as_python_does() {
    assert!(verhoeff_ok(""));
}

#[test]
fn verhoeff_reads_the_digits_of_any_script_by_their_value() {
    let wide: String = "234567890124"
        .chars()
        .map(|c| char::from_u32(0xFF10 + c.to_digit(10).unwrap()).unwrap())
        .collect();
    assert!(verhoeff_ok(&wide));
}

#[test]
fn a_sin_is_found_exactly() {
    for (index, (template, value)) in [
        ("SIN {} ok", "130 692 544"),
        ("SIN no. {} ok", "527-348-916"),
        ("SIN: {}", "712345677"),
        ("Social Insurance Number {}", "130692544"),
        ("NAS : {}", "527 348 916"),
        ("num\u{e9}ro d'assurance sociale {}", "712 345 677"),
    ]
    .into_iter()
    .enumerate()
    {
        assert_finds(find_ca_sins, &format!("positive {index}"), template, value);
    }
}

#[test]
fn every_name_and_separator_of_a_sin_is_read() {
    for (index, template) in [
        "SIN number {}",
        "SIN NUMBER {}",
        "SIN No {}",
        "SIN no. {}",
        "SIN # {}",
        "SIN#{}",
        "SIN\t:\t\t{}",
        "SIN  :  {}",
        "SIN\n{}",
        "NAS no. : {}",
        "NAS # {}",
        "social insurance {}",
        "SOCIAL   INSURANCE  NO. {}",
        "Social Insurance number: {}",
        "num\u{e9}ro d\u{2019}assurance sociale {}",
        "NUM\u{c9}RO D'ASSURANCE SOCIALE {}",
        "num\u{e9}ro  d'assurance\tsociale # {}",
        // `i` reads the dotted capital and the dotless i, as Python's `(?i)` does.
        "soc\u{130}al \u{131}nsurance {}",
        "SOCIAL INSURANCE {}",
        "social insurance number {}",
        "social insurance no {}",
    ]
    .into_iter()
    .enumerate()
    {
        assert_finds(
            find_ca_sins,
            &format!("name {index}"),
            template,
            "130692544",
        );
    }
}

#[test]
fn the_reference_sin_near_misses_are_not_found() {
    for (name, text) in [
        ("luhn fails", "130 692 545"),
        ("starts with 8", "830 692 548"),
        ("no name", "order 130692544"),
        ("mixed separators", "SIN 130 692-544"),
        ("grouped in a sentence", "Budget: 130 692 544 EUR"),
        ("order id", "Order 527-348-916 shipped"),
        ("a longer number", "total 3 130 692 544"),
    ] {
        assert_misses(find_ca_sins, name, text);
    }
}

#[test]
fn a_sin_never_starts_with_zero_or_eight() {
    assert_misses(find_ca_sins, "zero", "SIN 012345674");
    assert_misses(find_ca_sins, "eight", "SIN 830692548");
    assert_finds(find_ca_sins, "nine", "SIN {}", "912345675");
    assert_misses(find_ca_sins, "luhn of the nine", "SIN 912345670");
}

#[test]
fn the_name_is_case_sensitive_where_the_reference_says_so() {
    assert_misses(find_ca_sins, "lowercase", "sin 130692544");
    assert_misses(find_ca_sins, "title case", "Sin 130692544");
    assert_misses(find_ca_sins, "name cut short", "SI 130692544");
    assert_misses(find_ca_sins, "other word", "SINS 130692544");
    assert_misses(find_ca_sins, "words between", "SIN of the member 130692544");
}

#[test]
fn a_digit_beside_the_name_does_not_hide_it_and_a_letter_does() {
    assert_finds(find_ca_sins, "digit", "order 9SIN {}", "130 692 544");
    assert_finds(find_ca_sins, "digit nas", "9NAS {}", "130692544");
    assert_misses(find_ca_sins, "letter", "BASIN 130 692 544");
    assert_misses(find_ca_sins, "letter nas", "ANAS 130 692 544");
    assert_misses(
        find_ca_sins,
        "letter long name",
        "xsocial insurance 130692544",
    );
    // The long names are matched in either case, but the test before them is ASCII letters.
    assert_finds(find_ca_sins, "dotted capital", "\u{130}SIN {}", "130692544");
}

#[test]
fn a_digit_or_hyphen_after_a_sin_hides_it() {
    for (name, text) in [
        ("digit", "SIN 1306925445"),
        ("hyphen", "SIN 130692544-"),
        ("hyphen grouped", "SIN 130-692-544-"),
        ("wide digit", "SIN 130692544\u{ff11}"),
    ] {
        assert_misses(find_ca_sins, name, text);
    }
    assert_finds(find_ca_sins, "space", "SIN {} 5", "130692544");
}

#[test]
fn a_hidden_sin_is_retried_one_character_later() {
    let text = "ASIN 130692544, SIN 527348916";
    assert_eq!(find_ca_sins(text), vec![(20, 29)]);
}

#[test]
fn a_sin_in_another_script_is_read_by_value() {
    let wide: String = "130692544"
        .chars()
        .skip(1)
        .map(|c| char::from_u32(0xFF10 + c.to_digit(10).unwrap()).unwrap())
        .collect();
    assert!(luhn_ok(&format!("1{wide}")));
    assert_finds(find_ca_sins, "wide", "SIN {}", &format!("1{wide}"));
}

#[test]
fn an_aadhaar_is_found_exactly() {
    for (index, (template, value)) in [
        ("uid {} ok", "2345 6789 0124"),
        ("uid {} ok", "4987-6543-2102"),
        ("Aadhaar no. {}", "987654321012"),
        ("Aadhaar Number {}", "987654321012"),
        ("AADHAAR NO. {}", "987654321012"),
        ("Aadhaar Number: {}", "987654321012"),
    ]
    .into_iter()
    .enumerate()
    {
        assert_finds(
            find_in_aadhaar,
            &format!("positive {index}"),
            template,
            value,
        );
    }
}

#[test]
fn every_name_of_an_aadhaar_is_read() {
    for (index, template) in [
        "aadhaar {}",
        "Aadhar {}",
        "AADHAR NO {}",
        "aadhaar #{}",
        "aadhaar # {}",
        "aadhaar:{}",
        "Aadhaar number {}",
        "Aadhaar NUMBER : {}",
        "aadhaar  no.\t{}",
        "309Aadhaar Number {}",
    ]
    .into_iter()
    .enumerate()
    {
        assert_finds(
            find_in_aadhaar,
            &format!("name {index}"),
            template,
            "987654321012",
        );
    }
}

#[test]
fn the_reference_aadhaar_near_misses_are_not_found() {
    for (name, text) in [
        ("verhoeff fails", "2345 6789 0125"),
        ("starts with 1", "1345 6789 0124"),
        ("no name", "order 234567890124"),
        ("bare verhoeff fails", "Aadhaar Number 987654321013"),
        ("bare starts with 1", "Aadhaar Number 187654321012"),
        ("thirteen digits", "Aadhaar Number 9876543210123"),
        ("longer word", "Aadhaar Numbers 987654321012"),
        ("starts like no", "Aadhaar Note 987654321012"),
        (
            "not beside it",
            "Aadhaar Number of the member: 987654321012",
        ),
    ] {
        assert_misses(find_in_aadhaar, name, text);
    }
}

#[test]
fn a_grouped_aadhaar_needs_one_separator_and_no_digit_or_hyphen_around_it() {
    for (name, text) in [
        ("mixed separators", "2345-6789 0124"),
        ("no separators", "2345 6789012"),
        ("digit before", "12345 6789 0124"),
        ("hyphen before", "-2345 6789 0124"),
        ("digit after", "2345 6789 01245"),
        ("hyphen after", "2345 6789 0124-"),
        ("starts with 0", "0345 6789 0124"),
    ] {
        assert_misses(find_in_aadhaar, name, text);
    }
    assert_finds(find_in_aadhaar, "letters around", "x{}y", "2345 6789 0124");
}

#[test]
fn a_bare_aadhaar_is_followed_by_anything_but_a_digit() {
    assert_misses(find_in_aadhaar, "digit", "Aadhaar 9876543210121");
    assert_finds(find_in_aadhaar, "hyphen", "Aadhaar {}-", "987654321012");
    assert_misses(
        find_in_aadhaar,
        "letter before the name",
        "Xaadhaar 580927525309",
    );
    assert_finds(
        find_in_aadhaar,
        "digit before the name",
        "309Aadhaar Number {}",
        "580927525309",
    );
}

#[test]
fn a_hidden_aadhaar_is_retried_one_character_later() {
    assert_eq!(
        find_in_aadhaar("12345 6789 0124, 2345 6789 0124"),
        vec![(17, 31)]
    );
    assert_eq!(
        find_in_aadhaar("Xaadhaar 580927525309 aadhaar 580927525309"),
        vec![(30, 42)]
    );
}

#[test]
fn an_aadhaar_in_another_script_is_read_by_value() {
    let wide: String = "345 6789 0124"
        .chars()
        .map(|c| {
            c.to_digit(10)
                .map_or(c, |d| char::from_u32(0xFF10 + d).unwrap())
        })
        .collect();
    assert_finds(find_in_aadhaar, "wide", "uid {}", &format!("2{wide}"));
}

#[test]
fn both_groups_of_an_aadhaar_are_reported_by_their_own_span() {
    let text = "2345 6789 0124 and Aadhaar 987654321012";
    assert_eq!(find_in_aadhaar(text), vec![(0, 14), (27, 39)]);
}

#[test]
fn an_empty_text_has_no_keyword_ids() {
    assert_misses(find_ca_sins, "sin", "");
    assert_misses(find_in_aadhaar, "aadhaar", "");
}
