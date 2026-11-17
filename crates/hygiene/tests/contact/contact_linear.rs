//! Linear time on the shapes that cost the reference's patterns their memory or their time:
//! the finders run on the same text at `n` and `4n` characters and the larger takes less than
//! eight times the smaller (a quadratic read would take sixteen).
use std::time::{Duration, Instant};

use crate::reference::{find_emails, find_phones};

const SMALL: usize = 50_000;
const LARGE: usize = 4 * SMALL;
/// The smaller run is taken to last at least this long, so noise in a very fast run is not a
/// ratio.
const FLOOR: Duration = Duration::from_millis(10);
const BEST_OF: usize = 3;

/// `unit` repeated to about `size` characters, between `head` and `tail`.
fn shape(head: &str, unit: &str, tail: &str, size: usize) -> String {
    let count = size.saturating_sub(head.len() + tail.len()) / unit.len();
    format!("{head}{}{tail}", unit.repeat(count))
}

fn best_of(find: fn(&str) -> Vec<(usize, usize)>, text: &str) -> Duration {
    (0..BEST_OF)
        .map(|_| {
            let started = Instant::now();
            let found = find(text);
            let took = started.elapsed();
            std::hint::black_box(found);
            took
        })
        .min()
        .expect("at least one run")
}

fn assert_linear(
    find: fn(&str) -> Vec<(usize, usize)>,
    name: &str,
    head: &str,
    unit: &str,
    tail: &str,
) {
    let small = best_of(find, &shape(head, unit, tail, SMALL));
    let large = best_of(find, &shape(head, unit, tail, LARGE));
    assert!(
        large.as_secs_f64() < 8.0 * small.max(FLOOR).as_secs_f64(),
        "{name}: {SMALL} characters took {small:?}, {LARGE} took {large:?}"
    );
}

#[test]
fn the_phone_finder_is_linear_on_every_shape_of_digit_run() {
    // A finder that is broken fails here at once, not after every shape has been read.
    assert_eq!(find_phones("call +14155550132 now"), vec![(5, 17)]);
    assert_eq!(
        find_phones("+14155551000 +14155551001"),
        vec![(0, 12), (13, 25)]
    );
    for (name, head, unit, tail) in [
        ("one digit run", "", "1", ""),
        ("digits and spaces", "", "1 ", ""),
        ("digits and hyphens", "", "1-", ""),
        ("digit run, hyphen, digit", "", "1", "-5"),
        ("spaces between digits", "1", " ", "1"),
        ("parentheses", "", "(1)", ""),
        ("dots", "", "1.2.", ""),
        ("plus after plus", "", "+1", ""),
        ("a chain of numbers", "", "+14155550132 ", ""),
        ("a chain, hyphen apart", "", "+14155550132-", ""),
        ("numbers behind a digit", "", "415-555-0132 ", ""),
        ("separators only", "", " -.()", ""),
        ("letters", "", "a", ""),
        ("long digit run, then a number", "", "9", " +14155550132"),
    ] {
        assert_linear(find_phones, name, head, unit, tail);
    }
}

#[test]
fn the_email_finder_is_linear_on_every_shape_of_token() {
    // A finder that is broken fails here at once, not after every shape has been read.
    assert_eq!(find_emails("mail a@b.co now"), vec![(5, 11)]);
    assert_eq!(find_emails("a@b.c..d a@b.c"), vec![(0, 5), (9, 14)]);
    for (name, head, unit, tail) in [
        ("letters", "", "a", ""),
        ("a local part", "", "a", "@b.c"),
        ("domain labels", "a@b", ".c", ""),
        ("at signs", "", "a@", ""),
        ("only at signs", "", "@", ""),
        ("two heads", "", "a@b.c@", ""),
        ("a double dot, then a long run", "a@b.c..", "x", "@d.e"),
        ("many double dots", "", "a@b.c..", ""),
        ("double dots in a domain", "a@b", ".c..", ""),
        ("dots", "", ".", ""),
        ("dots after a head", "a@b.", ".", ""),
        ("hyphens before an at", "", "-", "@b.c"),
        ("a head that fails", "", "a@b", ""),
        ("a long domain, no dot", "a@", "b", ""),
        (
            "heads with long domains",
            "",
            "a@bbbbbbbbbbbbbbbbbbbbbbbb",
            "",
        ),
    ] {
        assert_linear(find_emails, name, head, unit, tail);
    }
}
