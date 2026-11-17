//! The offset conversions and the character lookups: the edges of every function that reads a
//! byte offset of a text. The classes are in `classes`.
use dagnam_hygiene::{char_at, char_before, to_char_offsets};

use super::{FindingsCase, cases};

mod classes;
mod versions;

#[test]
fn byte_offsets_become_code_point_offsets_in_one_pass() {
    assert_eq!(
        to_char_offsets("a\u{e9}\u{1d4b3}z", &[0, 1, 3, 7, 8]),
        [0, 1, 2, 3, 4]
    );
    assert_eq!(to_char_offsets("abc", &[0, 0, 1, 3, 3]), [0, 0, 1, 3, 3]);
    assert_eq!(to_char_offsets("abc", &[]), Vec::<usize>::new());
    assert_eq!(to_char_offsets("", &[0, 0]), [0, 0]);
}

#[test]
fn an_offset_inside_a_character_counts_it_and_one_past_the_end_counts_them_all() {
    // "\u{e9}" is two bytes, U+1D4B3 four: an offset inside one rounds up to the next boundary.
    assert_eq!(to_char_offsets("\u{e9}\u{1d4b3}", &[1, 3, 5]), [1, 2, 2]);
    assert_eq!(to_char_offsets("a\u{e9}", &[4, 99, usize::MAX]), [2, 2, 2]);
    assert_eq!(to_char_offsets("", &[1]), [0]);
}

#[test]
fn offsets_in_any_order_get_their_own_answers_in_the_order_given() {
    // The ends of overlapping spans, flattened: not ascending.
    assert_eq!(
        to_char_offsets("abcdefghijkl", &[0, 10, 2, 5]),
        [0, 10, 2, 5]
    );
    assert_eq!(to_char_offsets("abcd", &[3, 1, 4]), [3, 1, 4]);
    assert_eq!(to_char_offsets("abcd", &[4, 3, 2, 1, 0]), [4, 3, 2, 1, 0]);
    assert_eq!(
        to_char_offsets("abcd", &[2, 0, 2, 4, 0, 2]),
        [2, 0, 2, 4, 0, 2]
    );
    // Multi-byte characters, shuffled; 1 and 6 are inside characters, 99 is past the end.
    let text = "a\u{e9}\u{1d4b3}z";
    assert_eq!(
        to_char_offsets(text, &[8, 3, 99, 0, 7, 1, 6]),
        [4, 2, 4, 0, 3, 1, 3]
    );
    assert_eq!(to_char_offsets(text, &[2, 5]), [2, 3]);
    assert_eq!(to_char_offsets("", &[5, 0, 3]), [0, 0, 0]);
}

#[test]
fn an_offset_answers_the_same_alone_as_among_others_in_any_order() {
    for case in cases::<FindingsCase>("findings") {
        let mut bytes: Vec<usize> = case.text.char_indices().map(|(at, _)| at).collect();
        bytes.push(case.text.len());
        let alone: Vec<usize> = bytes
            .iter()
            .map(|&at| to_char_offsets(&case.text, &[at])[0])
            .collect();
        // Ascending, descending, and a stride that interleaves the two ends.
        let descending: Vec<usize> = bytes.iter().rev().copied().collect();
        let stride: Vec<usize> = (0..bytes.len())
            .map(|i| bytes[(i * 7 + 3) % bytes.len()])
            .collect();
        for order in [bytes.clone(), descending, stride] {
            let expected: Vec<usize> = order
                .iter()
                .map(|&at| alone[bytes.binary_search(&at).unwrap()])
                .collect();
            assert_eq!(to_char_offsets(&case.text, &order), expected);
        }
    }
}

#[test]
fn every_boundary_of_every_findings_text_converts_to_its_index() {
    for case in cases::<FindingsCase>("findings") {
        let mut bytes: Vec<usize> = case.text.char_indices().map(|(at, _)| at).collect();
        bytes.push(case.text.len());
        let expected: Vec<usize> = (0..bytes.len()).collect();
        assert_eq!(to_char_offsets(&case.text, &bytes), expected);
    }
}

#[test]
fn the_character_before_and_at_an_offset_need_a_boundary() {
    assert_eq!(char_before("a\u{e9}", 3), Some('\u{e9}'));
    assert_eq!(char_before("a\u{e9}", 1), Some('a'));
    assert_eq!(char_before("\u{1d4b3}z", 4), Some('\u{1d4b3}'));
    // Inside a character is `None` too, not the character around it.
    assert_eq!(char_before("\u{660}x", 1), None);
    assert_eq!(char_at("\u{660}x", 1), None);
    assert_eq!(char_before("\u{660}x", 2), Some('\u{660}'));
    assert_eq!(char_at("\u{660}x", 2), Some('x'));
    assert_eq!(char_before("a\u{e9}", 0), None);
    assert_eq!(char_before("a\u{e9}", 2), None);
    assert_eq!(char_before("a\u{e9}", 4), None);
    assert_eq!(char_before("", 0), None);
    assert_eq!(char_at("a\u{e9}", 0), Some('a'));
    assert_eq!(char_at("a\u{e9}", 1), Some('\u{e9}'));
    assert_eq!(char_at("\u{1d4b3}z", 0), Some('\u{1d4b3}'));
    assert_eq!(char_at("a\u{e9}", 2), None);
    assert_eq!(char_at("a\u{e9}", 3), None);
    assert_eq!(char_at("a\u{e9}", usize::MAX), None);
    assert_eq!(char_at("", 0), None);
}
