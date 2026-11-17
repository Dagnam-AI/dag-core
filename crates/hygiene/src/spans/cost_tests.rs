//! The cost of the scan, in characters the detectors were handed (`chars_read`) and in reads, not
//! in time: a count says the same on a loaded machine, in a debug build and under coverage, and
//! it cannot pass for a reason of its own the way a ratio of two short timings can. The
//! wall-clock check on the whole `findings` is `tests/spans/linear_time.rs`.
use super::tests::VERSION;
use super::window_tests::{LOOKS, runs};
use super::*;

/// Alternating `a` and `b`: a run of either is hidden by the other before it, so a full read finds
/// the first character only, and each next one is found only once the one before it is cut away.
static ALTERNATING: [Detector; 2] = [
    Detector {
        code: "PII_A",
        loose: false,
        placeholder: None,
        find: |text, _| runs(text, 'a', 'b', ""),
    },
    Detector {
        code: "PII_B",
        loose: false,
        placeholder: None,
        find: |text, _| runs(text, 'b', 'a', ""),
    },
];

/// Read `text` through `detectors`; the counters of the scan.
fn counters(detectors: &'static [Detector], text: &str) -> Scan<'static> {
    let text: &'static str = Box::leak(text.to_owned().into_boxed_str());
    let mut scan = Scan::new(detectors, text, VERSION);
    scan.run();
    scan
}

#[test]
fn the_scan_counts_the_characters_of_every_whole_read_and_window() {
    // One whole read of 8 characters, then the window after `AAAA`, which holds `bbbb`, then
    // the window after `bbbb`, which is empty.
    let scan = counters(&LOOKS, "AAAAbbbb");
    assert_eq!((scan.full_reads, scan.windows), (1, 2));
    assert_eq!(scan.chars_read, 8 + 4);
    // Characters, not bytes.
    assert_eq!(counters(&LOOKS, "éé").chars_read, 2);
}

#[test]
fn a_chain_of_findings_that_each_hide_the_next_costs_a_window_apiece() {
    for links in [200, 800] {
        let scan = counters(&ALTERNATING, &"ab".repeat(links / 2));
        assert_eq!(scan.full_reads, 1, "{links} links");
        assert_eq!(scan.windows, links - 1, "{links} links");
        // The whole text once, then a window of up to 48 characters for every link after the first.
        let windows: usize = (1..links).map(|k| WINDOW.min(links - k)).sum();
        assert_eq!(scan.chars_read, links + windows, "{links} links");
        // The cost is a constant number of characters per character of the text.
        assert!(scan.chars_read <= (1 + WINDOW) * links, "{links} links");
    }
}

#[test]
fn a_text_of_many_findings_costs_one_whole_read_and_a_window_per_finding() {
    // Each of the 60 findings is read whole at once; the text before each one is a window of
    // its own (nothing in it is hidden: the cost does not grow with the number of findings).
    let links = 60;
    let text = "Aadhaar Number 580927525309".repeat(links);
    let mut scan = Scan::new(DETECTORS, &text, VERSION);
    assert_eq!(scan.run().len(), links);
    assert_eq!(scan.full_reads, 1);
    assert_eq!(scan.windows, links);
}

/// `head`, then `unit` repeated to about `size` characters, then `tail`.
fn shape(head: &str, unit: &str, tail: &str, size: usize) -> String {
    let count = size.saturating_sub(head.len() + tail.len()) / unit.len();
    format!("{head}{}{tail}", unit.repeat(count))
}

#[test]
fn a_long_run_of_one_shape_is_read_in_a_constant_number_of_characters_per_character() {
    for (name, head, unit, tail) in [
        ("digits", "", "1", ""),
        ("digits and spaces", "", "1 ", ""),
        ("digits and hyphens", "", "1-", ""),
        ("digits and dots", "", "1.", ""),
        ("digit groups", "", "12 345 ", ""),
        ("phone-like groups", "", "+1 (415) 555 ", ""),
        ("a word character", "", "a", ""),
        ("short words", "", "ab ", ""),
        ("at signs", "", "a@", ""),
        ("an at sign between dots", "", ".@.", ""),
        ("a local part", "", "a.", "@b.co"),
        ("domain labels", "a@", "b.", "co"),
        ("a long label", "a@", "b", ".co"),
        ("dates", "", "1/1/", ""),
        ("separators", "", "- ", ""),
        (
            "a keyword and digits",
            "",
            "Aadhaar Number 580927525309",
            "",
        ),
        ("a keyword and a group", "", "SIN 130 692 544 ", ""),
    ] {
        for size in [4_000, 16_000] {
            let text = shape(head, unit, tail, size);
            let scan = counters(DETECTORS, &text);
            let len = text.chars().count();
            assert!(
                scan.chars_read <= MOST_READ_PER_CHAR * len,
                "{name}: {} characters read of {len}",
                scan.chars_read
            );
        }
    }
}

/// How many characters of the text the scan may be handed, per character of it. It catches a
/// read that reads text again: the text once, plus a window of at most 48 after each finding. Eleven
/// of the shapes above find nothing, so for them `chars_read == len` by construction and the bound
/// cannot fail; the other six measure at most 1.6. A whole-rest window inside `read` is not
/// caught here (real detectors build no hiding chain): `ALTERNATING` pins that.
const MOST_READ_PER_CHAR: usize = 3;
