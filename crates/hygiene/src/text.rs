//! Case-folding wrappers for class spellings and code-point offsets.
//!
//! The character classes themselves (Python's `\w`, `\s`, `\d`, one table per Unicode version)
//! live in [`crate::unicode`].

/// `class` with case folding off: `(?-i:class)`.
///
/// Python's `\w` ignores the `(?i)` flag, but a bare class under it also matches every code
/// point that folds to a member (U+0345 folds to a Greek letter), so a class used inside a
/// `(?i)` pattern goes through this.
///
/// Wrap a whole standalone class or alternative, never part of a bracket class: inside
/// `[...]` the text `(?-i:X)` is not a group, it is the characters `(`, `?`, `-` and so on, and
/// the result compiles but means something else. A class that mixes the word class with other
/// members is an alternation: `(?:` + `case_sensitive(word_class(version))` + `|[.+-])`.
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
