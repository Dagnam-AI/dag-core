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

/// The word shingles of a normalised text (words separated by single spaces).
fn shingles(text: &str) -> Vec<String> {
    let words: Vec<&str> = text.split(' ').filter(|word| !word.is_empty()).collect();
    if words.len() < SHINGLE_SIZE {
        return words.iter().map(ToString::to_string).collect();
    }
    words.windows(SHINGLE_SIZE).map(|w| w.join(" ")).collect()
}

/// Index pairs `(low, high)` that share at least one LSH band, in ascending order.
fn candidate_pairs(signatures: &[[u64; NUM_PERMUTATIONS]]) -> BTreeSet<(usize, usize)> {
    let mut bands: Vec<HashMap<&[u64; ROWS_PER_BAND], Vec<usize>>> = vec![HashMap::new(); BANDS];
    for (index, signature) in signatures.iter().enumerate() {
        for (band, rows) in bands
            .iter_mut()
            .zip(signature.as_chunks::<ROWS_PER_BAND>().0)
        {
            band.entry(rows).or_default().push(index);
        }
    }
    let mut candidates = BTreeSet::new();
    for members in bands.iter().flat_map(HashMap::values) {
        for (at, &low) in members.iter().enumerate() {
            candidates.extend(members.iter().skip(at + 1).map(|&high| (low, high)));
        }
    }
    candidates
}

/// The root of `index`. Without path compression: the roots always sit at the lowest index,
/// so the chains stay short and compression could change no answer. A chain has at most
/// `parent.len()` links, which bounds the walk.
fn find(parent: &[usize], index: usize) -> usize {
    std::iter::successors(Some(index), |&at| {
        parent.get(at).copied().filter(|&above| above != at)
    })
    .take(parent.len())
    .last()
    .unwrap_or(index)
}

/// The fraction of signature positions at which the two texts agree.
fn similarity(left: &[u64; NUM_PERMUTATIONS], right: &[u64; NUM_PERMUTATIONS]) -> f64 {
    let agree: u32 = left.iter().zip(right).map(|(a, b)| u32::from(a == b)).sum();
    f64::from(agree) / WIDTH
}

/// Clusters normalised texts: the indices of every text that duplicates an earlier one,
/// and each verified pair. `None` when `threshold` is outside `(0, 1]`.
///
/// A cluster's representative is its lowest index, which is never reported as a duplicate.
#[must_use]
pub fn near_duplicates(texts: &[&str], threshold: f64) -> Option<(Vec<usize>, Vec<Pair>)> {
    // Written as a conjunction so that NaN, which compares false, is refused too.
    let valid = threshold > 0.0 && threshold <= 1.0;
    if !valid {
        return None;
    }
    let signatures: Vec<[u64; NUM_PERMUTATIONS]> = texts
        .iter()
        .map(|text| minhash_signature(shingles(text).iter().map(String::as_str)))
        .collect();
    let mut parent: Vec<usize> = (0..texts.len()).collect();
    let mut pairs = Vec::new();
    for (low, high) in candidate_pairs(&signatures) {
        let score = signatures
            .get(low)
            .zip(signatures.get(high))
            .map_or(0.0, |(left, right)| similarity(left, right));
        if score < threshold {
            continue;
        }
        pairs.push((low, high, score));
        let (root_low, root_high) = (find(&parent, low), find(&parent, high));
        if root_low != root_high {
            // The higher root goes under the lower, so the kept row is the lowest index.
            let (root_low, root_high) = (root_low.min(root_high), root_low.max(root_high));
            // `get_mut` is `Some` for every root; the iterator form leaves no untestable `else`.
            parent
                .get_mut(root_high)
                .into_iter()
                .for_each(|slot| *slot = root_low);
        }
    }
    let duplicates = (0..texts.len())
        .filter(|&index| find(&parent, index) != index)
        .collect();
    Some((duplicates, pairs))
}
