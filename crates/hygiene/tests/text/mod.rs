//! Python's character classes and the offset conversions: the predicates against the reference
//! ranges, the regex spellings against the crate, and the edges of every offset function.
use dagnam_hygiene::{
    DIGIT_CLASS, SPACE_CLASS, WORD_CLASS, case_sensitive, char_at, char_before, to_char_offsets,
};
use serde::Deserialize;

use super::{FindingsCase, cases};

/// The lines of `corpus/classes.jsonl`, and the ranges and code points in each of its classes.
const CLASSES_CASES: usize = 1;
const CLASS_RANGES: [usize; 3] = [749, 10, 64];
const CLASS_CODE_POINTS: [usize; 3] = [138_558, 29, 680];
/// The code points of Unicode, one past the last.
const CODE_POINTS: usize = 0x11_0000;
/// The one `classes.jsonl` line: inclusive code-point ranges of Python's `\w`, `\s` and `\d`.
#[derive(Deserialize)]
struct ClassesCase {
    word: Vec<(u32, u32)>,
    space: Vec<(u32, u32)>,
    digit: Vec<(u32, u32)>,
}

/// One class of the reference beside the port's two spellings of it.
struct Class {
    name: &'static str,
    ranges: Vec<(u32, u32)>,
    predicate: fn(char) -> bool,
    spelling: &'static str,
}

/// The three classes: `word`, `space`, `digit`.
fn classes() -> [Class; 3] {
    let case = cases::<ClassesCase>("classes").remove(0);
    let class = |name, ranges, predicate, spelling: &'static String| Class {
        name,
        ranges,
        predicate,
        spelling: spelling.as_str(),
    };
    [
        class("word", case.word, dagnam_hygiene::is_py_word, &WORD_CLASS),
        class(
            "space",
            case.space,
            dagnam_hygiene::is_py_space,
            &SPACE_CLASS,
        ),
        class(
            "digit",
            case.digit,
            dagnam_hygiene::is_py_digit,
            &DIGIT_CLASS,
        ),
    ]
}

/// One flag per code point: is it inside one of the inclusive `ranges`?
fn bitmap(ranges: &[(u32, u32)]) -> Vec<bool> {
    let mut flags = vec![false; CODE_POINTS];
    for &(low, high) in ranges {
        flags[low as usize..=high as usize].fill(true);
    }
    flags
}

/// Every Unicode scalar value, ascending: no surrogate can be in a `&str`.
fn every_char() -> String {
    (0..=0x10_FFFF).filter_map(char::from_u32).collect()
}

#[test]
fn the_class_ranges_are_sorted_disjoint_merged_and_free_of_surrogates() {
    for (index, Class { name, ranges, .. }) in classes().into_iter().enumerate() {
        assert_eq!(ranges.len(), CLASS_RANGES[index], "{name}");
        assert!(
            ranges
                .iter()
                .all(|&(low, high)| low <= high && high < 0x11_0000)
        );
        assert!(ranges.windows(2).all(|w| w[0].1 + 1 < w[1].0), "{name}");
        assert!(
            ranges
                .iter()
                .all(|&(low, high)| high < 0xD800 || low > 0xDFFF),
            "{name}"
        );
        let points = bitmap(&ranges).into_iter().filter(|&member| member).count();
        assert_eq!(points, CLASS_CODE_POINTS[index], "{name}");
    }
}

#[test]
fn every_predicate_agrees_with_the_reference_on_every_code_point() {
    for Class {
        name,
        ranges,
        predicate,
        ..
    } in classes()
    {
        let flags = bitmap(&ranges);
        let mut checked = 0;
        for code in 0..=0x10_FFFF_u32 {
            if let Some(c) = char::from_u32(code) {
                assert_eq!(predicate(c), flags[code as usize], "{name} at U+{code:04X}");
                checked += 1;
            }
        }
        assert_eq!(checked, CODE_POINTS - 0x800, "{name}: every scalar value");
    }
}

#[test]
fn the_predicates_keep_the_edges_the_crates_own_classes_get_wrong() {
    // The superscript two is alphanumeric but not a digit; the combining ypogegrammeni is
    // alphabetic in Rust's `\w` and not in Python's; U+001C is a space only to Python.
    assert!(dagnam_hygiene::is_py_word('\u{b2}') && dagnam_hygiene::is_py_word('_'));
    assert!(!dagnam_hygiene::is_py_digit('\u{b2}') && dagnam_hygiene::is_py_digit('\u{660}'));
    assert!(!dagnam_hygiene::is_py_word('\u{345}') && !dagnam_hygiene::is_py_word('-'));
    assert!(dagnam_hygiene::is_py_space('\u{1c}') && dagnam_hygiene::is_py_space('\u{85}'));
    assert!(!dagnam_hygiene::is_py_space('\u{200b}') && !dagnam_hygiene::is_py_space('a'));
}

