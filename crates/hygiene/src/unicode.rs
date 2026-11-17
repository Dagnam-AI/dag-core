//! Python's character classes, one table per Unicode version.
//!
//! Python's `\w`, `\s` and `\d` are not the `regex` crate's (another Unicode version, other
//! members) and not Rust's `char` methods (`is_alphanumeric` counts combining marks,
//! `is_whitespace` lacks U+001C to U+001F, `is_numeric` counts every number). They are not even
//! one set: they follow the Unicode database of the interpreter that runs them, and `CPython`
//! 3.12 (Unicode 15.0.0), 3.13 (15.1.0) and 3.14 (16.0.0) classify a few thousand code points
//! differently. So the core ships all three, each written by its own interpreter into
//! `data/py_classes_<major>_<minor>.txt` by `scripts/gen_py_classes.py`, and every answer here
//! is for one [`UnicodeVersion`] that the caller names. The predicates and the regex-syntax
//! spellings of a version read the same ranges, so a pattern and a character test in code
//! cannot disagree.
use std::fmt::Write as _;
use std::sync::OnceLock;

use crate::table::{Table, parse};

/// A Unicode version whose class tables the core ships: the one the interpreter that runs the
/// reference reports as `unicodedata.unidata_version`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnicodeVersion {
    /// Unicode 15.0.0: `CPython` 3.12.
    V15_0,
    /// Unicode 15.1.0: `CPython` 3.13.
    V15_1,
    /// Unicode 16.0.0: `CPython` 3.14, free-threaded or not.
    V16_0,
}

impl UnicodeVersion {
    /// Every shipped version, oldest first.
    pub const ALL: [Self; 3] = [Self::V15_0, Self::V15_1, Self::V16_0];

    /// The version as `unicodedata.unidata_version` spells it (`"15.1.0"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V15_0 => "15.0.0",
            Self::V15_1 => "15.1.0",
            Self::V16_0 => "16.0.0",
        }
    }

    /// The version `unidata_version` names, or `None` for one the core has no tables for.
    #[must_use]
    pub fn from_unidata(unidata_version: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|version| version.as_str() == unidata_version)
    }

    fn classes(self) -> &'static Classes {
        match self {
            Self::V15_0 => &V15_0,
            Self::V15_1 => &V15_1,
            Self::V16_0 => &V16_0,
        }
    }
}

/// One value per [`UnicodeVersion`], built on first use: the patterns and class spellings of a
/// version are only paid for when a text is read under it.
pub(crate) struct Versioned<T> {
    v15_0: OnceLock<T>,
    v15_1: OnceLock<T>,
    v16_0: OnceLock<T>,
    build: fn(UnicodeVersion) -> T,
}

impl<T> Versioned<T> {
    /// A value that `build` makes for each version the first time it is asked for.
    pub(crate) const fn new(build: fn(UnicodeVersion) -> T) -> Self {
        Self {
            v15_0: OnceLock::new(),
            v15_1: OnceLock::new(),
            v16_0: OnceLock::new(),
            build,
        }
    }

    /// The value for `version`.
    pub(crate) fn get(&self, version: UnicodeVersion) -> &T {
        let cell = match version {
            UnicodeVersion::V15_0 => &self.v15_0,
            UnicodeVersion::V15_1 => &self.v15_1,
            UnicodeVersion::V16_0 => &self.v16_0,
        };
        cell.get_or_init(|| (self.build)(version))
    }
}

/// The three classes of one version.
struct Classes {
    word: Table,
    space: Table,
    digit: Table,
}

impl Classes {
    /// Parsed by the compiler: a malformed table is a build error (see `table`).
    const fn parse(table: &str) -> Self {
        Self {
            word: parse(table, "word"),
            space: parse(table, "space"),
            digit: parse(table, "digit"),
        }
    }
}

static V15_0: Classes = Classes::parse(include_str!("../data/py_classes_15_0.txt"));
static V15_1: Classes = Classes::parse(include_str!("../data/py_classes_15_1.txt"));
static V16_0: Classes = Classes::parse(include_str!("../data/py_classes_16_0.txt"));

fn contains(ranges: &[(u32, u32)], c: char) -> bool {
    let code = u32::from(c);
    let after = ranges.partition_point(|&(_, high)| high < code);
    ranges.get(after).is_some_and(|&(low, _)| low <= code)
}

/// Python's `\w`: `str.isalnum()` or `_`, under `version`'s Unicode database.
#[must_use]
pub fn is_py_word(version: UnicodeVersion, c: char) -> bool {
    contains(version.classes().word.as_slice(), c)
}

