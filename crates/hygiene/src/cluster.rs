//! Near-duplicate clustering: shingling, banded LSH candidates, verified by the full signature.
use crate::candidates::Candidates;
use crate::limits::{Limits, NearDedupError, check_text_count, check_text_lengths};
use crate::minhash::{NUM_PERMUTATIONS, minhash_signature};

/// Words per shingle; a text with fewer words shingles to its words.
pub const SHINGLE_SIZE: usize = 3;
/// `NUM_PERMUTATIONS` as the divisor of the Jaccard estimate.
const WIDTH: f64 = 128.0;
const _: () = assert!(NUM_PERMUTATIONS == 128);

/// One clustered pair: `(kept_index, duplicate_index, similarity)`.
pub type Pair = (usize, usize, f64);

pub(crate) type Signature = [u64; NUM_PERMUTATIONS];

/// The word shingles of a text.
///
/// # Input
///
/// The text must already be normalised exactly like the Python `row_text` output: casefolded,
/// punctuation removed, a single U+0020 between words. Only U+0020 separates words here;
/// Python's `split()` separates on any whitespace, so unnormalised input clusters differently.
fn shingles(text: &str) -> Vec<String> {
    let words: Vec<&str> = text.split(' ').filter(|word| !word.is_empty()).collect();
    if words.len() < SHINGLE_SIZE {
        return words.iter().map(ToString::to_string).collect();
    }
    words.windows(SHINGLE_SIZE).map(|w| w.join(" ")).collect()
}

/// The signature of a normalised text.
pub(crate) fn sign(text: &str) -> Signature {
    minhash_signature(shingles(text).iter().map(String::as_str))
}

/// The root of `index`, halving the path on the way up, as the reference does.
///
/// A path holds at most `parent.len()` nodes, which is all the loop is bounded by: it reaches
/// the root by the last iteration at the latest. The bound keeps a wrong root test from spinning forever.
#[allow(clippy::indexing_slicing)] // `index` and every `parent` value are indices into `parent`
fn find(parent: &mut [usize], mut index: usize) -> usize {
    for _ in 0..parent.len() {
        if parent[index] == index {
            break;
        }
        parent[index] = parent[parent[index]];
        index = parent[index];
    }
    index
}

/// Joins the clusters of `low` and `high`, the higher root going under the lower, so that a
/// cluster's root is its lowest index (the row that is kept).
#[allow(clippy::indexing_slicing)] // `find` returns an index into `parent`
fn union(parent: &mut [usize], low: usize, high: usize) {
    let (root_low, root_high) = (find(parent, low), find(parent, high));
    parent[root_low.max(root_high)] = root_low.min(root_high);
}

/// The fraction of signature positions at which two texts agree.
#[allow(clippy::indexing_slicing)] // `low` and `high` come from candidates, below `signatures.len()`
fn similarity(signatures: &[Signature], low: usize, high: usize) -> f64 {
    let agree: u32 = signatures[low]
        .iter()
        .zip(&signatures[high])
        .map(|(a, b)| u32::from(a == b))
        .sum();
    f64::from(agree) / WIDTH
}

/// The walk over the candidate pairs, in the reference's order: each pair counted before it is
/// compared, so the walk stops instead of making comparison `limits.comparisons + 1`, and each
/// pair at or over the threshold counted before it is kept, so it stops instead of keeping pair
/// `limits.pairs + 1`. After an `Err` the walk is not read again.
struct Walk {
    candidates: Candidates,
    threshold: f64,
    limits: Limits,
    comparisons: usize,
    found: usize,
}

impl Walk {
    fn new(signatures: &[Signature], threshold: f64, limits: Limits) -> Self {
        Self {
            candidates: Candidates::new(signatures),
            threshold,
            limits,
            comparisons: 0,
            found: 0,
        }
    }

    /// The next verified pair, the limit the walk passes, or `None` at its end.
    fn step(&mut self, signatures: &[Signature]) -> Option<Result<Pair, NearDedupError>> {
        loop {
            let (low, high) = self.candidates.next()?;
            self.comparisons += 1;
            if self.comparisons > self.limits.comparisons {
                return Some(Err(NearDedupError::TooManyComparisons {
                    limit: self.limits.comparisons,
                }));
            }
            let score = similarity(signatures, low, high);
            if score < self.threshold {
                continue;
            }
            self.found += 1;
            if self.found > self.limits.pairs {
                return Some(Err(NearDedupError::TooManyPairs {
                    limit: self.limits.pairs,
                }));
            }
            return Some(Ok((low, high, score)));
        }
    }
}

/// The most pairs the Rust result reserves room for before it holds any: a batch of `usize::MAX`
/// limits could name far more than memory has, and the vector grows past this as it must.
const RESERVE: usize = 1 << 20;

/// A batch that has passed its checks and its limits, its pairs not yet read.
///
/// [`Batch::new`] runs the checks of [`near_duplicates_within`] and counts the pairs, holding
/// nothing of them. The pairs are then read in chunks with [`Batch::fill`], so a caller that must
/// convert them (a binding making Python objects) never holds all of them in Rust as well.
pub struct Batch {
    signatures: Vec<Signature>,
    walk: Walk,
    parent: Vec<usize>,
    pair_count: usize,
}

