//! Where the PII findings in a text are: every detector over the text, overlapping candidates
//! resolved into one region each.
//!
//! **A placeholder is an opaque token.** A finding is a span inside one maximal run of text
//! that holds no placeholder (a segment), and whether it is reported is a function of that
//! segment alone, read as an independent string. The findings of a text are the least fixed
//! point of "detect in each piece, cut at the accepted spans, read every piece left over
//! again": the text a redaction leaves holds no finding when each piece is read on its own, and
//! the next scan reads exactly those pieces.
//!
//! Cost: every segment is read whole once, and the text after each accepted span is read again
//! through a small window ([`WINDOW`]), because the only place a finding that was hidden by its
//! neighbour can appear is the start of the text that follows it. A chain of findings that each
//! hide the next costs a window apiece, not a pass over the text. Linear in the text.
//!
//! Inside this module every offset is a byte offset into the text; [`findings`] turns them into
//! code-point offsets once, at the end.
use std::cmp::min_by_key;
use std::sync::LazyLock;

use regex::Regex;

use crate::detectors::{DETECTORS, Detector, compile};
use crate::text::{is_py_word, to_char_offsets};

/// After an accepted span the text right after it is read as a string of its own through a
/// window of this many characters, doubled while a candidate reaches the window's edge (so a
/// long finding costs its own length).
const WINDOW: usize = 48;
/// A guard looks at most two characters past a span; a candidate that ends closer to the
/// window's edge than this may read differently in the whole piece and is not trusted.
const MARGIN: usize = 3;

/// What a redaction writes for a class without a placeholder of its own, `{code}` being the
/// class code.
const REDACTION_TEMPLATE: &str = "[REDACTED:{code}]";

/// The pattern of every placeholder a redaction writes: the template for any `PII_` code (so a
/// class a later registry adds is opaque too) and each class's own placeholder, read from the
/// registry so that a class that gains one is cut at without a change here.
fn placeholder_pattern(detectors: &[Detector]) -> String {
    let template =
        regex::escape(REDACTION_TEMPLATE).replace(&regex::escape("{code}"), "PII_[A-Z0-9_]+");
    detectors
        .iter()
        .filter_map(|detector| detector.placeholder)
        .fold(template, |all, own| format!("{all}|{}", regex::escape(own)))
}

static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| compile(&placeholder_pattern(DETECTORS)));

/// One candidate or region: a byte span, the detector's position in the registry (the last
/// tie-break of the ranking) and the detector.
#[derive(Clone, Copy)]
struct Cand {
    start: usize,
    end: usize,
    order: usize,
    detector: &'static Detector,
}

impl Cand {
    /// The ranking of overlapping candidates, best first: a class that is not loose, then the
    /// earliest start, then the shorter span (the more specific reading: the user part of an
    /// address), then registry order. At one start the shorter span is the one with the
    /// smaller end, so the end is compared, not the length.
    fn rank(self) -> (bool, usize, usize, usize) {
        (self.detector.loose, self.start, self.end, self.order)
    }

    /// The same candidate `by` bytes further on.
    fn moved(self, by: usize) -> Self {
        Self {
            start: self.start + by,
            end: self.end + by,
            ..self
        }
    }
}

/// `text[from..to]`; every offset here is a character boundary (a finder that returned another
/// would hide text from every later read, so a debug build refuses it), so the empty text is
/// never the answer in practice.
fn slice(text: &str, from: usize, to: usize) -> &str {
    debug_assert!(text.is_char_boundary(from) && text.is_char_boundary(to));
    text.get(from..to).unwrap_or_default()
}

/// Maximal runs of `text` between placeholders, as half-open byte ranges; empty runs dropped.
fn segments(text: &str) -> Vec<(usize, usize)> {
    let mut pieces = Vec::new();
    let mut cursor = 0;
    for placeholder in PLACEHOLDER.find_iter(text) {
        if placeholder.start() > cursor {
            pieces.push((cursor, placeholder.start()));
        }
        cursor = placeholder.end();
    }
    if cursor < text.len() {
        pieces.push((cursor, text.len()));
    }
    pieces
}

