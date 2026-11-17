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
/// The characters of the text where the cost of reading names inside a credential shows (it
/// does not at the 4,000 and 16,000 of the shapes above).
const LONG: usize = 2_048_000;
/// A sample so small that any finder, even a quadratic one, answers it at once.
const TINY: usize = 500;

/// The longest one run may take: a finder that does not finish in this long is not linear, and
/// the run is given up on (one failed attempt, see `retried`) instead of waited for. An honest run
/// reads the largest text here in about a second, even in a debug build under coverage on a shared
/// runner; this leaves it twenty times that, and a hung finder is still bounded, because the run
/// is read through a channel with this timeout. `cargo mutants` gives each mutant a computed
/// limit (about 288 s on CI) and counts a mutant that outlives it as caught, so a stuck mutant
/// dies by whichever of the two comes first.
const PATIENCE: Duration = Duration::from_secs(20);
/// How many attempts a timing check gets: it passes when any one does. A quadratic finder fails
/// every attempt; a machine that was busy for one of them does not fail the next.
const ATTEMPTS: usize = 3;

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

/// The best of `runs` runs of the finder over `text`.
fn best_of(runs: usize, text: &Arc<String>) -> Result<Duration, String> {
    let mut best = timed(text)?;
    for _ in 1..runs {
        best = best.min(timed(text)?);
    }
    Ok(best)
}

/// Runs `check` (given the attempt's number) until one attempt returns `Ok`, at most `ATTEMPTS`
/// times; the error is the last attempt's. A quiet machine settles a check with the first
/// attempt, which makes one run of each text; a busy one gets more, each the best of three runs,
/// because a quadratic finder fails every attempt and noise does not. A run that outlives
/// `PATIENCE` fails its attempt like any other measurement.
fn retried(mut check: impl FnMut(usize) -> Result<(), String>) -> Result<(), String> {
    let mut last = Ok(());
    for attempt in 0..ATTEMPTS {
        last = check(attempt);
        if last.is_ok() {
            break;
        }
    }
    last
}

/// The runs an attempt makes of each text: one, then the best of three.
fn runs_of(attempt: usize) -> usize {
    if attempt == 0 { 1 } else { 3 }
}

/// Whether a text `k` times as large took less than `2k` times as long. A floor keeps timer
/// noise on a fast finder from deciding the ratio.
fn scales_linearly(small: &Arc<String>, large: &Arc<String>) -> Result<(), String> {
    let floor = Duration::from_millis(5);
    let ratio = u32::try_from(2 * large.len() / small.len()).unwrap();
    retried(|attempt| {
        let runs = runs_of(attempt);
        let (a, b) = (best_of(runs, small)?, best_of(runs, large)?);
        if b < ratio * a.max(floor) {
            return Ok(());
        }
        Err(format!(
            "{} characters then {}: {a:?} then {b:?}",
            small.len(),
            large.len()
        ))
    })
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

/// How many times as long as its control the text with names inside a credential may take. Both
/// are the same length and made of the same characters, so a slow or time-shared machine slows
/// both and the ratio cancels (about 4.7 on a quiet machine, 5.5 to 6.2 under three times as many
/// busy processes as cores). The control reads no name, so the ratio is not 1: the finder still
/// finds each name in the shape and passes it over. With that skip removed (`taken_end` never
/// passes a name over) it reads the rest of the credential for every name, and the ratio is
/// 27.4 to 29.0 in a debug build (8.7 to 8.9 s against 0.31 s); at half the size it is 16.2 to
/// 17.3, which is why the text is this long. The limit sits between the two, 2.4 times from each.
const CONTROL_LIMIT: u32 = 12;

/// Whether `shape` took less than `CONTROL_LIMIT` times as long as `control`, in one of the
/// attempts. A floor keeps timer noise on a fast finder from deciding the ratio.
fn costs_no_more_than_its_control(
    shape: &Arc<String>,
    control: &Arc<String>,
) -> Result<(), String> {
    let floor = Duration::from_millis(5);
    retried(|_| {
        let (a, b) = (timed(control)?, timed(shape)?);
        if b < CONTROL_LIMIT * a.max(floor) {
            return Ok(());
        }
        Err(format!(
            "{} characters took {b:?}, over {CONTROL_LIMIT} times the {a:?} that the same characters take with no name inside a credential",
            shape.len()
        ))
    })
}

/// Every name inside a credential taken whose value starts inside it is passed over, not judged:
/// judged, each would read the rest of the credential, so the cost would grow with the square of
/// its length. That cost shows only on a text far larger than the shapes above use, so this is a
/// text of its own: a name (case 1908's `secret=`) and three characters, which make a credential
/// that takes the rest of the text, then a name (case 1958's `password=`) repeated inside it. The
/// control is the same text with each name reversed, so it is a name no longer.
#[test]
fn names_inside_a_credential_taken_are_passed_over() {
    let fill = |c: &str| Part::Fill(c.into());
    let head = assemble(&[Part::Cut(1908, 12, 19), fill("Z"), fill("x"), fill("9")]);
    let unit = assemble(&[Part::Cut(1958, 8, 17)]);
    let backwards: String = unit.chars().rev().collect();
    let text_of = |unit: &str| Arc::new(text_of(&head, unit, "", LONG));
    let (shape, control) = (text_of(&unit), text_of(&backwards));
    assert_eq!(find_secrets(&shape).len(), 1);
    assert_eq!(find_secrets(&control).len(), 1);
    if let Err(took) = costs_no_more_than_its_control(&shape, &control) {
        panic!("{took}");
    }
}
