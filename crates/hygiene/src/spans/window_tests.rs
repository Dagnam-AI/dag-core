//! The window rules of the scan, shown on detectors made to show them.
use super::tests::view;
use super::*;

// --- the window rules, on detectors made to show them

/// The maximal runs of `ch` in `text` that a character `hides` does not directly precede,
/// and that `not_before` does not directly follow (a guard on both sides, as a pattern's
/// lookbehind and lookahead are).
pub(super) fn runs(text: &str, ch: char, hides: char, not_before: &str) -> Vec<(usize, usize)> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut spans = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].1 != ch {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < chars.len() && chars[j].1 == ch {
            j += 1;
        }
        let end = chars.get(j).map_or(text.len(), |&(at, _)| at);
        let hidden = i > 0 && chars[i - 1].1 == hides;
        let refused = !not_before.is_empty() && text[end..].starts_with(not_before);
        if !hidden && !refused {
            spans.push((chars[i].0, end));
        }
        i = j;
    }
    spans
}

fn literal(text: &str, needle: &str) -> Vec<(usize, usize)> {
    text.match_indices(needle)
        .map(|(at, found)| (at, at + found.len()))
        .collect()
}

/// `AAAA` is a class of its own; a run of `b` is another, hidden when `A` stands right before
/// it (the way a digit hides a phone number from its guard) until `AAAA` is a finding.
pub(super) static LOOKS: [Detector; 2] = [
    Detector {
        code: "PII_A",
        loose: false,
        placeholder: None,
        find: |text| literal(text, "AAAA"),
    },
    Detector {
        code: "PII_B",
        loose: false,
        placeholder: None,
        find: |text| runs(text, 'b', 'A', ""),
    },
];
/// As `LOOKS`, but a run of `b` is refused when `zz` follows it: a guard two characters long.
static GUARDED: [Detector; 2] = [
    Detector {
        code: "PII_A",
        loose: false,
        placeholder: None,
        find: |text| literal(text, "AAAA"),
    },
    Detector {
        code: "PII_B",
        loose: false,
        placeholder: None,
        find: |text| runs(text, 'b', 'A', "zz"),
    },
];
/// As `LOOKS`, with runs of a two-byte character.
static WIDE: [Detector; 2] = [
    Detector {
        code: "PII_A",
        loose: false,
        placeholder: None,
        find: |text| literal(text, "AAAA"),
    },
    Detector {
        code: "PII_B",
        loose: false,
        placeholder: None,
        find: |text| runs(text, 'é', 'A', ""),
    },
];

/// As `LOOKS`, with a third class: runs of `c`, refused when `zz` follows (so a window that ends
/// between the two `z` accepts what the piece refuses), and never hidden by what precedes.
static TOUCHING: [Detector; 3] = [
    Detector {
        code: "PII_A",
        loose: false,
        placeholder: None,
        find: |text| literal(text, "AAAA"),
    },
    Detector {
        code: "PII_B",
        loose: false,
        placeholder: None,
        find: |text| runs(text, 'b', 'A', ""),
    },
    Detector {
        code: "PII_C",
        loose: false,
        placeholder: None,
        find: |text| runs(text, 'c', '\0', "zz"),
    },
];

/// The maximal runs of `ch` in `text` with no `keyword` in the `reach` bytes before them: a class
/// whose pattern reads a keyword in front, which is gone once the text before it is cut away.
fn after_no_keyword(text: &str, ch: char, keyword: &str, reach: usize) -> Vec<(usize, usize)> {
    runs(text, ch, '\0', "")
        .into_iter()
        .filter(|&(start, _)| !text[start.saturating_sub(reach)..start].contains(keyword))
        .collect()
}

/// `LOOKS`, then `d` runs refused when `e` follows, then `e` runs that `AAAA` within sixteen
/// bytes before them hides. A `d` run before an `e` run is refused until the `e` run is a finding.
static KEYWORDS: [Detector; 4] = [
    Detector {
        code: "PII_A",
        loose: false,
        placeholder: None,
        find: |text| literal(text, "AAAA"),
    },
    Detector {
        code: "PII_B",
        loose: false,
        placeholder: None,
        find: |text| runs(text, 'b', 'A', ""),
    },
    Detector {
        code: "PII_D",
        loose: false,
        placeholder: None,
        find: |text| runs(text, 'd', '\0', "e"),
    },
    Detector {
        code: "PII_E",
        loose: false,
        placeholder: None,
        find: |text| after_no_keyword(text, 'e', "AAAA", 16),
    },
];

/// The findings of `text`, with offsets in bytes, and how many windows it took.
fn scan(detectors: &'static [Detector], text: &str) -> (Vec<(usize, usize, &'static str)>, usize) {
    let mut scan = Scan::new(detectors, text);
    let mut found = scan.run();
    found.sort_by_key(|c| c.start);
    (view(&found), scan.windows)
}

#[test]
fn the_fakes_hide_a_run_behind_the_class_before_it() {
    assert_eq!(runs("AAAAbbbb", 'b', 'A', ""), []);
    assert_eq!(runs("xbbbb", 'b', 'A', ""), [(1, 5)]);
    assert_eq!(runs("bbxbb", 'b', 'A', ""), [(0, 2), (3, 5)]);
    assert_eq!(runs("bbzz", 'b', 'A', "zz"), []);
    assert_eq!(runs("bbz", 'b', 'A', "zz"), [(0, 2)]);
}

