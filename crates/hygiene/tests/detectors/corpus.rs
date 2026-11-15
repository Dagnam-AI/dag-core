//! The reference's findings in `corpus/findings.jsonl` against these ten finders.
//!
//! The corpus holds findings after the scan resolved them, and the scan is not one run of every
//! finder on the whole text: a resolved finding is a region (the union of every overlapping
//! candidate, named by its best-ranked member), and the text between findings is read again as
//! a piece of its own. So for each corpus span of these classes, a finder reports exactly it on
//! the whole text, or the corpus span is one of these:
//!
//! * a wider region that holds a span of the finder (a phone, email or credential candidate
//!   outside this module widened it);
//! * a span the finder reports on the text that starts at the region, ending where the region
//!   does (the piece the scan reads after the finding before it, where a guard that looked at
//!   that finding's last character no longer applies).
//!
//! The other direction holds with no exception: each span any of these finders reports on a
//! corpus text lies inside some corpus span of that case, of any class (regions never overlap,
//! so a span inside none would be one the reference does not report). Messages carry the case
//! number, the class and the offsets, never the text.
use dagnam_hygiene::{DETECTORS, to_char_offsets};

use crate::{FindingsCase, cases};

/// The corpus spans of these ten classes: how many the finders report exactly on the whole text,
/// how many are wider regions that hold a span, and how many only a piece read finds.
const EXACT: usize = 3_355;
const INSIDE: usize = 24;
const PIECE: usize = 34;

/// Every span of every finder on `text`, as `(start, end, code)` in code points.
fn spans_of_every_finder(text: &str) -> Vec<(usize, usize, &'static str)> {
    DETECTORS
        .iter()
        .flat_map(|detector| {
            let spans = (detector.find)(text);
            let bytes: Vec<usize> = spans.iter().flat_map(|&(s, e)| [s, e]).collect();
            let offsets = to_char_offsets(text, &bytes);
            offsets
                .chunks(2)
                .map(|pair| (pair[0], pair[1], detector.code))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The `code` finder's spans on `text`, in code points.
fn spans_of(code: &str, text: &str) -> Vec<(usize, usize)> {
    spans_of_every_finder(text)
        .into_iter()
        .filter(|&(_, _, c)| c == code)
        .map(|(s, e, _)| (s, e))
        .collect()
}

#[test]
fn every_reference_span_of_these_classes_is_found_exactly_inside_or_by_the_piece_it_starts() {
    let (mut exact, mut inside, mut piece) = (0, 0, 0);
    for (number, case) in cases::<FindingsCase>("findings").iter().enumerate() {
        let found = spans_of_every_finder(&case.text);
        let length = case.text.chars().count();
        for (start, end, code) in &case.spans {
            if !DETECTORS.iter().any(|detector| detector.code == code) {
                continue;
            }
            let (start, end) = (*start, *end);
            let mine: Vec<(usize, usize)> = found
                .iter()
                .filter(|f| f.2 == code)
                .map(|f| (f.0, f.1))
                .collect();
            if mine.contains(&(start, end)) {
                exact += 1;
            } else if mine.iter().any(|&(s, e)| start <= s && e <= end) {
                inside += 1;
            } else {
                let tail: String = case.text.chars().skip(start).collect();
                assert!(
                    spans_of(code, &tail)
                        .iter()
                        .any(|&(s, e)| s == 0 && e == end - start),
                    "case {number}: no {code} span in {start}..{end} of a {length}-character text"
                );
                piece += 1;
            }
        }
        // Nothing is found that the reference did not: each finder span of the text lies inside
        // some corpus region of the case, of any class (a region is the union of what overlaps,
        // so it holds every span that touches it).
        for &(s, e, c) in &found {
            assert!(
                case.spans.iter().any(|&(rs, re, _)| rs <= s && e <= re),
                "case {number}: {c} span {s}..{e} lies in no corpus region of a {length}-character text"
            );
        }
    }
    assert_eq!((exact, inside, piece), (EXACT, INSIDE, PIECE));
}
