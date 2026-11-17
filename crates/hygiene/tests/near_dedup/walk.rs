//! The walk over the candidate pairs: what it counts, where it stops, and that a limit it does not
//! reach changes nothing. The limits are the walk's own (`Limits`), small here so that a batch of
//! a few texts reaches them; their real values are pinned in `limits.rs` and the crate's tests.
use dagnam_hygiene::{Batch, Limits, NearDedupError, near_duplicates, near_duplicates_within};

/// Ten identical texts: 45 candidate pairs, every one a near-duplicate.
const IDENTICAL: [&str; 10] = ["a b c d e f"; 10];

fn within(
    texts: &[&str],
    threshold: f64,
    comparisons: usize,
    pairs: usize,
) -> Result<usize, NearDedupError> {
    near_duplicates_within(texts, threshold, Limits { comparisons, pairs }).map(|r| r.1.len())
}

#[test]
fn a_comparison_is_counted_before_it_is_made_so_the_limit_itself_is_allowed() {
    assert_eq!(within(&IDENTICAL, 0.5, 45, usize::MAX), Ok(45));
    assert_eq!(
        within(&IDENTICAL, 0.5, 44, usize::MAX),
        Err(NearDedupError::TooManyComparisons { limit: 44 })
    );
    assert_eq!(
        within(&IDENTICAL, 0.5, 0, usize::MAX),
        Err(NearDedupError::TooManyComparisons { limit: 0 })
    );
}

#[test]
fn a_candidate_that_is_not_a_near_duplicate_is_a_comparison_all_the_same() {
    // Three candidates, of which one pair reaches the threshold of 1.0.
    let texts = ["a b c d e f", "a b c d e f", "a b c d e g"];
    assert_eq!(within(&texts, 1.0, 3, usize::MAX), Ok(1));
    assert_eq!(
        within(&texts, 1.0, 2, usize::MAX),
        Err(NearDedupError::TooManyComparisons { limit: 2 })
    );
}

#[test]
fn a_pair_is_counted_before_it_is_kept_so_the_limit_itself_is_allowed() {
    assert_eq!(within(&IDENTICAL, 0.5, usize::MAX, 45), Ok(45));
    assert_eq!(
        within(&IDENTICAL, 0.5, usize::MAX, 44),
        Err(NearDedupError::TooManyPairs { limit: 44 })
    );
    assert_eq!(
        within(&IDENTICAL, 0.5, usize::MAX, 0),
        Err(NearDedupError::TooManyPairs { limit: 0 })
    );
}

#[test]
fn a_pair_past_the_limit_is_refused_though_the_batch_has_no_comparison_left_to_make() {
    // The 45th comparison is the 45th pair: the comparison is allowed, the pair is not.
    assert_eq!(
        within(&IDENTICAL, 0.5, 45, 44),
        Err(NearDedupError::TooManyPairs { limit: 44 })
    );
}

#[test]
fn the_comparison_that_would_pass_its_limit_is_refused_before_its_pair_is_counted() {
    // Both limits are at 44: the 45th candidate trips the comparison limit, not the pair limit.
    assert_eq!(
        within(&IDENTICAL, 0.5, 44, 44),
        Err(NearDedupError::TooManyComparisons { limit: 44 })
    );
}

#[test]
fn the_pair_limit_is_not_reached_by_candidates_that_are_not_pairs() {
    let texts = ["a b c d e f", "a b c d e f", "a b c d e g"];
    // One pair is kept, two candidates are not: a limit of one pair holds.
    assert_eq!(within(&texts, 1.0, usize::MAX, 1), Ok(1));
    assert_eq!(
        within(&texts, 1.0, usize::MAX, 0),
        Err(NearDedupError::TooManyPairs { limit: 0 })
    );
}

#[test]
fn limits_that_are_not_reached_change_nothing() {
    let open = near_duplicates_within(&IDENTICAL, 0.5, Limits::DEFAULT);
    assert_eq!(open, near_duplicates(&IDENTICAL, 0.5));
    assert_eq!(
        near_duplicates_within(
            &IDENTICAL,
            0.5,
            Limits {
                comparisons: 45,
                pairs: 45
            }
        ),
        open
    );
    let (duplicates, pairs) = open.unwrap();
    assert_eq!(duplicates, (1..10).collect::<Vec<_>>());
    assert_eq!(pairs.len(), 45);
}

#[test]
fn pairs_come_in_the_order_of_the_walk() {
    let (_, pairs) = near_duplicates(&["p q r", "p q r", "p q r", "p q r"], 0.9).unwrap();
    let order: Vec<(usize, usize)> = pairs.iter().map(|&(low, high, _)| (low, high)).collect();
    assert_eq!(order, [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]);
}

#[test]
fn the_message_names_the_limit_in_force() {
    let comparisons = within(&IDENTICAL, 0.5, 44, usize::MAX).unwrap_err();
    assert_eq!(
        comparisons.to_string(),
        "more than MAX_COMPARISONS (44) comparisons"
    );
    let pairs = within(&IDENTICAL, 0.5, usize::MAX, 3).unwrap_err();
    assert_eq!(
        pairs.to_string(),
        "more than MAX_PAIRS (3) near-duplicate pairs"
    );
}

#[test]
fn a_batch_is_read_in_chunks_that_make_up_the_whole_result() {
    let mut batch = Batch::new(&IDENTICAL, 0.5, Limits::DEFAULT).unwrap();
    assert_eq!(batch.pair_count(), 45);
    let (mut read, mut chunks) = (Vec::new(), Vec::new());
    // Eight reads at most (seven chunks and the empty one that ends them): a `fill` that never
    // reports an empty chunk fails the comparison below instead of growing `chunks` for ever.
    for _ in 0..8 {
        let before = read.len();
        let added = batch.fill(&mut read, 7);
        assert_eq!(added, read.len() - before);
        chunks.push(added);
    }
    assert_eq!(chunks, [7, 7, 7, 7, 7, 7, 3, 0]);
    assert_eq!(batch.fill(&mut read, 7), 0);
    let whole = near_duplicates(&IDENTICAL, 0.5).unwrap();
    assert_eq!(read, whole.1);
    assert_eq!(batch.duplicates(), whole.0);
}

#[test]
fn a_result_larger_than_what_is_reserved_ahead_is_the_same_result() {
    // 1,500 identical texts: 1,124,250 pairs, more than the result reserves room for at once.
    let identical = vec!["a b c d"; 1_500];
    let (duplicates, pairs) = near_duplicates(&identical, 0.9).unwrap();
    assert_eq!(pairs.len(), 1_500 * 1_499 / 2);
    assert_eq!(duplicates.len(), 1_499);
    assert_eq!(pairs.first(), Some(&(0, 1, 1.0)));
    assert_eq!(pairs.last(), Some(&(1_498, 1_499, 1.0)));
}