#[test]
fn a_finding_hidden_by_its_neighbour_is_found_once_the_neighbour_is_cut() {
    let (found, _) = scan(&LOOKS, "AAAAbbbb");
    assert_eq!(found, [(0, 4, "PII_A"), (4, 8, "PII_B")]);
    // The whole-piece read alone does not see it.
    assert_eq!(view(&detect(&LOOKS, "AAAAbbbb")), [(0, 4, "PII_A")]);
}

#[test]
fn text_after_a_finding_without_a_letter_digit_or_at_sign_is_not_read_again() {
    let (found, windows) = scan(&LOOKS, "AAAA----");
    assert_eq!(found, [(0, 4, "PII_A")]);
    assert_eq!(windows, 0);
}

#[test]
fn a_candidate_that_ends_inside_the_margin_of_the_edge_doubles_the_window() {
    let text = format!("AAAA{}", "b".repeat(5 * WINDOW));
    let mut scan = Scan::new(&LOOKS, &text);
    let found = scan.run();
    assert_eq!(
        view(&found),
        [(0, 4, "PII_A"), (4, 4 + 5 * WINDOW, "PII_B")]
    );
    // 48, 96 and 192 characters reach the edge; 384 holds the rest; then the empty rest.
    assert_eq!(scan.windows, 5);
    assert_eq!(scan.full_reads, 1);
}

#[test]
fn a_candidate_beyond_the_margin_is_trusted_without_a_wider_window() {
    // The run ends 3 characters before the window's edge: nothing a guard looks at changes.
    let text = format!("AAAA{}zyqrs", "b".repeat(WINDOW - MARGIN));
    let (found, windows) = scan(&GUARDED, &text);
    assert_eq!(found, [(0, 4, "PII_A"), (4, 4 + WINDOW - MARGIN, "PII_B")]);
    assert_eq!(windows, 2);
}

#[test]
fn a_candidate_inside_the_margin_is_not_trusted_on_what_the_window_shows() {
    // The window holds the run and one `z`; the piece holds `zz`, which the run refuses.
    let text = format!("AAAA{}zzy", "b".repeat(WINDOW - 1));
    let (found, _) = scan(&GUARDED, &text);
    assert_eq!(found, [(0, 4, "PII_A")]);
}

#[test]
fn a_candidate_after_the_first_region_does_not_widen_the_window() {
    // `bb` is the first region. The run after `c` ends one character short of the edge and
    // looks accepted there, but `zz` follows it in the piece: widening the window for it
    // would not change the answer, only the cost, and the next window reads it anyway.
    let text = format!("AAAAbbc{}zzy", "b".repeat(44));
    let (found, windows) = scan(&GUARDED, &text);
    assert_eq!(found, [(0, 4, "PII_A"), (4, 6, "PII_B")]);
    assert_eq!(windows, 2);
}

#[test]
fn a_candidate_that_starts_where_the_first_region_ends_does_not_widen_the_window() {
    // `bb` is the first region, `c` starts right where it ends and reaches the edge, but the piece
    // refuses it (`zz` follows): widening the window would not change the answer.
    let text = format!("AAAAbb{}zzy", "c".repeat(45));
    let (found, windows) = scan(&TOUCHING, &text);
    assert_eq!(found, [(0, 4, "PII_A"), (4, 6, "PII_B")]);
    assert_eq!(windows, 2);
}

#[test]
fn the_text_between_two_regions_of_a_window_is_read_again_without_its_right_neighbour() {
    // The window finds `bbbbb` and `ee`; `dd` between them was refused for the `e` after it, and
    // is a finding once `ee` is cut away. (`bbbbb` is longer than the `AAAA` before it, so a piece
    // taken from the wrong end would start before the window.)
    let text = "AAAAbbbbbddee";
    let (found, _) = scan(&KEYWORDS, text);
    assert_eq!(
        found,
        [
            (0, 4, "PII_A"),
            (4, 9, "PII_B"),
            (9, 11, "PII_D"),
            (11, 13, "PII_E")
        ]
    );
    assert_eq!(view(&detect(&KEYWORDS, text)), [(0, 4, "PII_A")]);
}

#[test]
fn the_window_counts_characters_not_bytes() {
    let text = format!("AAAA{}", "é".repeat(60));
    let (found, windows) = scan(&WIDE, &text);
    assert_eq!(found, [(0, 4, "PII_A"), (4, 4 + 120, "PII_B")]);
    assert_eq!(windows, 3);
}

#[test]
fn the_rest_of_a_piece_is_read_whole_when_its_window_holds_nothing() {
    // The full read finds `AAAA` and the `e` (`dd` is refused for the `e` after it). The window
    // after `AAAA` holds only `x` and stops short of the piece's end, so it cannot say there is
    // nothing further on: the rest of the piece is read whole, which finds `dd` once `e` is cut.
    let text = format!("AAAA{}dde", "x".repeat(60));
    let mut scan = Scan::new(&KEYWORDS, &text);
    let mut found = scan.run();
    found.sort_by_key(|c| c.start);
    assert_eq!(
        view(&found),
        [(0, 4, "PII_A"), (64, 66, "PII_D"), (66, 67, "PII_E")]
    );
    // The first read, the rest after `AAAA` (which finds `dd`), then the `x`s before `dd` again.
    assert_eq!(scan.full_reads, 3);
    assert_eq!(scan.windows, 2);
}
