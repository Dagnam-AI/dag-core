//! The bounds of one near-duplicate batch, and the error that names the one it passes.
//!
//! The batch is not the caller's own: its size is another party's choice, so every size is bounded
//! before the memory it would need is allocated. The numbers and the messages are the reference's
//! (`dagnam_contracts.hygiene.near_dedup`), which reads the same limits on the rows as they were
//! sent (before any is normalised) and again on the normalised texts; this crate only ever sees
//! the normalised texts, so it holds the second reading. No message holds any text of the batch.
use std::fmt;

/// The most texts one batch holds.
pub const MAX_TEXTS: usize = 50_000;
/// The most characters in one text.
pub const MAX_TEXT_CHARS: usize = 1_000_000;
/// The most characters in all the texts of a batch together.
pub const MAX_TOTAL_CHARS: usize = 20_000_000;
/// The most candidate pairs (two texts that share an LSH band, each pair once) the walk compares,
/// whether or not they turn out to be near-duplicates.
pub const MAX_COMPARISONS: usize = 40_000_000;
/// The most near-duplicate pairs a batch may have: the result lists every one of them.
pub const MAX_PAIRS: usize = 20_000_000;

/// Why a batch is refused. The `Display` text is the reference's message for the same case; it
/// names the limit and its value and never holds a character of the batch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NearDedupError {
    /// The threshold is not in `(0, 1]` (NaN included).
    Threshold,
    /// More than [`MAX_TEXTS`] texts.
    TooManyTexts {
        /// How many there are.
        count: usize,
    },
    /// A text of more than [`MAX_TEXT_CHARS`] characters.
    TextTooLong {
        /// Its length in characters.
        chars: usize,
    },
    /// More than [`MAX_TOTAL_CHARS`] characters in all.
    TooManyChars,
    /// More candidate pairs than the walk may compare (see [`Limits`]).
    TooManyComparisons {
        /// The limit in force: [`MAX_COMPARISONS`] unless the call was given smaller [`Limits`].
        limit: usize,
    },
    /// More near-duplicate pairs than the batch may have (see [`Limits`]).
    TooManyPairs {
        /// The limit in force: [`MAX_PAIRS`] unless the call was given smaller [`Limits`].
        limit: usize,
    },
}

impl fmt::Display for NearDedupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Threshold => f.write_str("threshold must be in (0, 1]"),
            Self::TooManyTexts { count } => {
                write!(f, "{count} texts is more than MAX_TEXTS ({MAX_TEXTS})")
            }
            Self::TextTooLong { chars } => write!(
                f,
                "a text of {chars} characters is more than MAX_TEXT_CHARS ({MAX_TEXT_CHARS})"
            ),
            Self::TooManyChars => write!(
                f,
                "the texts hold more than MAX_TOTAL_CHARS ({MAX_TOTAL_CHARS}) characters"
            ),
            // The name is the reference's constant; the number is the limit in force, which the
            // reference prints when its constant is patched, as a test or a differential may do.
            Self::TooManyComparisons { limit } => {
                write!(f, "more than MAX_COMPARISONS ({limit}) comparisons")
            }
            Self::TooManyPairs { limit } => {
                write!(f, "more than MAX_PAIRS ({limit}) near-duplicate pairs")
            }
        }
    }
}

impl std::error::Error for NearDedupError {}

/// The two limits the walk enforces. `Limits::DEFAULT` is [`MAX_COMPARISONS`] and [`MAX_PAIRS`];
/// a smaller one lets a test, or a differential against the reference with its constants patched,
/// reach a limit without a batch of the full size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// The most candidate pairs compared: the call stops instead of making the next one.
    pub comparisons: usize,
    /// The most near-duplicate pairs kept: the call stops instead of keeping the next one.
    pub pairs: usize,
}

impl Limits {
    /// The limits of a batch: [`MAX_COMPARISONS`] and [`MAX_PAIRS`].
    pub const DEFAULT: Self = Self {
        comparisons: MAX_COMPARISONS,
        pairs: MAX_PAIRS,
    };
    /// No limit: for the second walk over a batch the first one has let through.
    pub(crate) const NONE: Self = Self {
        comparisons: usize::MAX,
        pairs: usize::MAX,
    };
}

/// Refuses a batch of more than [`MAX_TEXTS`] texts. Read before anything of the batch is.
///
/// # Errors
///
/// [`NearDedupError::TooManyTexts`].
pub const fn check_text_count(count: usize) -> Result<(), NearDedupError> {
    if count > MAX_TEXTS {
        return Err(NearDedupError::TooManyTexts { count });
    }
    Ok(())
}