#[test]
fn every_class_spelling_compiles_and_matches_exactly_the_members() {
    let all = every_char();
    for Class {
        name,
        predicate,
        spelling: class,
        ..
    } in classes()
    {
        let regex = regex::Regex::new(class).unwrap();
        let matched: Vec<char> = regex
            .find_iter(&all)
            .flat_map(|m| m.as_str().chars())
            .collect();
        let expected: Vec<char> = all.chars().filter(|&c| predicate(c)).collect();
        assert_same_chars(&matched, &expected, name);
        let negated = regex::Regex::new(&format!("[^{class}]")).unwrap();
        let outside: usize = negated
            .find_iter(&all)
            .map(|m| m.as_str().chars().count())
            .sum();
        assert_eq!(
            outside,
            all.chars().count() - expected.len(),
            "{name}: negated"
        );
        assert!(
            class.starts_with("[\\u{") && class.ends_with("}]"),
            "{name}"
        );
    }
}

#[test]
fn the_word_class_without_the_underscore_is_an_intersection_with_a_negated_class() {
    // `[^\W_]` in the documented spelling: the word class intersected with "not an underscore".
    let regex = regex::Regex::new(&format!("[{}&&[^_]]", WORD_CLASS.as_str())).unwrap();
    let all = every_char();
    let found: Vec<char> = regex
        .find_iter(&all)
        .flat_map(|m| m.as_str().chars())
        .collect();
    let words: Vec<char> = all
        .chars()
        .filter(|&c| dagnam_hygiene::is_py_word(c) && c != '_')
        .collect();
    assert_same_chars(&found, &words, "word class without underscore");
}

#[test]
fn the_case_sensitive_spelling_does_not_fold_under_case_insensitive_matching() {
    // U+0345 folds to a Greek letter, so under `(?i)` the bare class matches it; the
    // `(?-i:...)` form is Python's `\w`, which ignores the flag.
    let bare = regex::Regex::new(&format!("(?i){}", WORD_CLASS.as_str())).unwrap();
    assert!(bare.is_match("\u{345}"));
    let wrapped = regex::Regex::new(&format!("(?i){}", case_sensitive(&WORD_CLASS))).unwrap();
    assert!(!wrapped.is_match("\u{345}"));
    let all = every_char();
    let found: Vec<char> = wrapped
        .find_iter(&all)
        .flat_map(|m| m.as_str().chars())
        .collect();
    let words: Vec<char> = all
        .chars()
        .filter(|&c| dagnam_hygiene::is_py_word(c))
        .collect();
    assert_same_chars(&found, &words, "wrapped word class");
    assert_eq!(case_sensitive("[a]"), "(?-i:[a])");
}

#[test]
fn a_word_class_beside_other_members_is_an_alternation_of_wrapped_and_bare_classes() {
    // The spelling for a Python `[\w.+-]` under `(?i)`: the wrapper goes round the whole
    // standalone class, never inside a bracket class.
    let pattern = format!("(?i)^(?:{}|[.+-])$", case_sensitive(&WORD_CLASS));
    let regex = regex::Regex::new(&pattern).unwrap();
    for member in ["a", "K", "_", "7", "\u{b2}", ".", "+", "-"] {
        assert!(regex.is_match(member), "{member:?}");
    }
    for other in ["\u{345}", "/", " ", "", "ab", "\u{1f}"] {
        assert!(!regex.is_match(other), "{other:?}");
    }
}

/// Fails with the counts and the first differing code point, never the lists (which hold up
/// to 138,000 characters).
fn assert_same_chars(got: &[char], want: &[char], what: &str) {
    let first = got.iter().zip(want).position(|(a, b)| a != b);
    assert!(
        got.len() == want.len() && first.is_none(),
        "{what}: {} characters, {} expected; first difference at index {first:?}: {:?} against {:?}",
        got.len(),
        want.len(),
        first.and_then(|at| got.get(at)).map(|&c| u32::from(c)),
        first.and_then(|at| want.get(at)).map(|&c| u32::from(c)),
    );
}

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

#[test]
fn the_classes_file_holds_exactly_the_one_line_the_readme_counts() {
    assert_eq!(cases::<ClassesCase>("classes").len(), CLASSES_CASES);
}
