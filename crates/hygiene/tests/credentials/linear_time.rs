//! Time: a token that grows must cost time that grows with it, not with its square, whatever
//! shape it takes. Each shape in `shapes.jsonl` is a head, a unit repeated and a tail, cut from
//! the credentials corpus and joined with single neutral characters (see `corpus.rs`). Some
//! shapes hold a credential the finder reports (the row's `f`), the others only the heads and
//! names it must read and refuse; the test times both, and checks on the smallest text that
//! each shape still does what its row says, so a cut that drifted fails instead of timing
//! nothing.
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use crate::reference::find_secrets;
use serde::Deserialize;

use super::corpus::{Part, assemble};

/// The characters of the smaller of the two timed texts.
const SMALL: usize = 4_000;
/// How many times larger the larger timed text is than the smaller.
const FACTOR: usize = 4;
/// A sample so small that any finder, even a quadratic one, answers it at once.
const TINY: usize = 500;

/// The longest one run may take: a finder that does not finish in this long is not linear, and
/// the test says so instead of waiting for it. A linear finder reads the largest text here in
/// milliseconds, even in a debug build on a busy machine; a quadratic one that is stuck must
/// fail the test well inside the 20 seconds `cargo mutants` allows a run.
const PATIENCE: Duration = Duration::from_secs(2);

/// One line of `shapes.jsonl`.
#[derive(Deserialize)]
struct Shape {
    #[serde(rename = "n")]
    name: String,
    /// Whether the finder reports something in the shape's text at `TINY` characters.
    #[serde(rename = "f")]
    finds: bool,
    #[serde(rename = "h")]
    head: Vec<Part>,
    #[serde(rename = "u")]
    unit: Vec<Part>,
    #[serde(rename = "t")]
    tail: Vec<Part>,
}

/// `head`, as many `unit`s as fit in `size` characters, `tail`.
fn text_of(head: &str, unit: &str, tail: &str, size: usize) -> String {
    let count = size.saturating_sub(head.len() + tail.len()) / unit.len();
    format!("{head}{}{tail}", unit.repeat(count))
}

/// How long one run of the finder over `text` takes, read on a thread of its own so that a run
/// that never ends fails the test. (The thread is left behind; the test binary's exit ends it.)
fn timed(text: &Arc<String>) -> Result<Duration, String> {
    let (sender, receiver) = mpsc::channel();
    let text = Arc::clone(text);
    std::thread::spawn(move || {
        let start = Instant::now();
        std::hint::black_box(find_secrets(&text));
        // The receiver has gone when the run outlived its patience; nobody is left to tell.
        let _ = sender.send(start.elapsed());
    });
    receiver
        .recv_timeout(PATIENCE)
        .map_err(|_| format!("a run did not finish in {PATIENCE:?}"))
}

/// The best of three runs of the finder over `text`.
fn best_of_three(text: &Arc<String>) -> Result<Duration, String> {
    let mut best = timed(text)?;
    for _ in 0..2 {
        best = best.min(timed(text)?);
    }
    Ok(best)
}

/// Whether four times the text took less than eight times as long. A quiet machine settles it
/// with one run of each; a busy one (parallel tests) gets two more attempts with the best of
/// three runs each, because a quadratic finder fails every attempt and noise does not.
fn scales_linearly(small: &Arc<String>, large: &Arc<String>) -> Result<(), String> {
    // A floor keeps timer noise on a fast finder from deciding the ratio.
    let floor = Duration::from_millis(5);
    let within = |a: Duration, b: Duration| b < 8 * a.max(floor);
    let (a, b) = (timed(small)?, timed(large)?);
    if within(a, b) {
        return Ok(());
    }
    let mut last = format!("{a:?} then {b:?}");
    for _ in 0..2 {
        let (a, b) = (best_of_three(small)?, best_of_three(large)?);
        if within(a, b) {
            return Ok(());
        }
        last = format!("{a:?} then {b:?}");
    }
    Err(format!(
        "{} characters then {}: {last}",
        small.len(),
        large.len()
    ))
}

#[test]
fn four_times_the_text_takes_less_than_eight_times_as_long() {
    let shapes: Vec<Shape> = include_str!("shapes.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(shapes.len(), 45);
    for shape in shapes {
        let (head, unit, tail) = (
            assemble(&shape.head),
            assemble(&shape.unit),
            assemble(&shape.tail),
        );
        // A finder that cannot answer even this in a moment is not slow, it is stuck.
        let tiny = Arc::new(text_of(&head, &unit, &tail, TINY));
        if let Err(took) = timed(&tiny) {
            panic!("{}: {took}", shape.name);
        }
        assert_eq!(
            !find_secrets(&tiny).is_empty(),
            shape.finds,
            "{}: the finder's answer on the shape is not the row's",
            shape.name
        );
        let small = Arc::new(text_of(&head, &unit, &tail, SMALL));
        let large = Arc::new(text_of(&head, &unit, &tail, SMALL * FACTOR));
        if let Err(took) = scales_linearly(&small, &large) {
            panic!("{}: {took}", shape.name);
        }
    }
}