/// Overlapping candidates form one region over their union, replaced once and named by the
/// best-ranked member.
///
/// A union, not a drop: a value pattern that over-matches into a neighbouring number must not
/// leave the rest of that number as text. Its cost is that a host glued to a URL password
/// through the email pattern goes with the password.
fn resolve(mut candidates: Vec<Cand>) -> Vec<Cand> {
    candidates.sort_by_key(|c| (c.start, c.end));
    let mut regions: Vec<(usize, usize, Cand)> = Vec::new(); // (start, end, the best-ranked member)
    for span in candidates {
        match regions.last_mut() {
            Some((_, end, best)) if span.start < *end => {
                *end = (*end).max(span.end);
                *best = min_by_key(*best, span, |c| c.rank());
            }
            _ => regions.push((span.start, span.end, span)),
        }
    }
    regions
        .into_iter()
        .map(|(start, end, best)| Cand { start, end, ..best })
        .collect()
}

/// Every detector's spans in `piece`, read as an independent string.
///
/// Always every class: a class a policy does not act on can still be what hides, or is hidden
/// by, one it does, so the scan and every policy resolve the same set.
fn detect(detectors: &'static [Detector], piece: &str) -> Vec<Cand> {
    detectors
        .iter()
        .enumerate()
        .flat_map(|(order, detector)| {
            (detector.find)(piece)
                .into_iter()
                .filter(|&(start, end)| end > start)
                .map(move |(start, end)| Cand {
                    start,
                    end,
                    order,
                    detector,
                })
        })
        .collect()
}

/// The pieces of `[a, b)` between `spans`: possibly empty ones, which hold no content and so are
/// never read.
fn cut(a: usize, b: usize, spans: &[Cand]) -> Vec<(usize, usize)> {
    let mut pieces = Vec::new();
    let mut cursor = a;
    for span in spans {
        pieces.push((cursor, span.start));
        cursor = span.end;
    }
    pieces.push((cursor, b));
    pieces
}

/// Whether any detector can find anything in `piece`: a letter or a digit, or an at sign. The
/// address pattern needs no letter or digit (`_@_._` is one), and neither does a URL's password
/// made of punctuation, but both need the at sign; every other class needs a letter or a
/// digit. Narrower than the union of the patterns' required characters would hide a finding from
/// the read that decides it, so this is derived from them, not guessed.
fn holds_content(piece: &str) -> bool {
    piece
        .chars()
        .any(|c| (is_py_word(c) && c != '_') || c == '@')
}

/// Whether fewer than [`MARGIN`] characters of `window` follow byte `end`.
fn near_edge(window: &str, end: usize) -> bool {
    slice(window, end, window.len())
        .chars()
        .take(MARGIN)
        .count()
        < MARGIN
}

/// One read of a window of the text.
struct Look<'t> {
    /// The text read.
    window: &'t str,
    /// The window's regions, relative to its start.
    regions: Vec<Cand>,
    /// Whether the window reaches the end of the piece.
    whole: bool,
}

/// The whole scan of one text.
struct Scan<'t> {
    detectors: &'static [Detector],
    text: &'t str,
    /// The accepted spans, in the order they were accepted.
    out: Vec<Cand>,
    /// Pieces still to read whole.
    work: Vec<(usize, usize)>,
    /// How many pieces were read whole: the cost the unit tests pin.
    #[allow(dead_code)] // read by the unit tests only
    full_reads: usize,
    /// How many windows were read: the cost the unit tests pin.
    #[allow(dead_code)] // read by the unit tests only
    windows: usize,
    /// How many characters the detectors were handed, whole pieces and windows together: the
    /// cost the unit tests pin, in a unit that does not depend on a clock.
    #[allow(dead_code)] // read by the unit tests only
    chars_read: usize,
}

impl<'t> Scan<'t> {
    fn new(detectors: &'static [Detector], text: &'t str) -> Self {
        Self {
            detectors,
            text,
            out: Vec::new(),
            work: segments(text),
            full_reads: 0,
            windows: 0,
            chars_read: 0,
        }
    }

    /// Every detector's spans in `piece`, counting what was read.
    fn detect(&mut self, piece: &str) -> Vec<Cand> {
        self.chars_read += piece.chars().count();
        detect(self.detectors, piece)
    }

