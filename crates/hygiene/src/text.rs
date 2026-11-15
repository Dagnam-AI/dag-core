//! Python's character classes and code-point offsets.
//!
//! Python's `\w`, `\s` and `\d` are not the `regex` crate's (another Unicode version, other
//! members) and not Rust's `char` methods (`is_alphanumeric` counts combining marks,
//! `is_whitespace` lacks U+001C to U+001F, `is_numeric` counts every number). Every answer
//! here comes from one table, `data/py_classes.txt`, written by `scripts/gen_py_classes.py`
//! from the reference interpreter; the predicates and the regex-syntax spellings read the
//! same ranges, so a pattern and a character test in code cannot disagree.
use std::fmt::Write as _;
use std::sync::LazyLock;

use crate::table::{Table, parse};

const TABLE: &str = include_str!("../data/py_classes.txt");

// Parsed by the compiler: a malformed table is a build error (see `table`).
static WORD: Table = parse(TABLE, "word");
static SPACE: Table = parse(TABLE, "space");
static DIGIT: Table = parse(TABLE, "digit");

fn contains(ranges: &[(u32, u32)], c: char) -> bool {
    let code = u32::from(c);
    let after = ranges.partition_point(|&(_, high)| high < code);
    ranges.get(after).is_some_and(|&(low, _)| low <= code)
}

/// Python's `\w`: `str.isalnum()` or `_` (Unicode 15.1, as the reference interpreter has it).
#[must_use]
pub fn is_py_word(c: char) -> bool {
    contains(WORD.as_slice(), c)
}

/// Python's `\s`: `str.isspace()`, Unicode `White_Space` plus U+001C to U+001F.
#[must_use]
pub fn is_py_space(c: char) -> bool {
    contains(SPACE.as_slice(), c)
}

/// Python's `\d`: general category `Nd`.
#[must_use]
pub fn is_py_digit(c: char) -> bool {
    contains(DIGIT.as_slice(), c)
}

/// The members of `ranges` as a `regex` bracketed class of explicit `\u{..}` ranges.
fn class(ranges: &[(u32, u32)]) -> String {
    let body = ranges
        .iter()
        .fold(String::from("["), |mut class, (low, high)| {
            // Writing to a `String` cannot fail.
            let _ = write!(class, "\\u{{{low:x}}}-\\u{{{high:x}}}");
            class
        });
    body + "]"
}

/// `regex` syntax for [`is_py_word`]: `[\u{30}-\u{39}...]`, no `\w`.
///
/// A static built on first use, because the ranges are parsed from the table at run time; it
/// derefs to a `&str`. Python's `\W` is the class negated and nested: `[^[...]]`. Under `(?i)`
/// wrap it with [`case_sensitive`].
pub static WORD_CLASS: LazyLock<String> = LazyLock::new(|| class(WORD.as_slice()));
/// `regex` syntax for [`is_py_space`]; see [`WORD_CLASS`].
pub static SPACE_CLASS: LazyLock<String> = LazyLock::new(|| class(SPACE.as_slice()));
/// `regex` syntax for [`is_py_digit`]; see [`WORD_CLASS`].
pub static DIGIT_CLASS: LazyLock<String> = LazyLock::new(|| class(DIGIT.as_slice()));

/// `class` with case folding off: `(?-i:class)`.
///
/// Python's `\w` ignores the `(?i)` flag, but a bare class under it also matches every code
/// point that folds to a member (U+0345 folds to a Greek letter), so a class used inside a
/// `(?i)` pattern goes through this.
///
/// Wrap a whole standalone class or alternative, never part of a bracket class: inside
/// `[...]` the text `(?-i:X)` is not a group, it is the characters `(`, `?`, `-` and so on, and
/// the result compiles but means something else. A class that mixes the word class with other
/// members is an alternation: `(?:` + `case_sensitive(&WORD_CLASS)` + `|[.+-])`.
#[must_use]
pub fn case_sensitive(class: &str) -> String {
    format!("(?-i:{class})")
}

/// The character that ends at byte `at` of `text`, for "not preceded by" checks.
///
/// **`None` also when `at` is inside a character**, not only at the start of the text or past
/// its end: an offset that is not on a character boundary has no character before it. Offsets
/// that come from `regex` matches are always boundaries.
#[must_use]
pub fn char_before(text: &str, at: usize) -> Option<char> {
    text.get(..at).and_then(|before| before.chars().next_back())
}

/// The character that starts at byte `at` of `text`.
///
/// **`None` also when `at` is inside a character**, not only at the end of the text or past
/// it: an offset that is not on a character boundary has no character starting there.
#[must_use]
pub fn char_at(text: &str, at: usize) -> Option<char> {
    text.get(at..).and_then(|rest| rest.chars().next())
}

/// `bytes` as code-point offsets, for offsets in ascending order: one pass over `text`.
fn count_passed(text: &str, bytes: impl Iterator<Item = usize>) -> impl Iterator<Item = usize> {
    let mut starts = text.char_indices().map(|(start, _)| start).peekable();
    let mut passed = 0;
    bytes.map(move |at| {
        while starts.next_if(|&start| start < at).is_some() {
            passed += 1;
        }
        passed
    })
}

/// Converts byte offsets into `text` to code-point offsets, one answer per offset, in the
/// order given (the offsets need not be sorted, and may repeat, as the ends of overlapping
/// spans do).
///
/// An offset's result is the number of characters that start before it, so a boundary maps to
/// the index of the character there, an offset inside a character counts that character (it
/// rounds up to the next boundary) and one past the end maps to the length in characters.
/// Nothing panics.
///
/// The offsets are stably sorted by value and answered in one pass over the text, then put
/// back in input order. The standard stable sort is linear on input that is already ascending
/// (the usual case: the offsets of findings in order), so that case is one pass plus two
/// linear scans, and any other order costs `O(n log n)` in the number of offsets.
#[must_use]
pub fn to_char_offsets(text: &str, bytes: &[usize]) -> Vec<usize> {
    let mut by_value: Vec<(usize, usize)> = bytes.iter().copied().enumerate().collect();
    by_value.sort_by_key(|&(_, at)| at);
    let counts = count_passed(text, by_value.iter().map(|&(_, at)| at));
    let mut by_index: Vec<(usize, usize)> = by_value
        .iter()
        .map(|&(index, _)| index)
        .zip(counts)
        .collect();
    by_index.sort_by_key(|&(index, _)| index);
    by_index.into_iter().map(|(_, count)| count).collect()
}

#[cfg(test)]
mod tests {
    use super::{DIGIT, SPACE, TABLE, WORD, parse};

    #[test]
    fn the_shipped_table_reads_the_same_with_windows_line_endings() {
        let crlf = TABLE.replace("\r\n", "\n").replace('\n', "\r\n");
        for (name, table) in [("word", &WORD), ("space", &SPACE), ("digit", &DIGIT)] {
            assert_eq!(parse(&crlf, name).as_slice(), table.as_slice(), "{name}");
            assert!(!table.as_slice().is_empty(), "{name}");
        }
    }
}
