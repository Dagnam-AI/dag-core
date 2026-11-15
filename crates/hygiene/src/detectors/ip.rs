//! IPv4 and IPv6 addresses: a candidate pattern, the standard parser, and the reference's rules
//! for what is not personal data.
use std::net::{IpAddr, Ipv6Addr};
use std::sync::LazyLock;

use regex::Regex;

use super::{Pattern, compile, group_span, scan};
use crate::text::{
    DIGIT_CLASS, SPACE_CLASS, WORD_CLASS, case_sensitive, char_before, is_py_digit, is_py_word,
};

/// How many characters before a candidate the version rule reads.
const WINDOW: usize = 32;

/// `\d{1,3}` four times joined by dots, then a character that is not a word character or a
/// dot, or a dot with no word character after it, or the end (`docs/patterns.md`, P22).
static IPV4: Pattern = LazyLock::new(|| {
    let (d, w) = (DIGIT_CLASS.as_str(), WORD_CLASS.as_str());
    compile(&format!(
        r"((?:{d}{{1,3}}\.){{3}}{d}{{1,3}})(?:\z|[^{w}.]|\.(?:\z|[^{w}]))"
    ))
});

/// Up to seven groups of hex digits and colons, then an IPv4 tail or one more group, then a
/// character that is not a word character or a colon, or a dot with no word character after
/// it, or the end (`docs/patterns.md`, P23).
static IPV6: Pattern = LazyLock::new(|| {
    let (d, w) = (DIGIT_CLASS.as_str(), WORD_CLASS.as_str());
    compile(&format!(
        r"((?:[0-9A-Fa-f]{{0,4}}:){{2,7}}(?:{d}{{1,3}}(?:\.{d}{{1,3}}){{3}}|[0-9A-Fa-f]{{0,4}}))(?:\z|[^{w}:.]|\.(?:\z|[^{w}]))"
    ))
});

/// A version right after its product (`Chrome/120.0.0.0`) or keyword (`build 3.2.1.9`).
///
/// The product is a token after whitespace or `(`/`;`, so a URL's `//` and a path's `/srv/` are
/// not one: an address there is an address. Written for `(?i)` as `docs/patterns.md` P24
/// has it: `i` also reads U+0130 and U+0131, and every word class is wrapped so the flag does
/// not fold it (U+0345 would join); `\b` before `v` is the start or a non-word character.
static VERSION_BEFORE: Pattern = LazyLock::new(|| {
    let (s, w) = (SPACE_CLASS.as_str(), WORD_CLASS.as_str());
    let word = case_sensitive(w);
    let not_word = case_sensitive(&format!("[^{w}]"));
    let i = r"[i\u{130}\u{131}]";
    compile(&format!(
        r"(?i)(?:[{s}(;][A-Za-z\u{{130}}\u{{131}}](?:{word}|[.+\-])*/|(?:\A|{not_word})v|(?:\A|{not_word})ver\.?|vers{i}on|bu{i}ld|f{i}rmware|release){s}*\z"
    ))
});

/// Whether the text just ahead of a candidate (`before`, one space standing for the start of
/// the text) ends in a version's product or keyword.
fn follows_a_version(before: &str) -> bool {
    VERSION_BEFORE.is_match(before)
}

/// Whether `candidate`, found by the patterns, is a personal IP address; `before` is the text
/// just ahead of it, after a space that stands for the text's start.
///
/// The standard parser has the last word, less loopback and unspecified addresses (an
/// IPv4-mapped address counts as the IPv4 address it holds). An IPv4 address that is a network
/// id (last octet 0) or follows a version's product or keyword is a version. An IPv6 address
/// needs a digit and three groups or a hex letter: a slice (`a[1::2]`, `b[100::200]`) parses
/// as IPv6 but is at most `start::step`, all decimal.
#[must_use]
pub fn is_ip_address(candidate: &str, before: &str) -> bool {
    if !candidate.chars().any(is_py_digit) {
        return false;
    }
    match candidate.parse::<IpAddr>() {
        Err(_) => false,
        Ok(IpAddr::V4(v4)) => {
            !(v4.is_loopback()
                || v4.is_unspecified()
                || candidate.ends_with(".0")
                || follows_a_version(before))
        }
        Ok(IpAddr::V6(v6)) => {
            !(is_loopback(v6) || is_unspecified(v6))
                && (candidate
                    .split(':')
                    .filter(|group| !group.is_empty())
                    .count()
                    >= 3
                    || candidate
                        .chars()
                        .any(|c| matches!(c, 'A'..='F' | 'a'..='f')))
        }
    }
}

/// `::1`, or an IPv4-mapped loopback address (as Python 3.13 reads it).
fn is_loopback(v6: Ipv6Addr) -> bool {
    v6.is_loopback() || v6.to_ipv4_mapped().is_some_and(|v4| v4.is_loopback())
}

/// `::`, or the IPv4-mapped unspecified address.
fn is_unspecified(v6: Ipv6Addr) -> bool {
    v6.is_unspecified() || v6.to_ipv4_mapped().is_some_and(|v4| v4.is_unspecified())
}

/// The text just ahead of a candidate that starts at byte `start`: its last `WINDOW` characters,
/// after a space that stands for the start of the text.
fn window_before(text: &str, start: usize) -> String {
    let mut ahead: Vec<char> = text
        .get(..start)
        .into_iter()
        .flat_map(|head| head.chars().rev().take(WINDOW))
        .collect();
    ahead.push(' ');
    ahead.into_iter().rev().collect()
}

/// `spans` sorted, overlapping ones merged: one finding per stretch of text.
fn union(mut spans: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    spans.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in spans {
        match merged.last_mut() {
            Some(last) if start < last.1 => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }
    merged
}

/// The candidates of `pattern` that are personal addresses and do not follow a word character
/// or one of `refused`.
fn addresses(pattern: &Regex, text: &str, refused: &[char]) -> Vec<(usize, usize)> {
    let candidates = scan(pattern, text, |start, caps| {
        group_span(caps, 1).filter(|_| {
            !char_before(text, start).is_some_and(|c| is_py_word(c) || refused.contains(&c))
        })
    });
    candidates
        .into_iter()
        .filter(|&(from, to)| {
            text.get(from..to)
                .is_some_and(|candidate| is_ip_address(candidate, &window_before(text, from)))
        })
        .collect()
}

/// IPv4 and IPv6 addresses, by [`is_ip_address`].
#[must_use]
pub fn find_ip_addresses(text: &str) -> Vec<(usize, usize)> {
    let mut spans = addresses(&IPV4, text, &['.']);
    spans.extend(addresses(&IPV6, text, &[':', '.']));
    union(spans)
}

#[cfg(test)]
mod tests {
    use super::union;

    #[test]
    fn the_union_merges_overlapping_spans_and_keeps_touching_ones_apart() {
        assert_eq!(union(vec![(4, 9), (0, 3), (2, 5)]), vec![(0, 9)]);
        assert_eq!(union(vec![(0, 3), (3, 5)]), vec![(0, 3), (3, 5)]);
        assert_eq!(union(vec![(0, 9), (2, 5)]), vec![(0, 9)]);
        assert_eq!(union(vec![]), vec![]);
    }
}
