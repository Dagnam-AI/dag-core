//! The phone and email finders: the reference's positives and near misses as exact spans, the
//! two-stage reads, the shapes that must stay linear, and the corpus.
//!
//! Failure messages carry case names and offsets, never the text under test. Every number and
//! address here is synthetic: `555-01xx` numbers, reserved example domains, or values built in
//! the test.
mod contact_corpus;
mod contact_linear;
mod email_addresses;
mod email_domains;
mod phone_numbers;
mod phone_runs;

/// A finder: the byte spans of its findings.
pub(crate) type Finder = fn(&str) -> Vec<(usize, usize)>;

/// The byte spans of `needles` in `text`, each searched for after the end of the one before.
pub(crate) fn spans_of(text: &str, needles: &[&str]) -> Vec<(usize, usize)> {
    let mut from = 0;
    needles
        .iter()
        .map(|needle| {
            let start = from
                + text[from..]
                    .find(needle)
                    .expect("the needle is in the text");
            from = start + needle.len();
            (start, from)
        })
        .collect()
}

/// The finder reports exactly `needles` (in order) in `text`, and nothing else.
pub(crate) fn assert_reads(find: Finder, name: &str, text: &str, needles: &[&str]) {
    assert_eq!(find(text), spans_of(text, needles), "{name}");
}

/// The finder reports nothing on `text`.
pub(crate) fn assert_none(find: Finder, name: &str, text: &str) {
    assert_eq!(find(text), Vec::new(), "{name}");
}