/// Python's `\s`: `str.isspace()`, Unicode `White_Space` plus U+001C to U+001F.
#[must_use]
pub fn is_py_space(version: UnicodeVersion, c: char) -> bool {
    contains(version.classes().space.as_slice(), c)
}

/// Python's `\d`: general category `Nd`, under `version`'s Unicode database.
#[must_use]
pub fn is_py_digit(version: UnicodeVersion, c: char) -> bool {
    contains(version.classes().digit.as_slice(), c)
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

static WORD_CLASS: Versioned<String> =
    Versioned::new(|version| class(version.classes().word.as_slice()));
static SPACE_CLASS: Versioned<String> =
    Versioned::new(|version| class(version.classes().space.as_slice()));
static DIGIT_CLASS: Versioned<String> =
    Versioned::new(|version| class(version.classes().digit.as_slice()));

/// `regex` syntax for [`is_py_word`]: `[\u{30}-\u{39}...]`, no `\w`.
///
/// Built from the compile-time table on first use. Python's `\W` is the class negated and
/// nested: `[^[...]]`. Under `(?i)` wrap it with [`crate::case_sensitive`].
#[must_use]
pub fn word_class(version: UnicodeVersion) -> &'static str {
    WORD_CLASS.get(version)
}

/// `regex` syntax for [`is_py_space`]; see [`word_class`].
#[must_use]
pub fn space_class(version: UnicodeVersion) -> &'static str {
    SPACE_CLASS.get(version)
}

/// `regex` syntax for [`is_py_digit`]; see [`word_class`].
#[must_use]
pub fn digit_class(version: UnicodeVersion) -> &'static str {
    DIGIT_CLASS.get(version)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{Classes, UnicodeVersion, Versioned, parse};

    const TABLES: [&str; 3] = [
        include_str!("../data/py_classes_15_0.txt"),
        include_str!("../data/py_classes_15_1.txt"),
        include_str!("../data/py_classes_16_0.txt"),
    ];

    #[test]
    fn the_shipped_tables_read_the_same_with_windows_line_endings() {
        for (version, table) in UnicodeVersion::ALL.into_iter().zip(TABLES) {
            let crlf = table.replace("\r\n", "\n").replace('\n', "\r\n");
            let classes = version.classes();
            for (name, shipped) in [
                ("word", &classes.word),
                ("space", &classes.space),
                ("digit", &classes.digit),
            ] {
                assert_eq!(parse(&crlf, name).as_slice(), shipped.as_slice(), "{name}");
                assert!(!shipped.as_slice().is_empty(), "{name}");
            }
        }
    }

    #[test]
    fn each_version_has_its_own_file() {
        for (version, table) in UnicodeVersion::ALL.into_iter().zip(TABLES) {
            let unicode = version.as_str();
            let header = format!("Unicode {unicode}.");
            assert!(table.lines().next().unwrap().contains(&header), "{unicode}");
            let (read, shipped) = (Classes::parse(table), version.classes());
            assert_eq!(read.word.as_slice(), shipped.word.as_slice(), "{unicode}");
            assert_eq!(read.space.as_slice(), shipped.space.as_slice(), "{unicode}");
            assert_eq!(read.digit.as_slice(), shipped.digit.as_slice(), "{unicode}");
        }
    }

    #[test]
    fn a_versioned_value_is_built_once_per_version_on_first_use() {
        static BUILT: AtomicUsize = AtomicUsize::new(0);
        let value: Versioned<String> = Versioned::new(|version| {
            BUILT.fetch_add(1, Ordering::Relaxed);
            format!("built for {}", version.as_str())
        });
        assert_eq!(BUILT.load(Ordering::Relaxed), 0);
        for version in UnicodeVersion::ALL {
            let expected = format!("built for {}", version.as_str());
            assert_eq!(value.get(version), &expected);
            assert_eq!(value.get(version), &expected);
        }
        assert_eq!(BUILT.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn a_version_is_read_back_from_its_spelling_and_nothing_else() {
        for version in UnicodeVersion::ALL {
            assert_eq!(
                UnicodeVersion::from_unidata(version.as_str()),
                Some(version)
            );
        }
        for unknown in [
            "", "15", "15.0", "15.0.1", "14.0.0", "17.0.0", " 15.1.0", "15.1.0\n",
        ] {
            assert_eq!(UnicodeVersion::from_unidata(unknown), None, "{unknown:?}");
        }
    }
}
