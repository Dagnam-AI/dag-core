//! What a batch costs in memory: a refused batch next to nothing, the walk without holding the
//! candidate pairs, and the result in the memory it needs and no more. Measured on the test
//! binary's own allocator (`guard`), on the thread that makes the call.
use dagnam_hygiene::{
    Limits, MAX_TEXT_CHARS, MAX_TEXTS, MAX_TOTAL_CHARS, near_duplicates, near_duplicates_within,
};

use crate::guard::{high_water, reset_high_water};

/// The result of `call` and the most heap it had live at once beyond what was live before it.
fn peak_of<T>(call: impl FnOnce() -> T) -> (T, usize) {
    reset_high_water();
    let result = call();
    (result, high_water())
}

const KIB: usize = 1 << 10;
const MIB: usize = 1 << 20;
/// What one reported pair weighs: two indices and a similarity.
const PAIR_BYTES: usize = std::mem::size_of::<(usize, usize, f64)>();

#[test]
fn a_batch_one_past_a_size_limit_is_refused_having_cost_next_to_nothing() {
    // The batches are built first, outside the measure: the call itself must not copy them.
    let many = vec!["a"; MAX_TEXTS + 1];
    let long = ["a".repeat(MAX_TEXT_CHARS + 1)];
    let whole = MAX_TOTAL_CHARS / MAX_TEXT_CHARS;
    let mut heavy: Vec<String> = vec!["a".repeat(MAX_TEXT_CHARS); whole];
    heavy.push("a".to_owned());
    let long: Vec<&str> = long.iter().map(String::as_str).collect();
    let heavy: Vec<&str> = heavy.iter().map(String::as_str).collect();
    for (name, texts) in [("count", &many), ("one text", &long), ("total", &heavy)] {
        let (result, peak) = peak_of(|| near_duplicates(texts, 0.9));
        assert!(result.is_err(), "{name}");
        assert!(peak < 64 * KIB, "{name}: {peak} bytes");
    }
}

#[test]
fn a_batch_past_the_pair_limit_is_refused_before_its_pairs_are_built() {
    // 4,000 identical texts have almost 8 million pairs (190 MB as a result); the walk stops at
    // the 1,001st and holds the signatures and the buckets, a few MB.
    let identical = vec!["a b c d"; 4_000];
    let limits = Limits {
        comparisons: usize::MAX,
        pairs: 1_000,
    };
    let (result, peak) = peak_of(|| near_duplicates_within(&identical, 0.9, limits));
    assert!(result.is_err());
    assert!(peak < 8 * MIB, "{peak} bytes");
}

#[test]
fn the_result_takes_the_memory_of_its_pairs_and_no_more() {
    // 1,000 identical texts have 499,500 pairs. A vector that grew by doubling would hold the old
    // buffer and the new one at the moment it grew, half as much again.
    let identical = vec!["a b c d"; 1_000];
    let (result, peak) = peak_of(|| near_duplicates(&identical, 0.9));
    let pairs = result.unwrap().1.len();
    assert_eq!(pairs, 499_500);
    assert!(peak < pairs * PAIR_BYTES + 3 * MIB, "{peak} bytes");
}

#[test]
fn the_walk_does_not_hold_the_candidate_pairs() {
    // About a third of the pairs of 3,000 texts that share a word are candidates (1.5 million),
    // none of them near-duplicates; held as a set they would take more than 30 MB.
    let owned: Vec<String> = (0..3_000).map(|i| format!("common w{i}")).collect();
    let texts: Vec<&str> = owned.iter().map(String::as_str).collect();
    let (result, peak) = peak_of(|| near_duplicates(&texts, 0.9));
    assert_eq!(result.unwrap().1.len(), 0);
    assert!(peak < 8 * MIB, "{peak} bytes");
}
