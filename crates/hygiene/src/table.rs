//! The class table's parser, run by the compiler.
//!
//! `data/py_classes.txt` is read by `const` evaluation, so a malformed file is a build error
//! on every platform and never an empty class at run time. The format, written by
//! `scripts/gen_py_classes.py`:
//!
//! * `#` lines and blank lines are ignored; lines end in `\n` or `\r\n`;
//! * `[name]` starts a section;
//! * every other line of a section is `lo-hi` tokens (lowercase hex, inclusive code point
//!   ranges) separated by spaces; the ranges ascend and never overlap, and hold no surrogate.
//!
//! The messages are fixed strings and never quote the table.

/// The most ranges one section may hold (the largest, the word class, has 749).
const CAPACITY: usize = 1024;
/// The last code point.
const LAST: u32 = 0x10_FFFF;

/// The ranges of one section, inclusive and ascending.
pub(crate) struct Table {
    ranges: [(u32, u32); CAPACITY],
    len: usize,
}

impl Table {
    /// The ranges, ascending.
    pub(crate) const fn as_slice(&self) -> &[(u32, u32)] {
        self.ranges.split_at(self.len).0
    }
}

/// `after_bracket`, the rest of a line that began with `[`, is exactly `name]`.
const fn is_header(after_bracket: &[u8], name: &[u8]) -> bool {
    let Some((&b']', mut inner)) = after_bracket.split_last() else {
        return false;
    };
    let mut want = name;
    while let (Some((&a, inner_rest)), Some((&b, want_rest))) =
        (inner.split_first(), want.split_first())
    {
        if a != b {
            return false;
        }
        inner = inner_rest;
        want = want_rest;
    }
    inner.is_empty() && want.is_empty()
}

/// The next line of `text` without its `\n` or `\r\n`, and what follows it.
const fn next_line(text: &[u8]) -> (&[u8], &[u8]) {
    let mut length = 0;
    while let Some(&byte) = text.split_at(length).1.first() {
        if byte == b'\n' {
            break;
        }
        length += 1;
    }
    let (line, rest) = text.split_at(length);
    let line = match line.split_last() {
        Some((&b'\r', without)) => without,
        _ => line,
    };
    // Past the newline, when there is one.
    let rest = match rest.split_first() {
        Some((_, after)) => after,
        None => rest,
    };
    (line, rest)
}

/// The hex number at the start of `text` and what follows it.
const fn number(text: &[u8]) -> (u32, &[u8]) {
    let mut value: u32 = 0;
    let mut digits = 0;
    let mut rest = text;
    while let Some((&byte, after)) = rest.split_first() {
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => break,
        };
        value = value * 16 + digit as u32;
        assert!(value <= LAST, "class table: number above U+10FFFF");
        digits += 1;
        rest = after;
    }
    assert!(digits > 0, "class table: not a hex number");
    (value, rest)
}

/// The ranges of section `[name]` of `table`; panics (so, in a `static`, fails the build) when
/// the table is malformed or the section is missing, empty or repeated.
#[allow(clippy::panic)] // a malformed table is a build-time defect, not a runtime input
pub(crate) const fn parse(table: &str, name: &str) -> Table {
    let mut out = Table {
        ranges: [(0, 0); CAPACITY],
        len: 0,
    };
    let mut found = false;
    let mut inside = false;
    // The first code point the next range may start at.
    let mut next = 0;
    let mut text = table.as_bytes();
    while !text.is_empty() {
        let (line, after) = next_line(text);
        text = after;
        match line.split_first() {
            Some((b'#', _)) | None => {}
            Some((b'[', after_bracket)) => {
                inside = is_header(after_bracket, name.as_bytes());
                assert!(!(inside && found), "class table: section repeated");
                found |= inside;
            }
            Some(_) if !inside => {}
            Some(_) => {
                let mut rest = line;
                while let Some((&byte, tail)) = rest.split_first() {
                    if byte == b' ' {
                        rest = tail;
                        continue;
                    }
                    let (low, after_low) = number(rest);
                    let Some((&b'-', after_dash)) = after_low.split_first() else {
                        panic!("class table: range is not lo-hi");
                    };
                    let (high, after_high) = number(after_dash);
                    assert!(
                        !matches!(after_high.first(), Some(&b) if b != b' '),
                        "class table: range token runs on"
                    );
                    assert!(low <= high, "class table: range is reversed");
                    assert!(low >= next, "class table: ranges are unsorted or overlap");
                    assert!(
                        low > 0xDFFF || high < 0xD800,
                        "class table: range holds a surrogate"
                    );
                    let (_, free) = out.ranges.split_at_mut(out.len);
                    let Some(slot) = free.first_mut() else {
                        panic!("class table: too many ranges");
                    };
                    *slot = (low, high);
                    out.len += 1;
                    next = high + 1;
                    rest = after_high;
                }
            }
        }
    }
    assert!(
        found && out.len > 0,
        "class table: section missing or empty"
    );
    out
}

#[cfg(test)]
mod tests {
    use super::{CAPACITY, parse};

    fn ranges(table: &str, name: &str) -> Vec<(u32, u32)> {
        parse(table, name).as_slice().to_vec()
    }

    /// `count` one-code-point ranges, `0-0 2-2 4-4 ...`.
    fn tokens(count: usize) -> String {
        let at: Vec<String> = (0..count)
            .map(|i| format!("{:x}-{:x}", i * 2, i * 2))
            .collect();
        at.join(" ")
    }

