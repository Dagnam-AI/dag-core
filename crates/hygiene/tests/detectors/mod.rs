//! The PII detectors: every finder against the reference's positives and near misses, the
//! checksums, the registry, and the corpus.
//!
//! Failure messages carry case names and offsets, never the text under test.
mod dates_of_birth;
mod ibans;
mod ip_addresses;
mod payment_cards;

/// A detector's finder: the byte spans of its findings.
pub(crate) type Finder = fn(&str) -> Vec<(usize, usize)>;

/// `template` with `value` in place of its `{}`, and the byte span `value` takes.
pub(crate) fn fill(template: &str, value: &str) -> (String, (usize, usize)) {
    let start = template.find("{}").unwrap();
    let text = template.replacen("{}", value, 1);
    (text, (start, start + value.len()))
}

/// The finder reports exactly `value` inside `template`, and nothing else.
pub(crate) fn assert_finds(find: Finder, name: &str, template: &str, value: &str) {
    let (text, span) = fill(template, value);
    assert_eq!(find(&text), vec![span], "{name}");
}

/// The finder reports nothing on `text`.
pub(crate) fn assert_misses(find: Finder, name: &str, text: &str) {
    assert_eq!(find(text), Vec::new(), "{name}");
}
