//! `find_dates_of_birth`.
use dagnam_hygiene::find_dates_of_birth;

use super::{assert_finds, assert_misses, fill};

#[test]
fn the_reference_dates_are_found_exactly() {
    for (index, (template, value)) in [
        ("DOB: {}", "14/03/1987"),
        ("D.O.B. {}", "03/14/87"),
        ("Date of birth is {}", "1987-03-14"),
        ("born on {}, in Leeds", "March 14, 1987"),
        ("birthdate - {}", "14th Mar 1987"),
        ("Birthday: {}", "14.03.1987"),
    ]
    .into_iter()
    .enumerate()
    {
        assert_finds(
            find_dates_of_birth,
            &format!("positive {index}"),
            template,
            value,
        );
    }
}

#[test]
fn the_reference_near_misses_are_not_found() {
    for (index, text) in [
        "Date: 14/03/1987, due 1987-03-14",
        "born in 1987; DOB: unknown",
        "birthday party on 14/03/2026",
        "adobe 14/03/1987",
        "DOB: 32/13/1987",
    ]
    .into_iter()
    .enumerate()
    {
        assert_misses(find_dates_of_birth, &format!("near miss {index}"), text);
    }
}

#[test]
fn every_keyword_and_every_date_form_is_read() {
    for (index, (template, value)) in [
        ("dob {}", "1/2/99"),
        ("DoB={}", "01-02-1999"),
        ("d.o.b {}", "1999.02.01"),
        ("date  of\n birth : {}", "2/1/2099"),
        ("Birth Date {}", "1999/2/1"),
        ("BIRTHDATE: {}", "31.12.99"),
        ("birthday is {}", "12/31/1999"),
        ("born {}", "1st February 1999"),
        ("born  on {}", "2nd Feb, 1999"),
        ("was born on {}", "3rd feb 2000"),
        ("DOB was {}", "4th Mar. 2000"),
        ("dob: {}", "Sept. 5, 2001"),
        ("dob: {}", "Sept 5th 2001"),
        ("dob: {}", "Oct 31st, 1999"),
        ("dob: {}", "november 1 1999"),
        ("dob: {}", "20 December 1999"),
        ("dob: {}", "6 june 2000"),
        ("dob: {}", "7 July 2000"),
        ("dob: {}", "8 Aug 2000"),
        ("dob: {}", "9 Jan 2000"),
        ("dob: {}", "10 September 1999"),
        ("dob: {}", "11 Nov 1999"),
        ("dob: {}", "April 12th 1999"),
        ("dob: {}", "13 May 1999"),
        ("dob: {}", "14 Dec 1999"),
        ("dob: {}", "15 Oct 1999"),
        ("dob: {}", "16 Sep 1999"),
        ("dob: {}", "17 Mar 1999"),
        ("dob: {}", "18 Jul 1999"),
        ("dob: {}", "19 Aug. 1999"),
        ("dob: {}", "21\u{a0}Jan\u{a0}1999"),
    ]
    .into_iter()
    .enumerate()
    {
        assert_finds(
            find_dates_of_birth,
            &format!("form {index}"),
            template,
            value,
        );
    }
}

#[test]
fn case_and_the_turkish_i_are_read_as_python_reads_them() {
    // Under `(?i)` Python reads U+0130 and U+0131 as `i`.
    assert_finds(
        find_dates_of_birth,
        "upper",
        "DATE OF BIRTH: {}",
        "14 APRIL 1987",
    );
    assert_finds(
        find_dates_of_birth,
        "dotted capital in birth",
        "B\u{130}RTHDAY {}",
        "14.03.1987",
    );
    assert_finds(
        find_dates_of_birth,
        "dotless in birth",
        "date of b\u{131}rth {}",
        "14.03.1987",
    );
    assert_finds(
        find_dates_of_birth,
        "in birthdate",
        "b\u{131}rth date {}",
        "14.03.1987",
    );
    assert_finds(
        find_dates_of_birth,
        "in april",
        "born {}",
        "14 Apr\u{130}l 1987",
    );
    assert_finds(
        find_dates_of_birth,
        "in is",
        "D.O.B. \u{130}s {}",
        "14.03.1987",
    );
}

