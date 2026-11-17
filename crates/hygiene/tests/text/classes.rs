//! Python's character classes, one table per Unicode version: the predicates against the
//! reference ranges of each version on every code point, the regex spellings against the
//! predicates, and what separates one version from the next.
use dagnam_hygiene::{
    UnicodeVersion, case_sensitive, digit_class, is_py_digit, is_py_space, is_py_word, space_class,
    word_class,
};
use serde::Deserialize;

use crate::cases;

/// For each version in `UnicodeVersion::ALL`: the ranges and code points in each class
/// (`word`, `space`, `digit`) of its line of `corpus/classes.jsonl`.
const CLASS_RANGES: [[usize; 3]; 3] = [[748, 10, 64], [749, 10, 64], [771, 10, 71]];
const CLASS_CODE_POINTS: [[usize; 3]; 3] =
    [[137_936, 29, 680], [138_558, 29, 680], [142_940, 29, 760]];
/// The code points of Unicode, one past the last.
const CODE_POINTS: usize = 0x11_0000;

/// One `classes.jsonl` line: the Unicode version and the inclusive code-point ranges of
/// Python's `\w`, `\s` and `\d` under the interpreter that has it.
#[derive(Deserialize)]
struct ClassesCase {
    unicode: String,
    word: Vec<(u32, u32)>,
    space: Vec<(u32, u32)>,
    digit: Vec<(u32, u32)>,
}

/// One class of the reference beside the port's two spellings of it.
struct Class {
    name: &'static str,
    ranges: Vec<(u32, u32)>,
    predicate: fn(UnicodeVersion, char) -> bool,
    spelling: &'static str,
}

/// The line of every shipped version, in the order of `UnicodeVersion::ALL`.
fn lines() -> Vec<(UnicodeVersion, ClassesCase)> {
    let lines = cases::<ClassesCase>("classes");
    assert_eq!(lines.len(), UnicodeVersion::ALL.len());
    UnicodeVersion::ALL.into_iter().zip(lines).collect()
}

