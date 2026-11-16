//! Unit tests of the span resolver's private parts: placeholders, the resolution rules and the
//! shortcut over text without content. The window rules are in `window_tests`, the cost of the
//! scan in `cost_tests`; the corpus and the wall-clock timing live in `tests/spans`.
use super::*;

/// The candidate of class `code` over bytes `start..end`.
pub(super) fn cand(start: usize, end: usize, code: &str) -> Cand {
    let order = DETECTORS.iter().position(|d| d.code == code).unwrap();
    Cand {
        start,
        end,
        order,
        detector: DETECTORS.get(order).unwrap(),
    }
}

/// `(start, end, code)` of each.
pub(super) fn view(spans: &[Cand]) -> Vec<(usize, usize, &'static str)> {
    spans
        .iter()
        .map(|c| (c.start, c.end, c.detector.code))
        .collect()
}

// --- placeholders and segments

#[test]
fn a_candidate_moves_on_by_an_offset_and_keeps_its_class() {
    let moved = cand(2, 5, "PII_PHONE").moved(10);
    assert_eq!((moved.start, moved.end), (12, 15));
    assert_eq!((moved.order, moved.detector.code), (1, "PII_PHONE"));
}

#[test]
fn the_placeholder_pattern_is_the_documented_one() {
    assert_eq!(
        PLACEHOLDER.as_str(),
        r"\[REDACTED:PII_[A-Z0-9_]+\]|<SECRET>"
    );
}

#[test]
fn only_the_secret_class_has_a_placeholder_of_its_own() {
    let own: Vec<(&str, &str)> = DETECTORS
        .iter()
        .filter_map(|d| d.placeholder.map(|p| (d.code, p)))
        .collect();
    assert_eq!(own, [("PII_SECRET", "<SECRET>")]);
}

#[test]
fn the_placeholder_pattern_is_built_from_the_detectors_that_have_one() {
    // Every class's own placeholder is escaped and joined after the template, in registry order.
    static OWN: [Detector; 3] = [
        Detector {
            code: "PII_X",
            loose: false,
            placeholder: Some("<A.B>"),
            find: |_| Vec::new(),
        },
        Detector {
            code: "PII_Y",
            loose: false,
            placeholder: None,
            find: |_| Vec::new(),
        },
        Detector {
            code: "PII_Z",
            loose: false,
            placeholder: Some("{z}"),
            find: |_| Vec::new(),
        },
    ];
    assert_eq!(
        placeholder_pattern(&OWN),
        r"\[REDACTED:PII_[A-Z0-9_]+\]|<A\.B>|\{z\}"
    );
    assert_eq!(placeholder_pattern(&[]), r"\[REDACTED:PII_[A-Z0-9_]+\]");
}

#[test]
fn text_without_a_placeholder_is_one_segment_and_no_text_is_none() {
    assert_eq!(segments("call me maybe"), [(0, 13)]);
    assert_eq!(segments(""), []);
}

#[test]
fn every_placeholder_is_a_cut() {
    for placeholder in [
        "[REDACTED:PII_EMAIL]",
        "[REDACTED:PII_OF_A_LATER_VERSION]",
        "<SECRET>",
    ] {
        let text = format!("ab{placeholder}cd");
        let n = placeholder.len();
        assert_eq!(segments(&text), [(0, 2), (2 + n, 4 + n)], "{placeholder}");
    }
}

#[test]
fn no_segment_is_empty_at_either_end_or_between_placeholders() {
    assert_eq!(
        segments("<SECRET>[REDACTED:PII_EMAIL]x<SECRET>"),
        [(28, 29)]
    );
    assert_eq!(segments("<SECRET><SECRET>"), []);
}

#[test]
fn an_unterminated_or_unknown_placeholder_is_text() {
    for text in ["[REDACTED:PII_EMAIL", "<SECRET", "[REDACTED:pii_email]"] {
        assert_eq!(segments(text), [(0, text.len())], "{text}");
    }
}

// --- resolve: one region per overlap, named by the best-ranked member

#[test]
fn disjoint_candidates_stay_separate_and_sorted() {
    let spans = vec![cand(10, 12, "PII_PHONE"), cand(0, 3, "PII_EMAIL")];
    assert_eq!(
        view(&resolve(spans)),
        [(0, 3, "PII_EMAIL"), (10, 12, "PII_PHONE")]
    );
}

#[test]
fn touching_candidates_are_not_one_region() {
    let spans = vec![cand(0, 3, "PII_EMAIL"), cand(3, 6, "PII_SECRET")];
    assert_eq!(
        view(&resolve(spans)),
        [(0, 3, "PII_EMAIL"), (3, 6, "PII_SECRET")]
    );
}

#[test]
fn overlapping_candidates_form_one_region_over_their_union() {
    let spans = vec![cand(0, 8, "PII_EMAIL"), cand(5, 12, "PII_SECRET")];
    assert_eq!(view(&resolve(spans)), [(0, 12, "PII_EMAIL")]);
    let spans = vec![cand(0, 8, "PII_EMAIL"), cand(2, 4, "PII_SECRET")];
    assert_eq!(view(&resolve(spans)), [(0, 8, "PII_EMAIL")]);
}

