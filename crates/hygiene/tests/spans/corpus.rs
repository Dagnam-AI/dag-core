//! `findings` against what the reference's `findings` reported, case by case.
//!
//! Two corpora hold that answer: `findings.jsonl` (no text with a credential region) and
//! `credentials.jsonl` (texts assembled from fragments, with the credentials in them). Every
//! span of every case must come out exactly: the finders on their own only agree with the
//! corpus up to a containing region, a piece read after a neighbour, or the piece between two
//! neighbours (`tests/detectors/corpus.rs`); resolving the overlaps and re-reading the text after
//! each finding is what turns those into the reference's exact spans.
use std::collections::BTreeMap;

use crate::reference::findings;

use crate::{CredentialCase, FindingsCase, cases};

/// What `findings` says of `text`, with the codes as owned strings.
fn found(text: &str) -> Vec<(usize, usize, String)> {
    findings(text)
        .into_iter()
        .map(|(start, end, code)| (start, end, code.to_owned()))
        .collect()
}

#[test]
fn findings_match_the_reference_on_every_corpus_text() {
    for (number, case) in cases::<FindingsCase>("findings").iter().enumerate() {
        assert_eq!(
            found(&case.text),
            case.spans,
            "case {number}: text of {} characters",
            case.text.chars().count()
        );
    }
}

#[test]
fn findings_match_the_reference_on_every_credentials_text() {
    for (number, case) in cases::<CredentialCase>("credentials").iter().enumerate() {
        let text = case.assembled();
        assert_eq!(
            found(&text),
            case.spans,
            "case {number}: text of {} characters",
            text.chars().count()
        );
    }
}

#[test]
fn the_corpora_cover_every_class_and_many_texts_with_several_findings() {
    // A corpus with one finding per text would pass a resolver that never read past a finding:
    // both corpora together hold every class, and many texts with several findings (the pieces
    // between them are read again).
    let mut per_class: BTreeMap<String, usize> = BTreeMap::new();
    let mut several = 0;
    for case in cases::<FindingsCase>("findings") {
        several += usize::from(case.spans.len() > 1);
        for (_, _, code) in case.spans {
            *per_class.entry(code).or_default() += 1;
        }
    }
    for case in cases::<CredentialCase>("credentials") {
        several += usize::from(case.spans.len() > 1);
        for (_, _, code) in case.spans {
            *per_class.entry(code).or_default() += 1;
        }
    }
    assert_eq!(per_class.len(), dagnam_hygiene::DETECTORS.len());
    assert!(several > 1_000, "{several} texts with several findings");
}

/// The text of the first corpus case that is one finding of class `code` and satisfies `keep`,
/// cut to the finding: a sample of the class, taken at run time.
fn sample(code: &str, keep: impl Fn(&str) -> bool) -> String {
    cases::<FindingsCase>("findings")
        .into_iter()
        .find_map(|case| match case.spans.as_slice() {
            [(start, end, c)] if c == code => {
                let found: String = case.text.chars().skip(*start).take(end - start).collect();
                keep(&found).then_some(found)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no corpus case holds just one {code} that fits"))
}

#[test]
fn a_finding_hidden_at_the_far_end_of_a_long_leftover_piece_is_found() {
    // A phone number glued by a hyphen to an IPv4 address is refused by its own guard while the
    // address is part of the text, and is a finding once the address is cut away. It stands more
    // than a window (48 characters) after the start of the text left over, so the window after
    // the first finding shows nothing and the rest has to be read whole.
    let email = sample("PII_EMAIL", |_| true);
    let phone = sample("PII_PHONE", |_| true);
    let ip = sample("PII_IP_ADDRESS", |found| !found.contains(':'));
    let card = sample("PII_PAYMENT_CARD", |_| true);
    let text = format!("{email} {}{phone}-{ip} {card}", "a".repeat(60));
    let length = |s: &str| s.chars().count();
    let phone_at = length(&email) + 1 + 60;
    let ip_at = phone_at + length(&phone) + 1;
    let card_at = ip_at + length(&ip) + 1;
    assert_eq!(
        findings(&text),
        [
            (0, length(&email), "PII_EMAIL"),
            (phone_at, phone_at + length(&phone), "PII_PHONE"),
            (ip_at, ip_at + length(&ip), "PII_IP_ADDRESS"),
            (card_at, card_at + length(&card), "PII_PAYMENT_CARD"),
        ]
    );
}