/// The three classes of a version: `word`, `space`, `digit`.
fn classes(version: UnicodeVersion, case: ClassesCase) -> [Class; 3] {
    let class = |name, ranges, predicate, spelling: &'static str| Class {
        name,
        ranges,
        predicate,
        spelling,
    };
    [
        class("word", case.word, is_py_word, word_class(version)),
        class("space", case.space, is_py_space, space_class(version)),
        class("digit", case.digit, is_py_digit, digit_class(version)),
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
fn the_classes_file_holds_one_line_per_shipped_version_oldest_first() {
    for (version, case) in lines() {
        assert_eq!(case.unicode, version.as_str());
    }
    assert_eq!(
        UnicodeVersion::ALL.map(UnicodeVersion::as_str),
        ["15.0.0", "15.1.0", "16.0.0"]
    );
}

#[test]
fn the_class_ranges_are_sorted_disjoint_merged_and_free_of_surrogates() {
    for (position, (version, case)) in lines().into_iter().enumerate() {
        for (index, Class { name, ranges, .. }) in classes(version, case).into_iter().enumerate() {
            let what = format!("{} {name}", version.as_str());
            assert_eq!(ranges.len(), CLASS_RANGES[position][index], "{what}");
            assert!(
                ranges
                    .iter()
                    .all(|&(low, high)| low <= high && high < 0x11_0000)
            );
            assert!(ranges.windows(2).all(|w| w[0].1 + 1 < w[1].0), "{what}");
            assert!(
                ranges
                    .iter()
                    .all(|&(low, high)| high < 0xD800 || low > 0xDFFF),
                "{what}"
            );
            let points = bitmap(&ranges).into_iter().filter(|&member| member).count();
            assert_eq!(points, CLASS_CODE_POINTS[position][index], "{what}");
        }
    }
}

#[test]
fn every_predicate_agrees_with_its_versions_reference_on_every_code_point() {
    for (version, case) in lines() {
        for Class {
            name,
            ranges,
            predicate,
            ..
        } in classes(version, case)
        {
            let flags = bitmap(&ranges);
            let mut checked = 0;
            for code in 0..=0x10_FFFF_u32 {
                if let Some(c) = char::from_u32(code) {
                    assert_eq!(
                        predicate(version, c),
                        flags[code as usize],
                        "{} {name} at U+{code:04X}",
                        version.as_str()
                    );
                    checked += 1;
                }
            }
            assert_eq!(checked, CODE_POINTS - 0x800, "{name}: every scalar value");
        }
    }
}

/// How many code points `predicate` answers differently under `a` and `b`.
fn differ(
    a: UnicodeVersion,
    b: UnicodeVersion,
    predicate: fn(UnicodeVersion, char) -> bool,
) -> usize {
    (0..=0x10_FFFF)
        .filter_map(char::from_u32)
        .filter(|&c| predicate(a, c) != predicate(b, c))
        .count()
}

#[test]
fn the_versions_differ_where_the_interpreters_do() {
    use UnicodeVersion::{V15_0, V15_1, V16_0};
    // CPython 3.12 against 3.13: CJK Extension I is a letter from Unicode 15.1.
    assert_eq!(differ(V15_0, V15_1, is_py_word), 622);
    assert_eq!(differ(V15_0, V15_1, is_py_digit), 0);
    assert_eq!(differ(V15_0, V15_1, is_py_space), 0);
    assert!(!is_py_word(V15_0, '\u{2ebf0}') && is_py_word(V15_1, '\u{2ebf0}'));
    assert!(is_py_word(V16_0, '\u{2ebf0}') && is_py_word(V16_0, '\u{2ee5d}'));
    // CPython 3.13 against 3.14: Unicode 16 adds letters and ten-digit runs.
    assert_eq!(differ(V15_1, V16_0, is_py_word), 4382);
    assert_eq!(differ(V15_1, V16_0, is_py_digit), 80);
    assert_eq!(differ(V15_1, V16_0, is_py_space), 0);
    assert!(!is_py_word(V15_1, '\u{1c89}') && is_py_word(V16_0, '\u{1c89}'));
    assert!(!is_py_digit(V15_1, '\u{10d40}') && is_py_digit(V16_0, '\u{10d40}'));
}

#[test]
fn the_predicates_keep_the_edges_the_crates_own_classes_get_wrong() {
    // The superscript two is alphanumeric but not a digit; the combining ypogegrammeni is
    // alphabetic in Rust's `\w` and not in Python's; U+001C is a space only to Python.
    for v in UnicodeVersion::ALL {
        assert!(is_py_word(v, '\u{b2}') && is_py_word(v, '_'));
        assert!(!is_py_digit(v, '\u{b2}') && is_py_digit(v, '\u{660}'));
        assert!(!is_py_word(v, '\u{345}') && !is_py_word(v, '-'));
        assert!(is_py_space(v, '\u{1c}') && is_py_space(v, '\u{85}'));
        assert!(!is_py_space(v, '\u{200b}') && !is_py_space(v, 'a'));
    }
}

#[test]
fn every_class_spelling_compiles_and_matches_exactly_the_members() {
    let all = every_char();
    for (version, case) in lines() {
        for Class {
            name,
            predicate,
            spelling: class,
            ..
        } in classes(version, case)
        {
            let name = format!("{} {name}", version.as_str());
            let regex = regex::Regex::new(class).unwrap();
            let matched: Vec<char> = regex
                .find_iter(&all)
                .flat_map(|m| m.as_str().chars())
                .collect();
            let expected: Vec<char> = all.chars().filter(|&c| predicate(version, c)).collect();
            assert_same_chars(&matched, &expected, &name);
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
}

#[test]
fn the_word_class_without_the_underscore_is_an_intersection_with_a_negated_class() {
    // `[^\W_]` in the documented spelling: the word class intersected with "not an underscore".
    let all = every_char();
    for version in UnicodeVersion::ALL {
        let regex = regex::Regex::new(&format!("[{}&&[^_]]", word_class(version))).unwrap();
        let found: Vec<char> = regex
            .find_iter(&all)
            .flat_map(|m| m.as_str().chars())
            .collect();
        let words: Vec<char> = all
            .chars()
            .filter(|&c| is_py_word(version, c) && c != '_')
            .collect();
        assert_same_chars(&found, &words, "word class without underscore");
    }
}

#[test]
fn the_case_sensitive_spelling_does_not_fold_under_case_insensitive_matching() {
    // U+0345 folds to a Greek letter, so under `(?i)` the bare class matches it; the
    // `(?-i:...)` form is Python's `\w`, which ignores the flag.
    let all = every_char();
    for version in UnicodeVersion::ALL {
        let bare = regex::Regex::new(&format!("(?i){}", word_class(version))).unwrap();
        assert!(bare.is_match("\u{345}"));
        let wrapped =
            regex::Regex::new(&format!("(?i){}", case_sensitive(word_class(version)))).unwrap();
        assert!(!wrapped.is_match("\u{345}"));
        let found: Vec<char> = wrapped
            .find_iter(&all)
            .flat_map(|m| m.as_str().chars())
            .collect();
        let words: Vec<char> = all.chars().filter(|&c| is_py_word(version, c)).collect();
        assert_same_chars(&found, &words, "wrapped word class");
    }
    assert_eq!(case_sensitive("[a]"), "(?-i:[a])");
}

#[test]
fn a_word_class_beside_other_members_is_an_alternation_of_wrapped_and_bare_classes() {
    // The spelling for a Python `[\w.+-]` under `(?i)`: the wrapper goes round the whole
    // standalone class, never inside a bracket class.
    for version in UnicodeVersion::ALL {
        let pattern = format!("(?i)^(?:{}|[.+-])$", case_sensitive(word_class(version)));
        let regex = regex::Regex::new(&pattern).unwrap();
        for member in ["a", "K", "_", "7", "\u{b2}", ".", "+", "-"] {
            assert!(regex.is_match(member), "{member:?}");
        }
        for other in ["\u{345}", "/", " ", "", "ab", "\u{1f}"] {
            assert!(!regex.is_match(other), "{other:?}");
        }
    }
}

/// Fails with the counts and the first differing code point, never the lists (which hold up
/// to 143,000 characters).
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
fn near_dedup_and_minhash_read_no_character_class() {
    // Near-duplicate detection splits on the space only and hashes bytes: neither names a
    // class table, so neither takes a version. If one starts to, it must take one.
    for file in ["cluster.rs", "minhash.rs"] {
        let path = format!("{}/src/{file}", env!("CARGO_MANIFEST_DIR"));
        let source = std::fs::read_to_string(&path).unwrap();
        for needle in ["unicode", "UnicodeVersion", "is_py_", "_class(", "text::"] {
            assert!(!source.contains(needle), "{file} mentions {needle}");
        }
    }
}