#[test]
fn a_chain_of_overlaps_is_one_region() {
    let spans = vec![
        cand(0, 4, "PII_EMAIL"),
        cand(3, 7, "PII_SECRET"),
        cand(6, 9, "PII_EMAIL"),
    ];
    assert_eq!(view(&resolve(spans)), [(0, 9, "PII_EMAIL")]);
}

#[test]
fn a_class_that_is_not_loose_wins_over_a_loose_one() {
    // A phone pattern reads digits of an address or an id; the other class names it.
    let spans = vec![cand(0, 10, "PII_PHONE"), cand(4, 8, "PII_SECRET")];
    assert_eq!(view(&resolve(spans)), [(0, 10, "PII_SECRET")]);
    let spans = vec![cand(0, 10, "PII_SECRET"), cand(4, 14, "PII_PHONE")];
    assert_eq!(view(&resolve(spans)), [(0, 14, "PII_SECRET")]);
}

#[test]
fn the_earlier_start_wins() {
    // The class that comes later in the registry, starting first, names the region.
    let spans = vec![cand(5, 12, "PII_EMAIL"), cand(0, 8, "PII_SECRET")];
    assert_eq!(view(&resolve(spans)), [(0, 12, "PII_SECRET")]);
}

#[test]
fn the_shorter_span_wins_when_they_start_together() {
    // `ab` against `ab@host.com` (an address read over it).
    let spans = vec![cand(0, 18, "PII_EMAIL"), cand(0, 2, "PII_SECRET")];
    assert_eq!(view(&resolve(spans)), [(0, 18, "PII_SECRET")]);
}

#[test]
fn registry_order_breaks_a_tie() {
    let spans = vec![cand(0, 5, "PII_SECRET"), cand(0, 5, "PII_EMAIL")];
    assert_eq!(view(&resolve(spans)), [(0, 5, "PII_EMAIL")]);
    let spans = vec![cand(0, 5, "PII_EMAIL"), cand(0, 5, "PII_SECRET")];
    assert_eq!(view(&resolve(spans)), [(0, 5, "PII_EMAIL")]);
}

#[test]
fn a_region_member_that_ranks_better_but_ends_earlier_still_names_it() {
    // The union's end is the furthest member's, whoever names it.
    let spans = vec![
        cand(0, 3, "PII_SECRET"),
        cand(1, 20, "PII_EMAIL"),
        cand(2, 9, "PII_PHONE"),
    ];
    assert_eq!(view(&resolve(spans)), [(0, 20, "PII_SECRET")]);
}

#[test]
fn the_answer_does_not_depend_on_the_order_candidates_arrive_in() {
    let spans = vec![
        cand(0, 4, "PII_EMAIL"),
        cand(3, 7, "PII_SECRET"),
        cand(20, 25, "PII_PHONE"),
        cand(22, 30, "PII_IP_ADDRESS"),
    ];
    let expected = view(&resolve(spans.clone()));
    assert_eq!(expected, [(0, 7, "PII_EMAIL"), (20, 30, "PII_IP_ADDRESS")]);
    let mut shuffled = spans;
    shuffled.reverse();
    assert_eq!(view(&resolve(shuffled.clone())), expected);
    shuffled.swap(0, 2);
    assert_eq!(view(&resolve(shuffled)), expected);
}

// --- detect and the content shortcut

static EMPTY_SPANS: [Detector; 1] = [Detector {
    code: "PII_TEST",
    loose: false,
    placeholder: None,
    find: |_| vec![(2, 2), (1, 3), (5, 4)],
}];

#[test]
fn empty_and_reversed_spans_are_dropped() {
    assert_eq!(view(&detect(&EMPTY_SPANS, "abcdef")), [(1, 3, "PII_TEST")]);
}

#[test]
fn every_detector_reads_the_piece_in_registry_order() {
    // The phone is first in the text and second in the registry.
    let found = view(&detect(DETECTORS, "+44 20 7946 0958 jane@example.com"));
    assert_eq!(found, [(17, 33, "PII_EMAIL"), (0, 16, "PII_PHONE")]);
}

#[test]
fn a_piece_holds_content_when_it_has_a_letter_a_digit_or_an_at_sign() {
    for yes in ["a", "7", "@", "_@_", "-é-", "٣", "..x.."] {
        assert!(holds_content(yes), "{yes}");
    }
    for no in ["", "_", " \t\n", ":/._-+", "[]<>()'\"`", "___"] {
        assert!(!holds_content(no), "{no}");
    }
}

#[test]
fn a_piece_that_holds_nothing_is_clean_for_every_detector() {
    // The shortcut may be no narrower than what the patterns need: over punctuation alone, a
    // detector finds something only where an at sign is.
    let alphabet: Vec<char> = "_:/.-+ \t,;=()[]<>'\"`%#?!*~^$|{}\\".chars().collect();
    let mut text = String::new();
    for first in &alphabet {
        for second in &alphabet {
            for third in &alphabet {
                text.clear();
                text.extend([first, second, third]);
                assert!(!holds_content(&text));
                assert_eq!(view(&detect(DETECTORS, &text)), [], "{text:?}");
            }
        }
    }
}

#[test]
fn offsets_come_out_in_code_points() {
    let text = "ééé jane@example.com ✓ call +44 20 7946 0958 😀😀 4111111111111111";
    assert_eq!(
        findings(text),
        [
            (4, 20, "PII_EMAIL"),
            (28, 44, "PII_PHONE"),
            (48, 64, "PII_PAYMENT_CARD")
        ]
    );
}