    #[test]
    fn a_section_is_the_ranges_under_its_header_and_nothing_else() {
        let table = "# a comment\n\n[ab]\n9-9\n[a]\n1-2 5-5\n  c-d\n\n[b]\n7-8\n";
        assert_eq!(ranges(table, "a"), [(1, 2), (5, 5), (12, 13)]);
        assert_eq!(ranges(table, "ab"), [(9, 9)]);
        assert_eq!(ranges(table, "b"), [(7, 8)]);
    }

    #[test]
    fn a_table_reads_the_same_with_crlf_and_without_a_final_newline() {
        let table = "# c\n[a]\n1-2 5-5\n3a-3b\n[b]\n7-8";
        let crlf = table.replace('\n', "\r\n");
        for name in ["a", "b"] {
            assert_eq!(ranges(&crlf, name), ranges(table, name), "{name}");
        }
        assert_eq!(ranges(&crlf, "a"), [(1, 2), (5, 5), (0x3a, 0x3b)]);
        assert_eq!(ranges("[a]\n1-2", "a"), [(1, 2)]);
        assert_eq!(ranges("[a]\r\n1-2 \r\n", "a"), [(1, 2)]);
    }

    #[test]
    fn the_edges_of_the_valid_values_are_accepted() {
        assert_eq!(
            ranges("[a]\n0-0 10ffff-10ffff\n", "a"),
            [(0, 0), (0x10_FFFF, 0x10_FFFF)]
        );
        assert_eq!(ranges("[a]\n1-2 3-4\n", "a"), [(1, 2), (3, 4)]);
        assert_eq!(
            ranges("[a]\nd7ff-d7ff e000-e000\n", "a"),
            [(0xD7FF, 0xD7FF), (0xE000, 0xE000)]
        );
        assert_eq!(
            ranges(&format!("[a]\n{}\n", tokens(CAPACITY)), "a").len(),
            CAPACITY
        );
    }

    #[test]
    fn a_header_must_be_the_exact_name() {
        assert_eq!(ranges("[ab]\n1-1\n[a]\n2-2\n", "a"), [(2, 2)]);
        assert_eq!(ranges("[a]\n2-2\n[ab]\n1-1\n", "a"), [(2, 2)]);
        assert_eq!(ranges("[a]\n2-2\n[a\n1-1\n", "a"), [(2, 2)]);
        assert_eq!(ranges("[ab]\n2-2\n", "ab"), [(2, 2)]);
    }

    #[test]
    #[should_panic(expected = "section missing or empty")]
    fn a_missing_section_fails() {
        ranges("[b]\n1-1\n", "a");
    }

    #[test]
    #[should_panic(expected = "section missing or empty")]
    fn an_unterminated_header_is_no_section() {
        ranges("[a\n1-1\n", "a");
    }

    #[test]
    #[should_panic(expected = "section missing or empty")]
    fn an_empty_section_fails() {
        ranges("[a]\n# only a comment\n[b]\n1-1\n", "a");
    }

    #[test]
    #[should_panic(expected = "section repeated")]
    fn a_repeated_section_fails() {
        ranges("[a]\n1-1\n[a]\n2-2\n", "a");
    }

    #[test]
    #[should_panic(expected = "not a hex number")]
    fn a_non_hex_token_fails() {
        ranges("[a]\n1-2 xy-3\n", "a");
    }

    #[test]
    #[should_panic(expected = "not a hex number")]
    fn an_uppercase_digit_is_not_the_format() {
        ranges("[a]\nA-B\n", "a");
    }

    #[test]
    #[should_panic(expected = "not a hex number")]
    fn a_range_without_its_end_fails() {
        ranges("[a]\n1-\n", "a");
    }

    #[test]
    #[should_panic(expected = "not lo-hi")]
    fn a_single_number_fails() {
        ranges("[a]\n1-2 5\n", "a");
    }

    #[test]
    #[should_panic(expected = "not lo-hi")]
    fn a_number_followed_by_another_character_fails() {
        ranges("[a]\n1:2\n", "a");
    }

    #[test]
    #[should_panic(expected = "runs on")]
    fn a_token_with_trailing_characters_fails() {
        ranges("[a]\n1-2x\n", "a");
    }

    #[test]
    #[should_panic(expected = "above U+10FFFF")]
    fn a_code_point_past_the_last_fails() {
        ranges("[a]\n110000-110001\n", "a");
    }

    #[test]
    #[should_panic(expected = "above U+10FFFF")]
    fn a_number_too_long_for_a_code_point_fails() {
        ranges("[a]\n1-fffffffff\n", "a");
    }

    #[test]
    #[should_panic(expected = "reversed")]
    fn a_reversed_range_fails() {
        ranges("[a]\n5-3\n", "a");
    }

    #[test]
    #[should_panic(expected = "unsorted or overlap")]
    fn an_unsorted_range_fails() {
        ranges("[a]\n5-6 1-2\n", "a");
    }

    #[test]
    #[should_panic(expected = "unsorted or overlap")]
    fn an_overlapping_range_fails() {
        ranges("[a]\n1-5 5-6\n", "a");
    }

    #[test]
    #[should_panic(expected = "surrogate")]
    fn a_range_ending_in_the_surrogates_fails() {
        ranges("[a]\nd7ff-d800\n", "a");
    }

    #[test]
    #[should_panic(expected = "surrogate")]
    fn a_range_starting_in_the_surrogates_fails() {
        ranges("[a]\ndfff-e000\n", "a");
    }

    #[test]
    #[should_panic(expected = "too many ranges")]
    fn a_section_over_the_capacity_fails() {
        ranges(&format!("[a]\n{}\n", tokens(CAPACITY + 1)), "a");
    }
}
