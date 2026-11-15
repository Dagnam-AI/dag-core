//! Near-duplicate clustering: shingling, banded LSH candidates, verified by the full signature.
use std::collections::{BTreeSet, HashMap};

use crate::minhash::{NUM_PERMUTATIONS, minhash_signature};

/// Words per shingle; a text with fewer words shingles to its words.
pub const SHINGLE_SIZE: usize = 3;
/// LSH bands.
pub const BANDS: usize = 32;
/// Signature positions in one band.
const ROWS_PER_BAND: usize = NUM_PERMUTATIONS / BANDS;
/// `NUM_PERMUTATIONS` as the divisor of the Jaccard estimate.
const WIDTH: f64 = 128.0;
const _: () = assert!(NUM_PERMUTATIONS == 128 && BANDS * ROWS_PER_BAND == NUM_PERMUTATIONS);

/// One clustered pair: `(kept_index, duplicate_index, similarity)`.
pub type Pair = (usize, usize, f64);

type Signature = [u64; NUM_PERMUTATIONS];

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

#[allow(clippy::indexing_slicing)] // `band < BANDS`, and a signature holds exactly BANDS chunks
fn band_key(signature: &Signature, band: usize) -> &[u64; ROWS_PER_BAND] {
    &signature.as_chunks::<ROWS_PER_BAND>().0[band]
}

/// Index pairs `(low, high)` that share at least one LSH band, in ascending order.
///
/// One band at a time, so only one bucket map is alive at once.
fn candidate_pairs(signatures: &[Signature]) -> BTreeSet<(usize, usize)> {
    let mut candidates = BTreeSet::new();
    for band in 0..BANDS {
        let mut buckets: HashMap<&[u64; ROWS_PER_BAND], Vec<usize>> = HashMap::new();
        for (index, signature) in signatures.iter().enumerate() {
            buckets
                .entry(band_key(signature, band))
                .or_default()
                .push(index);
        }
        for members in buckets.values() {
            for (at, &low) in members.iter().enumerate() {
                candidates.extend(members.iter().skip(at + 1).map(|&high| (low, high)));
            }
        }
    }
    candidates
}

/// The root of `index`, halving the path on the way up, as the reference does.
///
/// A path holds at most `parent.len()` nodes, which is all the loop is bounded by: it ends at
/// the root long before. The bound keeps a wrong root test from spinning forever.
#[allow(clippy::indexing_slicing)] // every `parent` value is an index into `parent`
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

/// Clusters normalised texts: the indices of every text that duplicates an earlier one,
/// and each verified pair. `None` when `threshold` is outside `(0, 1]`.
///
/// A cluster's representative is its lowest index, which is never reported as a duplicate.
///
/// # Input
///
/// Every text must already be normalised exactly like the Python `row_text` output:
/// casefolded, punctuation removed, a single U+0020 between words. Only U+0020 separates
/// words here (Python's `split()` separates on any whitespace), so unnormalised input yields
/// different clusters than the reference.
#[must_use]
pub fn near_duplicates(texts: &[&str], threshold: f64) -> Option<(Vec<usize>, Vec<Pair>)> {
    // Written as a conjunction so that NaN, which compares false, is refused too.
    let valid = threshold > 0.0 && threshold <= 1.0;
    if !valid {
        return None;
    }
    let signatures: Vec<Signature> = texts
        .iter()
        .map(|text| minhash_signature(shingles(text).iter().map(String::as_str)))
        .collect();
    let mut parent: Vec<usize> = (0..texts.len()).collect();
    let mut pairs = Vec::new();
    for (low, high) in candidate_pairs(&signatures) {
        let score = similarity(&signatures, low, high);
        if score >= threshold {
            pairs.push((low, high, score));
            union(&mut parent, low, high);
        }
    }
    let duplicates = (0..texts.len())
        .filter(|&index| find(&mut parent, index) != index)
        .collect();
    Some((duplicates, pairs))
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
    fn candidates_are_the_pairs_that_share_any_band() {
        let texts = [
            "a b c d e f",
            "a b c d e f",
            "a b c d e g",
            "x y z w v u",
            "",
            "",
            "a b",
            "b a",
            "p q r s t u v w",
            "p q r s t u v x",
        ];
        let signatures: Vec<Signature> = texts
            .iter()
            .map(|t| minhash_signature(shingles(t).iter().map(String::as_str)))
            .collect();
        let expected: BTreeSet<(usize, usize)> = (0..texts.len())
            .flat_map(|low| ((low + 1)..texts.len()).map(move |high| (low, high)))
            .filter(|&(low, high)| {
                (0..BANDS).any(|band| {
                    band_key(&signatures[low], band) == band_key(&signatures[high], band)
                })
            })
            .collect();
        assert!(expected.contains(&(0, 1)) && expected.contains(&(4, 5)));
        assert!(!expected.contains(&(0, 3)));
        assert_eq!(candidate_pairs(&signatures), expected);
    }

    #[test]
    fn shingles_split_on_single_spaces_only() {
        assert_eq!(shingles("").len(), 0);
        assert_eq!(shingles("a b"), ["a", "b"]);
        assert_eq!(shingles("a b c d"), ["a b c", "b c d"]);
        assert_eq!(shingles("a\tb c"), ["a\tb", "c"]);
    }
}
