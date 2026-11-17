//! The two finders that read a run of text in code: phone numbers and email addresses.
//!
//! A number is found by a pattern and then read on, and an address by its head and then its
//! domain, in code rather than as one repeated regex group: a group that repeats once per
//! character (or per label) keeps a stack entry per repetition in a backtracking engine, which
//! for one unbroken megabyte cost the reference 60 MB or more. Here every step is a bounded
//! match or a single pass, so memory stays flat and time linear.
use crate::detectors::{Pattern, compile, group_span, scan};
use crate::text::char_before;
use crate::unicode::{
    UnicodeVersion, Versioned, digit_class, is_py_digit, is_py_space, is_py_word, space_class,
    word_class,
};

/// A phone number's body and the end it needs: `\+?\d`, seven to twenty digits, separators,
/// dots and parentheses, `\d`, in a group; then a terminator that is looked at and left, which
/// is what the reference's `(?![\d]|[ -]\d)` allows: the end, a character that is not a digit,
/// hyphen or space, or a space or hyphen that no digit follows (`docs/patterns.md`, P01 and P02;
/// the terminator lets a shorter end through where the longest meets a digit, as Python's
/// backtracking does).
fn number(version: UnicodeVersion) -> String {
    let (d, s) = (digit_class(version), space_class(version));
    format!(r"(\+?{d}[{d}{s}.()\-]{{7,20}}{d})(?:\z|[^{d}\- ]|[ \-](?:\z|[^{d}]))")
}

/// The number pattern of the reference's `_PHONE_CANDIDATE`; its lookbehinds are [`lead_ok`].
static CANDIDATE: Pattern = Versioned::new(|version| compile(&number(version)));

/// The reference's `_PHONE_NEXT`: a number right after an accepted one, across at most one space
/// or hyphen, anchored at the start of the text it is given (no lookbehind).
static NEXT: Pattern = Versioned::new(|version| compile(&format!(r"\A[ \-]?{}", number(version))));

/// The reference's `_EMAIL_HEAD`: a local part and its `@`; the lookbehind is [`head_start_ok`].
static EMAIL_HEAD: Pattern = Versioned::new(|version| {
    let w = word_class(version);
    compile(&format!(r"[{w}.+\-]+@"))
});

/// The reference's `_DOMAIN_RUN`: the (possibly empty) run of word characters, dots and hyphens.
static DOMAIN_RUN: Pattern = Versioned::new(|version| {
    let w = word_class(version);
    compile(&format!(r"[{w}.\-]*"))
});

/// `(?<![\d+])(?<![\d][ -])`: not after a digit or a plus, and not after a digit and then a
/// space or hyphen. That last half is what stops a 16-digit card number yielding a 12-digit
/// "phone" starting at its second group.
fn lead_ok(text: &str, start: usize, version: UnicodeVersion) -> bool {
    match char_before(text, start) {
        Some(c) if is_py_digit(version, c) || c == '+' => false,
        Some(' ' | '-') => !char_before(text, start - 1).is_some_and(|c| is_py_digit(version, c)),
        _ => true,
    }
}

/// The filters on top of the pattern, each removing a false positive the reference's corpus
/// produced: a leading `+` admits 9 to 15 digits, without one the floor rises to 10 (which is
/// what stops a dashed 9-digit US SSN reading as a phone number); and at least one separator or
/// a leading `+` is required (a bare digit run is an order id far more often than a phone
/// number).
fn phone_ok(raw: &str, version: UnicodeVersion) -> bool {
    let digits = raw.chars().filter(|&c| is_py_digit(version, c)).count();
    let international = raw.starts_with('+');
    if !(if international { 9 } else { 10 }..=15).contains(&digits) {
        return false;
    }
    international
        || raw
            .chars()
            .any(|c| is_py_space(version, c) || matches!(c, '.' | '(' | ')' | '-'))
}

/// Spans that look like a dialable number rather than a long id.
///
/// A list of numbers separated by one space or hyphen is found whole. No trailing digit is
/// allowed, so a 23-digit id cannot yield a 14-digit "phone" from its prefix.
///
/// The chain reads on from a number only as far as the next number the pattern finds with its
/// lookbehinds: the body of a number takes digits, spaces and parentheses, so left alone it
/// would run into the area code of a parenthesised number that follows and take its `(415` with
/// it, stranding the seven digits behind.
#[must_use]
pub fn find_phones(text: &str, version: UnicodeVersion) -> Vec<(usize, usize)> {
    let ordinary: Vec<(usize, usize)> = scan(CANDIDATE.get(version), text, |start, caps| {
        group_span(caps, 1).filter(|_| lead_ok(text, start, version))
    })
    .into_iter()
    .filter(|&(from, to)| text.get(from..to).is_some_and(|raw| phone_ok(raw, version)))
    .collect();
    let mut spans = Vec::new();
    for (index, &(start, end)) in ordinary.iter().enumerate() {
        spans.push((start, end));
        let stop = ordinary.get(index + 1).map_or(text.len(), |next| next.0);
        spans.extend(chain(end, |position| {
            next_number(text, position, stop, version)
        }));
    }
    spans
}