#[test]
fn the_year_digits_of_any_script_make_a_date() {
    // Only the `\d` of the pattern reads them; the day, month and the `19`/`20` are ASCII.
    assert_finds(find_dates_of_birth, "fullwidth", "DOB: {}", "14/03/\u{ff18}\u{ff17}");
    assert_misses(find_dates_of_birth, "fullwidth day", "DOB: \u{ff11}\u{ff14}/03/87");
}

#[test]
fn a_digit_beside_the_keyword_does_not_hide_it() {
    assert_finds(find_dates_of_birth, "digit", "1987DOB: {}", "14/03/1987");
    assert_finds(
        find_dates_of_birth,
        "underscore",
        "x_born on {}",
        "14/03/1987",
    );
    assert_finds(
        find_dates_of_birth,
        "other letter",
        "\u{e9}DOB: {}",
        "14/03/1987",
    );
}

#[test]
fn a_letter_beside_the_keyword_hides_it() {
    for (name, text) in [
        ("ascii", "adobe 14/03/1987"),
        ("ascii capital", "ADOB 14/03/1987"),
        ("dotted capital i", "\u{130}DOB 14/03/1987"),
        ("dotless i", "\u{131}DOB 14/03/1987"),
        ("long s", "\u{17f}DOB 14/03/1987"),
        ("kelvin", "\u{212a}DOB 14/03/1987"),
    ] {
        assert_misses(find_dates_of_birth, name, text);
    }
}

#[test]
fn a_hidden_keyword_is_retried_one_character_later() {
    let (text, (start, end)) = fill("aDOB: 14/03/1987, DOB: {}", "15/03/1987");
    assert_eq!(find_dates_of_birth(&text), vec![(start, end)]);
}

#[test]
fn a_digit_or_a_separator_and_digit_after_the_date_hides_it() {
    for (name, text) in [
        ("digit", "DOB: 14/03/19875"),
        ("slash digit", "DOB: 14/03/1987/5"),
        ("dot digit", "DOB: 14/03/87.5"),
        ("hyphen digit", "DOB: 14/03/87-5"),
        ("iso day", "DOB: 1987-03-145"),
    ] {
        assert_misses(find_dates_of_birth, name, text);
    }
}

#[test]
fn a_separator_with_no_digit_after_the_date_does_not() {
    assert_finds(find_dates_of_birth, "dot", "DOB: {}.", "14/03/87");
    assert_finds(find_dates_of_birth, "hyphen", "DOB: {}- x", "14/03/87");
    assert_finds(
        find_dates_of_birth,
        "slash then end",
        "DOB: {}/",
        "14/03/87",
    );
    assert_finds(find_dates_of_birth, "letter", "DOB: {}x", "14/03/87");
    assert_finds(find_dates_of_birth, "end", "DOB: {}", "14/03/87");
}

#[test]
fn a_date_with_a_two_digit_day_gives_way_to_the_later_alternative() {
    // `0?[1-9]` takes the `1` of `14`, meets a digit, and `[12]\d` takes both.
    assert_finds(find_dates_of_birth, "iso", "dob {}", "1987-03-14");
    assert_finds(find_dates_of_birth, "month first", "dob {}", "12/14/1987");
}

#[test]
fn several_dates_are_found_in_order() {
    let text = "DOB: 14/03/1987 and born on 1 May 1990";
    assert_eq!(find_dates_of_birth(text), vec![(5, 15), (28, 38)]);
}

#[test]
fn an_empty_text_has_no_dates() {
    assert_misses(find_dates_of_birth, "empty", "");
}