impl Batch {
    /// The checks of [`near_duplicates_within`]: the threshold, the sizes, and the walk's limits,
    /// the last by walking once to count.
    ///
    /// # Errors
    ///
    /// A [`NearDedupError`]: the threshold, or the first limit the batch passes.
    pub fn new(texts: &[&str], threshold: f64, limits: Limits) -> Result<Self, NearDedupError> {
        // Written as a conjunction so that NaN, which compares false, is refused too.
        let valid = threshold > 0.0 && threshold <= 1.0;
        if !valid {
            return Err(NearDedupError::Threshold);
        }
        check_text_count(texts.len())?;
        check_text_lengths(texts.iter().map(|text| text.chars().count()))?;
        let signatures: Vec<Signature> = texts.iter().map(|text| sign(text)).collect();
        let mut counting = Walk::new(&signatures, threshold, limits);
        let mut pair_count = 0_usize;
        while let Some(pair) = counting.step(&signatures) {
            pair?;
            pair_count += 1;
        }
        Ok(Self {
            walk: Walk::new(&signatures, threshold, Limits::NONE),
            parent: (0..texts.len()).collect(),
            signatures,
            pair_count,
        })
    }

    /// How many pairs the batch has: what [`Batch::fill`] will add up to, in all.
    #[must_use]
    pub const fn pair_count(&self) -> usize {
        self.pair_count
    }

    /// Appends the next `max` pairs, or the rest of them if fewer, to `out`, in the order of the
    /// walk, and returns how many it appended (none when they are all read).
    pub fn fill(&mut self, out: &mut Vec<Pair>, max: usize) -> usize {
        let before = out.len();
        // No limit on this walk, so every item is an `Ok`: the counting walk found what this does.
        while out.len() - before < max
            && let Some(Ok(pair)) = self.walk.step(&self.signatures)
        {
            union(&mut self.parent, pair.0, pair.1);
            out.push(pair);
        }
        out.len() - before
    }

    /// The indices of every text that duplicates an earlier one. Read after the last pair.
    #[must_use]
    pub fn duplicates(mut self) -> Vec<usize> {
        (0..self.parent.len())
            .filter(|&index| find(&mut self.parent, index) != index)
            .collect()
    }
}

/// Clusters normalised texts: the indices of every text that duplicates an earlier one,
/// and each verified pair. See [`near_duplicates_within`] for the checks and their order.
///
/// A cluster's representative is its lowest index, which is never reported as a duplicate.
///
/// # Input
///
/// Every text must already be normalised exactly like the Python `row_text` output:
/// casefolded, punctuation removed, a single U+0020 between words. Only U+0020 separates
/// words here (Python's `split()` separates on any whitespace), so unnormalised input yields
/// different clusters than the reference.
///
/// # Errors
///
/// A [`NearDedupError`]: the threshold, or the limit the batch passes.
pub fn near_duplicates(
    texts: &[&str],
    threshold: f64,
) -> Result<(Vec<usize>, Vec<Pair>), NearDedupError> {
    near_duplicates_within(texts, threshold, Limits::DEFAULT)
}

/// [`near_duplicates`] under the given [`Limits`].
///
/// The checks run in the reference's order, each before the memory the next step needs:
///
/// 1. the threshold, which must be in `(0, 1]`;
/// 2. the number of texts, then each text's length in characters and the running total, text by
///    text (this crate reads the normalised texts, the second reading of the reference's size
///    limits; a caller that has the raw rows reads the first before it normalises anything);
/// 3. the walk over the candidate pairs, counting comparisons and near-duplicate pairs. The walk
///    is run once to count, holding nothing, and again only when the batch is inside the limits,
///    to build the result, which has room for its pairs at once (up to a fixed number of them).
///
/// # Errors
///
/// A [`NearDedupError`]: the threshold, or the first limit the batch passes.
pub fn near_duplicates_within(
    texts: &[&str],
    threshold: f64,
    limits: Limits,
) -> Result<(Vec<usize>, Vec<Pair>), NearDedupError> {
    let mut batch = Batch::new(texts, threshold, limits)?;
    let mut pairs = Vec::with_capacity(batch.pair_count().min(RESERVE));
    while batch.fill(&mut pairs, RESERVE) > 0 {}
    Ok((batch.duplicates(), pairs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_halves_the_path_it_walks() {
        // 4 -> 3 -> 2 -> 1 -> 0: each step points the node at its grandparent.
        let mut parent = vec![0, 0, 1, 2, 3];
        assert_eq!(find(&mut parent, 4), 0);
        assert_eq!(parent, [0, 0, 0, 2, 2]);
        assert_eq!(find(&mut parent, 4), 0);
        assert_eq!(parent, [0, 0, 0, 2, 0]);
    }

    #[test]
    fn find_of_a_root_changes_nothing() {
        let mut parent = vec![0, 1, 1];
        assert_eq!(find(&mut parent, 1), 1);
        assert_eq!(parent, [0, 1, 1]);
    }

    #[test]
    fn union_puts_the_higher_root_under_the_lower() {
        let mut parent = vec![0, 1, 2, 3];
        union(&mut parent, 3, 1);
        assert_eq!(parent, [0, 1, 2, 1]);
        union(&mut parent, 2, 3);
        assert_eq!(parent, [0, 1, 1, 1]);
        union(&mut parent, 1, 0);
        assert_eq!(parent, [0, 0, 1, 1]);
        // Already one cluster: only the halving of the two paths moves anything.
        union(&mut parent, 3, 2);
        assert_eq!(parent, [0, 0, 0, 0]);
    }

    #[test]
    fn shingles_split_on_single_spaces_only() {
        assert_eq!(shingles("").len(), 0);
        assert_eq!(shingles("a b"), ["a", "b"]);
        assert_eq!(shingles("a b c d"), ["a b c", "b c d"]);
        assert_eq!(shingles("a\tb c"), ["a\tb", "c"]);
    }
}
