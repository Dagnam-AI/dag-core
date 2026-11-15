//! The reference's `PII_EMAIL` and `PII_PHONE` findings in `corpus/findings.jsonl`, by tier.
//!
//! `detectors/corpus.rs` holds the method (the four tiers and the no-false-positive check on
//! every finder); the exact counts of these two classes are pinned here.
use crate::detectors::corpus::tiers_by_class;

/// `[exact, inside, tail, between]`: the email corpus spans a finder reports exactly, the ones
/// that are wider regions, the ones found by the text from the region's start, and the ones
/// found by the piece between the neighbouring regions.
const EMAIL: [usize; 4] = [539, 4, 0, 0];
const PHONE: [usize; 4] = [839, 0, 11, 3];

#[test]
fn the_email_corpus_spans_fall_in_the_tiers_pinned_here() {
    assert_eq!(tiers_by_class().get("PII_EMAIL"), Some(&EMAIL));
}

#[test]
fn the_phone_corpus_spans_fall_in_the_tiers_pinned_here() {
    assert_eq!(tiers_by_class().get("PII_PHONE"), Some(&PHONE));
}
