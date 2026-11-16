//! `near_duplicates` on arbitrary texts and a threshold: it returns, refuses exactly the
//! thresholds outside `(0, 1]`, and what it returns is consistent with itself.
#![no_main]

use libfuzzer_sys::fuzz_target;

/// The most texts one input holds. The pairs of a run of identical texts grow with the square of
/// its length, so an input of thousands of newlines would end the run on memory, not on a bug;
/// whether the crate itself should cap that is an owner decision that is still pending.
const MOST_TEXTS: usize = 256;

/// The root of `index`: the lowest index of its cluster.
fn root(parent: &mut [usize], mut index: usize) -> usize {
    while parent[index] != index {
        index = parent[index];
    }
    index
}

fuzz_target!(|data: &[u8]| {
    let Some((head, rest)) = data.split_first_chunk::<8>() else {
        return;
    };
    // Any bit pattern: NaN, infinities, negatives, subnormals, above 1, and (rarely) in range.
    let threshold = f64::from_bits(u64::from_le_bytes(*head));
    let joined = String::from_utf8_lossy(rest);
    let texts: Vec<&str> = joined.split('\n').take(MOST_TEXTS).collect();
    let result = dagnam_hygiene::near_duplicates(&texts, threshold);
    if !(threshold > 0.0 && threshold <= 1.0) {
        assert!(result.is_none(), "a threshold outside (0, 1] must be refused");
        return;
    }
    let (duplicates, pairs) = result.expect("a threshold in (0, 1] is accepted");
    // The duplicates the pairs imply: the clusters the pairs join, less each one's lowest index.
    let mut parent: Vec<usize> = (0..texts.len()).collect();
    for &(low, high, similarity) in &pairs {
        assert!(low < high && high < texts.len(), "pair index out of range");
        assert!((threshold..=1.0).contains(&similarity), "similarity outside [threshold, 1]");
        let (a, b) = (root(&mut parent, low), root(&mut parent, high));
        parent[a.max(b)] = a.min(b);
    }
    let implied: Vec<usize> = (0..texts.len()).filter(|&i| root(&mut parent, i) != i).collect();
    // Sorted and unique by construction of `implied`, so equality checks both.
    assert_eq!(duplicates, implied, "duplicates are not the non-root set of the pairs");
});
