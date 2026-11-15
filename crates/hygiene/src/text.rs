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

/// Inclusive code-point ranges, ascending, disjoint and never adjacent.
type Ranges = Vec<(u32, u32)>;

const TABLE: &str = include_str!("../data/py_classes.txt");

/// The ranges of one `[section]` of the table: every hex number after the header, two to a range.
fn section(name: &str) -> Ranges {
    let body = TABLE
        .split('[')
        .find_map(|part| part.strip_prefix(name)?.strip_prefix("]\n"))
        .unwrap_or_default();
    let numbers = body
        .split(|c: char| !c.is_ascii_hexdigit())
        .filter_map(|digits| u32::from_str_radix(digits, 16).ok());
    let lows = numbers.clone().step_by(2);
    let highs = numbers.skip(1).step_by(2);
    lows.zip(highs).collect()
}

static WORD: LazyLock<Ranges> = LazyLock::new(|| section("word"));
static SPACE: LazyLock<Ranges> = LazyLock::new(|| section("space"));
static DIGIT: LazyLock<Ranges> = LazyLock::new(|| section("digit"));

fn contains(ranges: &[(u32, u32)], c: char) -> bool {
    let code = u32::from(c);
    let after = ranges.partition_point(|&(_, high)| high < code);
    ranges.get(after).is_some_and(|&(low, _)| low <= code)
}

/// Python's `\w`: `str.isalnum()` or `_` (Unicode 15.1, as the reference interpreter has it).
#[must_use]
pub fn is_py_word(c: char) -> bool {
    contains(&WORD, c)
}

/// Python's `\s`: `str.isspace()`, Unicode `White_Space` plus U+001C to U+001F.
#[must_use]
pub fn is_py_space(c: char) -> bool {
    contains(&SPACE, c)
}

/// Python's `\d`: general category `Nd`.
#[must_use]
pub fn is_py_digit(c: char) -> bool {
    contains(&DIGIT, c)
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
pub static WORD_CLASS: LazyLock<String> = LazyLock::new(|| class(&WORD));
/// `regex` syntax for [`is_py_space`]; see [`WORD_CLASS`].
pub static SPACE_CLASS: LazyLock<String> = LazyLock::new(|| class(&SPACE));
/// `regex` syntax for [`is_py_digit`]; see [`WORD_CLASS`].
pub static DIGIT_CLASS: LazyLock<String> = LazyLock::new(|| class(&DIGIT));

/// `class` with case folding off: `(?-i:class)`.
///
/// Python's `\w` ignores the `(?i)` flag, but a bare class under it also matches every code
/// point that folds to a member (U+0345 folds to a Greek letter), so a class used inside a
/// `(?i)` pattern goes through this.
#[must_use]
pub fn case_sensitive(class: &str) -> String {
    format!("(?-i:{class})")
}

/// The character that ends at byte `at` of `text`, for "not preceded by" checks.
///
/// `None` when `at` is 0, past the end, or inside a character.
#[must_use]
pub fn char_before(text: &str, at: usize) -> Option<char> {
    text.get(..at).and_then(|before| before.chars().next_back())
}

/// The character that starts at byte `at` of `text`.
///
/// `None` when `at` is the end of the text, past it, or inside a character.
#[must_use]
pub fn char_at(text: &str, at: usize) -> Option<char> {
    text.get(at..).and_then(|rest| rest.chars().next())
}

/// Converts ascending byte offsets into `text` to code-point offsets in one pass.
///
/// An offset's result is the number of characters that start before it, so a boundary maps to
/// the index of the character there, an offset inside a character counts that character (it
/// rounds up to the next boundary) and one past the end maps to the length in characters. An
/// offset below its predecessor gets the predecessor's result. Nothing panics.
#[must_use]
pub fn to_char_offsets(text: &str, bytes: &[usize]) -> Vec<usize> {
    let mut starts = text.char_indices().map(|(start, _)| start).peekable();
    let mut passed = 0;
    bytes
        .iter()
        .map(|&at| {
            while starts.next_if(|&start| start < at).is_some() {
                passed += 1;
            }
            passed
        })
        .collect()
}