/// Refuses a text of more than [`MAX_TEXT_CHARS`] characters, or texts of more than
/// [`MAX_TOTAL_CHARS`] together, from the lengths in characters, in the order of the texts: the
/// first text over a limit ends the read. The length of a Python `str` costs nothing, whatever
/// its size, so a binding reads these before it copies a text.
///
/// # Errors
///
/// [`NearDedupError::TextTooLong`], [`NearDedupError::TooManyChars`].
pub fn check_text_lengths(lengths: impl IntoIterator<Item = usize>) -> Result<(), NearDedupError> {
    let mut total = 0_usize;
    for chars in lengths {
        if chars > MAX_TEXT_CHARS {
            return Err(NearDedupError::TextTooLong { chars });
        }
        total += chars; // at most MAX_TEXTS * MAX_TEXT_CHARS: nowhere near overflowing
        if total > MAX_TOTAL_CHARS {
            return Err(NearDedupError::TooManyChars);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_numbers_are_the_references() {
        assert_eq!(
            (
                MAX_TEXTS,
                MAX_TEXT_CHARS,
                MAX_TOTAL_CHARS,
                MAX_COMPARISONS,
                MAX_PAIRS
            ),
            (50_000, 1_000_000, 20_000_000, 40_000_000, 20_000_000)
        );
        assert_eq!(Limits::DEFAULT.comparisons, MAX_COMPARISONS);
        assert_eq!(Limits::DEFAULT.pairs, MAX_PAIRS);
    }

    #[test]
    fn the_messages_are_the_references_and_hold_only_numbers() {
        let messages = [
            (NearDedupError::Threshold, "threshold must be in (0, 1]"),
            (
                NearDedupError::TooManyTexts { count: 50_001 },
                "50001 texts is more than MAX_TEXTS (50000)",
            ),
            (
                NearDedupError::TextTooLong { chars: 1_000_001 },
                "a text of 1000001 characters is more than MAX_TEXT_CHARS (1000000)",
            ),
            (
                NearDedupError::TooManyChars,
                "the texts hold more than MAX_TOTAL_CHARS (20000000) characters",
            ),
            (
                NearDedupError::TooManyComparisons {
                    limit: MAX_COMPARISONS,
                },
                "more than MAX_COMPARISONS (40000000) comparisons",
            ),
            (
                NearDedupError::TooManyPairs { limit: MAX_PAIRS },
                "more than MAX_PAIRS (20000000) near-duplicate pairs",
            ),
            (
                NearDedupError::TooManyComparisons { limit: 7 },
                "more than MAX_COMPARISONS (7) comparisons",
            ),
            (
                NearDedupError::TooManyPairs { limit: 3 },
                "more than MAX_PAIRS (3) near-duplicate pairs",
            ),
        ];
        for (error, message) in messages {
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn the_count_is_refused_one_past_the_limit() {
        assert_eq!(check_text_count(MAX_TEXTS), Ok(()));
        assert_eq!(
            check_text_count(MAX_TEXTS + 1),
            Err(NearDedupError::TooManyTexts {
                count: MAX_TEXTS + 1
            })
        );
    }

    #[test]
    fn a_text_is_refused_one_past_the_limit_and_the_total_one_past_its_own() {
        assert_eq!(check_text_lengths([]), Ok(()));
        assert_eq!(check_text_lengths([MAX_TEXT_CHARS]), Ok(()));
        assert_eq!(
            check_text_lengths([MAX_TEXT_CHARS + 1]),
            Err(NearDedupError::TextTooLong {
                chars: MAX_TEXT_CHARS + 1
            })
        );
        let whole = MAX_TOTAL_CHARS / MAX_TEXT_CHARS;
        assert_eq!(check_text_lengths(vec![MAX_TEXT_CHARS; whole]), Ok(()));
        assert_eq!(
            check_text_lengths(vec![MAX_TEXT_CHARS; whole].into_iter().chain([1])),
            Err(NearDedupError::TooManyChars)
        );
    }

    #[test]
    fn the_first_text_over_a_limit_ends_the_read() {
        // The long text comes before the total is passed, and a text after it is never read: the
        // count of the texts taken from the iterator proves that.
        let taken = std::cell::Cell::new(0_usize);
        let lengths = [1, MAX_TEXT_CHARS + 1, 1, 1].into_iter().inspect(|_| {
            taken.set(taken.get() + 1);
        });
        assert_eq!(
            check_text_lengths(lengths),
            Err(NearDedupError::TextTooLong {
                chars: MAX_TEXT_CHARS + 1
            })
        );
        assert_eq!(taken.get(), 2);
    }
}
