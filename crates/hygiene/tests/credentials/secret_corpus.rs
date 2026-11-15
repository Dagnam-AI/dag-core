//! The reference's credential cases in `corpus/credentials.jsonl` against `find_secrets`.
//!
//! The corpus holds findings after the scan resolved them: a resolved finding is a region (the
//! union of every overlapping candidate of any class, named by its best-ranked member), and the
//! text between findings is read again as a piece of its own. So for each `PII_SECRET` span of a
//! case, the finder reports exactly it on the whole text, or the corpus span is one of these:
//!
//! * a wider region that holds a span of the finder (an address, a phone number or another
//!   candidate overlapped the credential and widened it);
//! * the span the finder reports on the text between the findings around the region (a piece
//!   the scan reads again on its own), where a guard that looked at a neighbour's character no
//!   longer applies.
//!
//! The other direction holds with no exception: each span the finder reports lies inside some
//! span of the case, of any class (regions never overlap, so a span inside none would be one
//! the reference does not report). Messages carry the case number and the offsets, never text.
use dagnam_hygiene::{find_secrets, to_char_offsets};
use regex::Regex;

use crate::{CredentialCase, FindingsCase, cases};

/// The `PII_SECRET` spans of the credential cases: how many the finder reports exactly on the
/// whole text, how many are wider regions that hold a span, and how many only a piece finds.
const EXACT: usize = 10_181;
const INSIDE: usize = 687;
const PIECE: usize = 34;

/// The runs of `text` between placeholders (a class's `[REDACTED:...]` and `<SECRET>`), as
/// byte ranges: the scan reads each run as a string of its own, and so does this test.
fn segments(text: &str) -> Vec<(usize, usize)> {
    let placeholder = Regex::new(r"\[REDACTED:PII_[A-Z0-9_]+\]|<SECRET>").unwrap();
    let mut runs = Vec::new();
    let mut cursor = 0;
    for found in placeholder.find_iter(text) {
        runs.push((cursor, found.start()));
        cursor = found.end();
    }
    runs.push((cursor, text.len()));
    runs.retain(|&(start, end)| start < end);
    runs
}

/// What `find_secrets` reports on `text`, read one run between placeholders at a time, in code
/// points.
fn found_in(text: &str) -> Vec<(usize, usize)> {
    let spans: Vec<(usize, usize)> = segments(text)
        .into_iter()
        .flat_map(|(start, end)| {
            find_secrets(&text[start..end])
                .into_iter()
                .map(move |(s, e)| (start + s, start + e))
        })
        .collect();
    let bytes: Vec<usize> = spans.iter().flat_map(|&(s, e)| [s, e]).collect();
    let offsets = to_char_offsets(text, &bytes);
    offsets.chunks(2).map(|pair| (pair[0], pair[1])).collect()
}

/// Every span the finder reports lies inside some span the reference reports for the text.
fn assert_nothing_extra(number: usize, text: &str, expected: &[(usize, usize, String)]) {
    let length = text.chars().count();
    for (s, e) in found_in(text) {
        assert!(
            expected.iter().any(|&(rs, re, _)| rs <= s && e <= re),
            "case {number}: finder span {s}..{e} lies in no span of a {length}-character text"
        );
    }
}

#[test]
fn every_reference_secret_is_found_exactly_inside_or_by_the_piece_it_starts() {
    let (mut exact, mut inside, mut piece) = (0, 0, 0);
    for (number, case) in cases::<CredentialCase>("credentials").iter().enumerate() {
        let text = case.assembled();
        let found = found_in(&text);
        let length = text.chars().count();
        for &(start, end, ref code) in &case.spans {
            if code != "PII_SECRET" {
                continue;
            }
            if found.contains(&(start, end)) {
                exact += 1;
            } else if found.iter().any(|&(s, e)| start <= s && e <= end) {
                inside += 1;
            } else {
                // The scan reads the text between the findings around this one as a piece of its
                // own, where a guard that looked at a neighbour's character no longer applies.
                let prior = case
                    .spans
                    .iter()
                    .map(|&(_, e, _)| e)
                    .filter(|&e| e <= start)
                    .max()
                    .unwrap_or(0);
                let next = case
                    .spans
                    .iter()
                    .map(|&(s, _, _)| s)
                    .filter(|&s| s >= end)
                    .min()
                    .unwrap_or(length);
                let between: String = text.chars().skip(prior).take(next - prior).collect();
                assert!(
                    found_in(&between).contains(&(start - prior, end - prior)),
                    "case {number}: no secret in {start}..{end} of a {length}-character text"
                );
                piece += 1;
            }
        }
    }
    assert_eq!((exact, inside, piece), (EXACT, INSIDE, PIECE));
}

#[test]
fn the_finder_reports_nothing_the_reference_does_not_in_every_credential_case() {
    for (number, case) in cases::<CredentialCase>("credentials").iter().enumerate() {
        assert_nothing_extra(number, &case.assembled(), &case.spans);
    }
}

#[test]
fn a_near_miss_is_read_as_no_secret_unless_another_class_holds_the_text() {
    let mut near_misses = 0;
    for (number, case) in cases::<CredentialCase>("credentials").iter().enumerate() {
        if case.secrets() > 0 {
            continue;
        }
        near_misses += 1;
        let text = case.assembled();
        for (s, e) in found_in(&text) {
            assert!(
                case.spans.iter().any(|&(rs, re, _)| rs <= s && e <= re),
                "case {number}: near miss with a finder span {s}..{e} in no span"
            );
        }
    }
    assert!(
        near_misses >= crate::MIN_UNFLAGGED,
        "{near_misses} near misses"
    );
}

#[test]
fn the_texts_chosen_to_hold_no_credential_hold_none_for_the_finder() {
    for (number, case) in cases::<FindingsCase>("findings").iter().enumerate() {
        assert_nothing_extra(number, &case.text, &case.spans);
    }
}

#[test]
fn the_spans_are_ascending_and_apart_on_every_corpus_text() {
    let texts = cases::<CredentialCase>("credentials")
        .into_iter()
        .map(|case| case.assembled())
        .chain(
            cases::<FindingsCase>("findings")
                .into_iter()
                .map(|c| c.text),
        );
    for (number, text) in texts.enumerate() {
        let spans = find_secrets(&text);
        assert!(
            spans.iter().all(|&(start, end)| start < end),
            "text {number}: an empty span"
        );
        assert!(
            spans.windows(2).all(|pair| pair[0].1 <= pair[1].0),
            "text {number}: spans that overlap or are out of order"
        );
    }
}
