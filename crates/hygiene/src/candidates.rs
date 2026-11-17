//! The candidate pairs of a batch: two texts that share at least one LSH band, each pair once, in
//! ascending order (the `low` text ascending, and for each the `high` texts ascending). That is
//! the reference's order, so the walk that counts comparisons stops at the same pair it does.
//!
//! The pairs are never held. Texts that share a band's bucket are one list, a text points at the
//! list it is in, and the pairs of a text `low` are the later members of its lists, merged and
//! sorted one text at a time. A set of all the pairs holds the square of the number of identical
//! texts.
use std::collections::HashMap;
use std::ops::Range;

use crate::cluster::Signature;
use crate::minhash::NUM_PERMUTATIONS;

/// LSH bands.
pub const BANDS: usize = 32;
/// Signature positions in one band.
const ROWS_PER_BAND: usize = NUM_PERMUTATIONS / BANDS;
const _: () = assert!(NUM_PERMUTATIONS == 128 && BANDS * ROWS_PER_BAND == NUM_PERMUTATIONS);

/// What a text points at in a band whose bucket it does not share.
const ALONE: usize = usize::MAX;

/// The positions of band `band` of a signature, the key of its bucket.
#[allow(clippy::indexing_slicing)] // `band < BANDS`, and a signature holds exactly BANDS chunks
fn band_key(signature: &Signature, band: usize) -> &[u64; ROWS_PER_BAND] {
    &signature.as_chunks::<ROWS_PER_BAND>().0[band]
}

/// One band's buckets of two or more texts, and for each text the bucket it is in.
struct Band {
    /// Per text: its index in `buckets`, or `ALONE`.
    bucket_of: Vec<usize>,
    /// The texts that share the same positions of this band, ascending; at least two each.
    buckets: Vec<Vec<usize>>,
}

impl Band {
    #[allow(clippy::indexing_slicing)] // every member is an index below `signatures.len()`
    fn new(signatures: &[Signature], band: usize) -> Self {
        let mut by_key: HashMap<&[u64; ROWS_PER_BAND], Vec<usize>> = HashMap::new();
        for (index, signature) in signatures.iter().enumerate() {
            by_key
                .entry(band_key(signature, band))
                .or_default()
                .push(index);
        }
        let mut bucket_of = vec![ALONE; signatures.len()];
        let mut buckets = Vec::new();
        // The map's order is arbitrary; nothing reads the buckets in order, only through
        // `bucket_of`, so the walk is the same on every run.
        for members in by_key.into_values().filter(|members| members.len() > 1) {
            for &index in &members {
                bucket_of[index] = buckets.len();
            }
            buckets.push(members);
        }
        Self { bucket_of, buckets }
    }

    /// The members of `low`'s bucket after `low`, ascending; none when `low` is alone in it.
    fn later_than(&self, low: usize) -> &[usize] {
        let members = self
            .bucket_of
            .get(low)
            .and_then(|&bucket| self.buckets.get(bucket))
            .map_or(&[][..], Vec::as_slice);
        members
            .get(members.partition_point(|&m| m <= low)..)
            .unwrap_or_default()
    }
}

/// The candidate pairs `(low, high)`, `low < high`, in ascending order.
pub(crate) struct Candidates {
    bands: Vec<Band>,
    /// The texts that have not yet been `low`, ascending.
    lows: Range<usize>,
    /// Per text: the last `low` that listed it, so a text in several of `low`'s buckets is listed once.
    listed_by: Vec<usize>,
    /// The `low` the pairs being yielded belong to.
    low: usize,
    /// The `high` texts of `low`, ascending, and how many were yielded.
    highs: Vec<usize>,
    yielded: usize,
}

impl Candidates {
    /// The pairs of `signatures`. One band's map is alive at a time while the bands are built.
    pub(crate) fn new(signatures: &[Signature]) -> Self {
        Self {
            bands: (0..BANDS).map(|band| Band::new(signatures, band)).collect(),
            lows: 0..signatures.len(),
            listed_by: vec![usize::MAX; signatures.len()],
            low: 0,
            highs: Vec::new(),
            yielded: 0,
        }
    }

    /// Makes `low` the text whose pairs are yielded next.
    fn load(&mut self, low: usize) {
        self.low = low;
        self.yielded = 0;
        self.highs.clear();
        for band in &self.bands {
            for &high in band.later_than(low) {
                if let Some(by) = self.listed_by.get_mut(high)
                    && *by != low
                {
                    *by = low;
                    self.highs.push(high);
                }
            }
        }
        self.highs.sort_unstable();
    }
}

impl Iterator for Candidates {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<(usize, usize)> {
        loop {
            if let Some(&high) = self.highs.get(self.yielded) {
                self.yielded += 1;
                return Some((self.low, high));
            }
            // The texts are a finite range, so the walk ends whatever else changes.
            let low = self.lows.next()?;
            self.load(low);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::cluster::sign;

    /// The pairs that share any band, by comparing every pair of signatures.
    fn sharing_a_band(signatures: &[Signature]) -> BTreeSet<(usize, usize)> {
        (0..signatures.len())
            .flat_map(|low| ((low + 1)..signatures.len()).map(move |high| (low, high)))
            .filter(|&(low, high)| {
                (0..BANDS).any(|band| {
                    band_key(&signatures[low], band) == band_key(&signatures[high], band)
                })
            })
            .collect()
    }

    /// At most `most` of the pairs: a test never reads an iterator to its end, so one that does
    /// not end cannot grow without bound.
    fn first(pairs: Candidates, most: usize) -> Vec<(usize, usize)> {
        pairs.take(most).collect()
    }

    fn signatures_of(texts: &[&str]) -> Vec<Signature> {
        texts.iter().map(|text| sign(text)).collect()
    }

    #[test]
    fn candidates_are_the_pairs_that_share_any_band_each_once_in_ascending_order() {
        let signatures = signatures_of(&[
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
            "a b c d e f",
        ]);
        let expected = sharing_a_band(&signatures);
        assert!(expected.contains(&(0, 1)) && expected.contains(&(4, 5)));
        assert!(!expected.contains(&(0, 3)));
        // Taken a few past the expected number: an iterator that never ends (a mutant that stops
        // advancing) fails the comparison at once instead of filling the heap.
        let streamed = first(Candidates::new(&signatures), expected.len() + 4);
        // Ascending is the order of a sorted set; no pair twice.
        assert_eq!(streamed, expected.into_iter().collect::<Vec<_>>());
    }

    #[test]
    fn a_pair_that_shares_many_bands_is_one_candidate() {
        let signatures = signatures_of(&["x y z w", "x y z w", "x y z w"]);
        assert_eq!(
            first(Candidates::new(&signatures), 7),
            [(0, 1), (0, 2), (1, 2)]
        );
    }

    #[test]
    fn no_texts_and_one_text_have_no_candidates() {
        assert_eq!(first(Candidates::new(&[]), 4), []);
        assert_eq!(first(Candidates::new(&signatures_of(&["a b c"])), 4), []);
    }
}
