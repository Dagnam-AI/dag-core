//! The one integration-test binary: corpus cases and properties.
// Test binary: failing loudly is the point, and clippy.toml does not cover helper functions.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::de::DeserializeOwned;

/// The cases `corpus/minhash.jsonl` holds. Move with `corpus/README.md` on every re-export.
const MINHASH_CASES: usize = 739;
/// The cases `corpus/near_dedup.jsonl` holds.
const NEAR_DEDUP_CASES: usize = 180;
/// The cases `corpus/findings.jsonl` holds.
const FINDINGS_CASES: usize = 3795;
/// The cases `corpus/credentials.jsonl` holds.
const CREDENTIAL_CASES: usize = 9701;
/// The fewest credential cases with at least one `PII_SECRET` span, with none (a near miss
/// that carries a marker), and with two or more (several secrets in one text).
const MIN_FLAGGED: usize = 6798;
const MIN_UNFLAGGED: usize = 2903;
const MIN_MULTI_FLAGGED: usize = 2425;
/// The fewest findings of each class code the corpus may hold (no `PII_SECRET`: see the README).
const MIN_FINDINGS: [(&str, usize); 12] = [
    ("PII_CA_SIN", 218),
    ("PII_DATE_OF_BIRTH", 266),
    ("PII_EMAIL", 543),
    ("PII_EU_VAT", 537),
    ("PII_IBAN", 510),
    ("PII_IN_AADHAAR", 529),
    ("PII_IN_PAN", 118),
    ("PII_IP_ADDRESS", 715),
    ("PII_NATIONAL_ID", 54),
    ("PII_PAYMENT_CARD", 329),
    ("PII_PHONE", 853),
    ("PII_UK_NINO", 137),
];

/// One `minhash.jsonl` line: the shingles and the 128 signature values as decimal strings.
#[derive(Deserialize)]
struct MinhashCase {
    shingles: Vec<String>,
    signature: Vec<String>,
}

/// One `near_dedup.jsonl` line: row-text-normalised texts and the reference's verdict.
#[derive(Deserialize)]
struct NearDedupCase {
    texts: Vec<String>,
    threshold: f64,
    duplicate_indices: Vec<usize>,
    pairs: Vec<(usize, usize, f64)>,
}

/// One `findings.jsonl` line: a text and its spans as code-point offsets and a class code.
#[derive(Deserialize)]
struct FindingsCase {
    text: String,
    spans: Vec<(usize, usize, String)>,
}

/// One `credentials.jsonl` line: a text stored as fragments, and the reference's spans.
#[derive(Deserialize)]
struct CredentialCase {
    parts: Vec<String>,
    spans: Vec<(usize, usize, String)>,
}

impl CredentialCase {
    /// The text the reference ran on: the parts joined.
    fn assembled(&self) -> String {
        self.parts.concat()
    }

    /// How many of the spans are credentials.
    fn secrets(&self) -> usize {
        self.spans
            .iter()
            .filter(|(_, _, code)| code == "PII_SECRET")
            .count()
    }
}

/// Every case of `corpus/<name>.jsonl`.
fn cases<T: DeserializeOwned>(name: &str) -> Vec<T> {
    let path = format!("{}/../../corpus/{name}.jsonl", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    // Split on `\n` only (a text may hold U+2028 or U+0085): the empty piece after the
    // final newline is the one deliberate skip; any other blank line is a corrupt case.
    let mut lines: Vec<&str> = text.split('\n').collect();
    assert_eq!(lines.pop(), Some(""), "{name}.jsonl must end in a newline");
    lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            serde_json::from_str(line).unwrap_or_else(|e| panic!("{name}.jsonl:{}: {e}", index + 1))
        })
        .collect()
}

#[test]
fn the_core_api_version_starts_at_one() {
    assert_eq!(dagnam_hygiene::core_api(), 1);
}

#[test]
fn the_corpus_holds_exactly_the_cases_the_readme_counts() {
    assert_eq!(cases::<MinhashCase>("minhash").len(), MINHASH_CASES);
    assert_eq!(cases::<NearDedupCase>("near_dedup").len(), NEAR_DEDUP_CASES);
    assert_eq!(cases::<FindingsCase>("findings").len(), FINDINGS_CASES);
    assert_eq!(
        cases::<CredentialCase>("credentials").len(),
        CREDENTIAL_CASES
    );
}

#[test]
fn every_credential_case_assembles_to_a_text_its_spans_fit() {
    let cases = cases::<CredentialCase>("credentials");
    let (mut with, mut without, mut several) = (0, 0, 0);
    for case in &cases {
        assert!(case.parts.iter().all(|part| !part.is_empty()));
        let length = case.assembled().chars().count();
        assert!(case.spans.iter().all(|(s, e, _)| s < e && *e <= length));
        assert!(
            case.spans
                .windows(2)
                .all(|w| w[0].0 <= w[1].0 && w[0].1 <= w[1].0)
        );
        match case.secrets() {
            0 => without += 1,
            1 => with += 1,
            _ => {
                with += 1;
                several += 1;
            }
        }
    }
    assert!(with >= MIN_FLAGGED, "{with} cases with a secret");
    assert!(without >= MIN_UNFLAGGED, "{without} cases without one");
    assert!(several >= MIN_MULTI_FLAGGED, "{several} cases with several");
}

#[test]
fn every_case_has_the_shape_its_reader_expects() {
    for case in cases::<MinhashCase>("minhash") {
        assert_eq!(case.signature.len(), 128);
        assert!(case.signature.iter().all(|v| v.parse::<u64>().is_ok()));
        assert!(
            !case.shingles.is_empty() || case.signature.iter().all(|v| v == "18446744073709551615")
        );
    }
    for case in cases::<NearDedupCase>("near_dedup") {
        let n = case.texts.len();
        assert!(case.threshold > 0.0 && case.threshold <= 1.0);
        assert!(case.duplicate_indices.iter().all(|&i| i < n));
        assert!(
            case.pairs
                .iter()
                .all(|&(a, b, s)| a < b && b < n && (0.0..=1.0).contains(&s))
        );
    }
    for case in cases::<FindingsCase>("findings") {
        let length = case.text.chars().count();
        assert!(case.spans.iter().all(|(s, e, _)| s < e && *e <= length));
    }
}

#[test]
fn every_finding_class_keeps_at_least_the_findings_it_has_now() {
    let mut found: BTreeMap<String, usize> = BTreeMap::new();
    for case in cases::<FindingsCase>("findings") {
        for (_, _, code) in case.spans {
            *found.entry(code).or_default() += 1;
        }
    }
    assert_eq!(found.len(), MIN_FINDINGS.len(), "{found:?}");
    for (code, least) in MIN_FINDINGS {
        let count = found.get(code).copied().unwrap_or(0);
        assert!(
            count >= least,
            "{code}: {count} findings, at least {least} expected"
        );
    }
}