/// The numbers read on from `position`, each found by `next` from the end of the one before.
///
/// A link that does not end past where it was read from is refused (the text a finder reads is
/// finite, so every link must move on), so a `next` that stops advancing ends the chain instead
/// of filling memory with the same link.
fn chain(
    mut position: usize,
    next: impl Fn(usize) -> Option<(usize, usize)>,
) -> Vec<(usize, usize)> {
    let mut links = Vec::new();
    while let Some((from, to)) = next(position).filter(|&(_, to)| to > position) {
        links.push((from, to));
        position = to;
    }
    links
}

/// The accepted number that follows `position` across at most one space or hyphen, in the text
/// that ends at `stop`.
fn next_number(
    text: &str,
    position: usize,
    stop: usize,
    version: UnicodeVersion,
) -> Option<(usize, usize)> {
    text.get(position..stop).and_then(|rest| {
        let (from, to) = NEXT
            .get(version)
            .captures(rest)
            .and_then(|caps| group_span(&caps, 1))?;
        rest.get(from..to)
            .filter(|number| phone_ok(number, version))
            .map(|_| (position + from, position + to))
    })
}

/// Whether a head of an address may start at `start`: the character before is not a word
/// character, `.`, `+` or `-` (the reference's `(?<![\w.+-])`).
fn head_start_ok(text: &str, start: usize, version: UnicodeVersion) -> bool {
    !char_before(text, start)
        .is_some_and(|c| is_py_word(version, c) || matches!(c, '.' | '+' | '-'))
}

/// The end of the domain that begins at `start`: labels of word characters and hyphens between
/// single dots, at least one dot; `None` when there is none.
fn domain_end(text: &str, start: usize, version: UnicodeVersion) -> Option<usize> {
    DOMAIN_RUN
        .get(version)
        .find_at(text, start)
        .and_then(|run| {
            let run = run.as_str();
            let labels = run.split_once("..").map_or(run, |(first, _)| first);
            let labels = labels.trim_end_matches('.');
            labels
                .find('.')
                .filter(|&dot| dot != 0)
                .map(|_| start + labels.len())
        })
}

/// Every address. `pw@host` after a URL password is one too: where the two classes overlap, the
/// scan's overlap resolution names the region a secret.
#[must_use]
pub fn find_emails(text: &str, version: UnicodeVersion) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut position = 0;
    while let Some(head) = EMAIL_HEAD.get(version).find_at(text, position) {
        // Every position inside a head's run of head characters follows one and is refused by
        // the reference's lookbehind, so a refused head and a head with no domain both resume
        // after the `@`, which is what the reference's one-character restarts come to.
        let domain = if head_start_ok(text, head.start(), version) {
            domain_end(text, head.end(), version)
        } else {
            None
        };
        if let Some(end) = domain {
            spans.push((head.start(), end));
        }
        position = resume(domain, head.end());
    }
    spans
}

/// Where the address search resumes: at the end of the domain it just read, or after the head's
/// `@` when there is none.
///
/// The `max` is a progress guard and nothing else: a search position never moves back to or
/// before the `@` it has just passed, so no result of `domain_end` can hold the loop in place.
fn resume(domain: Option<usize>, after_head: usize) -> usize {
    domain.map_or(after_head, |end| end.max(after_head))
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::{chain, resume};

    /// A `next` that answers `answer(position)` for at most five calls and then nothing, with
    /// the number of calls it got, so a chain that stops refusing a link cannot run on.
    fn links(answer: fn(usize) -> (usize, usize)) -> (Vec<(usize, usize)>, usize) {
        let calls = Cell::new(0);
        let found = chain(12, |position| {
            calls.set(calls.get() + 1);
            (calls.get() <= 5).then(|| answer(position))
        });
        (found, calls.get())
    }

    #[test]
    fn a_link_that_ends_where_it_was_read_from_ends_the_chain() {
        assert_eq!(links(|position| (position, position)), (vec![], 1));
    }

    #[test]
    fn a_link_that_ends_before_where_it_was_read_from_ends_the_chain() {
        assert_eq!(links(|_| (1, 1)), (vec![], 1));
    }

    #[test]
    fn links_that_move_on_are_all_taken_until_the_reader_has_none() {
        let (found, calls) = links(|position| (position + 1, position + 13));
        assert_eq!(
            found,
            vec![(13, 25), (26, 38), (39, 51), (52, 64), (65, 77)]
        );
        assert_eq!(calls, 6);
    }

    #[test]
    fn the_search_resumes_at_the_domains_end_or_after_the_at() {
        assert_eq!(resume(Some(25), 10), 25);
        assert_eq!(resume(None, 10), 10);
    }

    #[test]
    fn a_domain_end_that_does_not_move_on_cannot_move_the_search_back() {
        assert_eq!(resume(Some(0), 10), 10);
        assert_eq!(resume(Some(9), 10), 10);
        assert_eq!(resume(Some(10), 10), 10);
    }
}