    /// Reads every segment whole, then the text after each accepted span through windows, and
    /// returns the findings in the order they were accepted. Every step accepts a span or
    /// retires a piece, so the worklist empties.
    fn run(&mut self) -> Vec<Cand> {
        while let Some((a, b)) = self.work.pop() {
            self.full_reads += 1;
            let spans = resolve(self.detect(slice(self.text, a, b)));
            if spans.is_empty() {
                continue;
            }
            let accepted: Vec<Cand> = spans.iter().map(|c| c.moved(a)).collect();
            self.out.extend_from_slice(&accepted);
            for (p, q) in cut(a, b, &accepted) {
                if holds_content(slice(self.text, p, q)) {
                    self.read(p, q);
                }
            }
        }
        std::mem::take(&mut self.out)
    }

    /// One read of the window of `width` characters from `p` (or the rest of the piece `[p, q)`,
    /// if that is shorter), doubled while a candidate may continue past its edge. The width
    /// doubles without overflow (it saturates), and a window that holds the rest of the piece
    /// cannot reach any edge, so the doubling ends.
    fn look(&mut self, p: usize, q: usize, width: usize) -> Look<'t> {
        self.windows += 1;
        let edge = slice(self.text, p, q)
            .char_indices()
            .nth(width)
            .map_or(q, |(at, _)| p + at);
        let window = slice(self.text, p, edge);
        let candidates = self.detect(window);
        let regions = resolve(candidates.clone());
        let whole = edge == q;
        let first_end = regions.first().map(|region| region.end);
        let reaches = !whole
            && candidates
                .iter()
                .any(|c| near_edge(window, c.end) && first_end.is_none_or(|end| c.start < end));
        if reaches {
            return self.look(p, q, width.saturating_mul(2));
        }
        Look {
            window,
            regions,
            whole,
        }
    }

    /// Reads the leftover piece `[p, q)` from its start, one window at a time. A region of the
    /// window that nothing near the window's edge can change is a region the whole piece has
    /// too, so each such region is accepted and the text between them is read as pieces of
    /// their own.
    fn read(&mut self, from: usize, q: usize) {
        let mut p = from;
        // Every pass accepts a region, which moves `p` on by at least one byte, or ends the
        // read: so there are never more passes than bytes, and a step that stopped advancing
        // would end the loop with a wrong answer instead of never ending it.
        for _ in from..q {
            let Look {
                window,
                regions,
                whole,
            } = self.look(p, q, WINDOW);
            if regions.is_empty() {
                if !whole {
                    self.work.push((p, q)); // nothing in the window; the rest gets a full read
                }
                break;
            }
            let mut cursor = 0;
            for region in &regions {
                if !whole && near_edge(window, region.end) {
                    break; // near the edge: the next window reads it
                }
                self.out.push(region.moved(p));
                if holds_content(slice(window, cursor, region.start)) {
                    self.work.push((p + cursor, p + region.start)); // its right edge changed
                }
                cursor = region.end;
            }
            p += cursor;
        }
    }
}

/// The findings of `text` read through `detectors`, as `(start, end, class code)` in code
/// points.
fn engine(detectors: &'static [Detector], text: &str) -> Vec<(usize, usize, &'static str)> {
    let mut found = Scan::new(detectors, text).run();
    found.sort_by_key(|c| c.start);
    let starts: Vec<usize> = found.iter().map(|c| c.start).collect();
    let ends: Vec<usize> = found.iter().map(|c| c.end).collect();
    let (starts, ends) = (to_char_offsets(text, &starts), to_char_offsets(text, &ends));
    found
        .iter()
        .zip(starts)
        .zip(ends)
        .map(|((c, start), end)| (start, end, c.detector.code))
        .collect()
}

/// Every finding in `text`, sorted by start, as `(start, end, class code)` with offsets in code
/// points: a fixed point in which every piece of text left is clean when read as an independent
/// string.
///
/// Every detector of [`DETECTORS`] reads each piece; overlapping candidates are one region over
/// their union, named by the best-ranked member (a class that is not loose, then the earliest
/// start, then the shorter span, then registry order). Text between placeholders (`[REDACTED:`
/// and a class code, `<SECRET>`) is read as separate strings.
#[must_use]
pub fn findings(text: &str) -> Vec<(usize, usize, &'static str)> {
    engine(DETECTORS, text)
}

#[cfg(test)]
mod cost_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod window_tests;
