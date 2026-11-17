//! Credentials by their shape alone, wherever they stand: provider keys, PEM private-key blocks,
//! `Bearer` and `Basic` values, a connection string's `pwd`, and a URL's password; plus the
//! two checks that tell a value is a credential on its own (`docs/patterns.md`, P07 to P11, P18
//! and P19).
use std::sync::LazyLock;

use super::{after, for_each_match, is_value_stop, name_start_ok, run_end, url};
use crate::detectors::{FixedPattern, Pattern, compile};
use crate::text::{case_sensitive, char_at, char_before};
use crate::unicode::{UnicodeVersion, Versioned, is_py_digit, space_class};

/// The spans of every credential `text` carries by its shape alone, name or no name.
pub(super) fn shape_spans(text: &str, version: UnicodeVersion) -> Vec<(usize, usize)> {
    let mut spans = provider_keys(text, version);
    spans.extend(pem_blocks(text));
    spans.extend(bearer_values(text, version));
    spans.extend(connection_passwords(text, version));
    spans.extend(url::url_password_spans(text, version));
    spans.extend(basic_values(text, version));
    spans
}

// ---- provider keys -------------------------------------------------------------------------

/// The literal head of every provider key shape: a candidate start, found cheaply, whose body
/// is then read in code after the lookbehind has been checked. (A pattern over whole keys would
/// scan a long run for each start the lookbehind then refuses.)
static KEY_HEAD: FixedPattern = LazyLock::new(|| {
    compile(
        r"sk-|[rs]k_live_|AKIA|ASIA|gh[pousr]_|github_pat_|xoxe\.xox[abepr]-|xox[abepr]-|AIza|hf_|glpat-|eyJ",
    )
});

/// `[A-Za-z0-9]`.
fn is_alnum(b: u8) -> bool {
    b.is_ascii_alphanumeric()
}

/// `[A-Za-z0-9_]`.
fn is_word_byte(b: u8) -> bool {
    is_alnum(b) || b == b'_'
}

/// `[A-Za-z0-9_-]`: also the characters a key may not follow.
fn is_key_byte(b: u8) -> bool {
    is_word_byte(b) || b == b'-'
}

/// `[A-Za-z0-9-]`.
fn is_dashed_byte(b: u8) -> bool {
    is_alnum(b) || b == b'-'
}

/// `[A-Z0-9]`.
fn is_upper_or_digit(b: u8) -> bool {
    b.is_ascii_uppercase() || b.is_ascii_digit()
}

/// Never: the class of a tail that no character is refused after.
fn is_never(_: u8) -> bool {
    false
}

/// The length of the run at the start of `text` of bytes that `ok` takes.
fn run_len(text: &str, ok: fn(u8) -> bool) -> usize {
    text.bytes().take_while(|&b| ok(b)).count()
}

/// A body of `ok` bytes, `least` of them or more: its length.
fn at_least(body: &str, ok: fn(u8) -> bool, least: usize) -> Option<usize> {
    Some(run_len(body, ok)).filter(|&len| len >= least)
}

/// A body of exactly `size` `ok` bytes, not followed by a byte `tail` takes: its length.
fn exactly(body: &str, ok: fn(u8) -> bool, size: usize, tail: fn(u8) -> bool) -> Option<usize> {
    let (key, rest) = body.split_at_checked(size)?;
    let closed = !rest.bytes().next().is_some_and(tail);
    (key.bytes().all(ok) && closed).then_some(size)
}

/// An `sk-` key's body: the run of key characters after `sk-` is split at its dashes, and some
/// segment of 20 or more word characters holds a digit, or is the last and a non-ASCII decimal
/// digit follows the run, or mixes upper and lower case (what a real key has and kebab-case
/// words do not). The key is the whole run.
fn sk_body(body: &str, version: UnicodeVersion) -> Option<usize> {
    let len = run_len(body, is_key_byte);
    let (run, rest) = body.split_at(len);
    let after_run = rest.chars().next();
    let accepted = run.rsplit('-').enumerate().any(|(from_end, segment)| {
        segment.len() >= 20
            && (segment.bytes().any(|b| b.is_ascii_digit())
                || (from_end == 0 && after_run.is_some_and(|c| is_py_digit(version, c)))
                || (segment.bytes().any(|b| b.is_ascii_uppercase())
                    && segment.bytes().any(|b| b.is_ascii_lowercase())))
    });
    accepted.then_some(len)
}

/// A JWT's body (after its `eyJ`): a run of ten or more key characters, `.eyJ`, a second run of
/// ten or more, a dot, and a run of any length, the signature.
fn jwt_body(body: &str) -> Option<usize> {
    let header = run_len(body, is_key_byte);
    let rest = after(body, header).strip_prefix(".eyJ")?;
    let payload = run_len(rest, is_key_byte);
    let signature = after(rest, payload).strip_prefix('.')?;
    (header >= 10 && payload >= 10)
        .then(|| header + ".eyJ".len() + payload + 1 + run_len(signature, is_key_byte))
}

/// The length of the body of the key that starts with `prefix`, or `None` when the text after
/// the prefix is not one.
fn body_len(prefix: &str, body: &str, version: UnicodeVersion) -> Option<usize> {
    match prefix {
        "sk-" => sk_body(body, version),
        "rk_live_" | "sk_live_" => at_least(body, is_alnum, 16),
        "AKIA" | "ASIA" => exactly(body, is_upper_or_digit, 16, is_alnum),
        "github_pat_" => at_least(body, is_word_byte, 22),
        "AIza" => exactly(body, is_key_byte, 35, is_key_byte),
        "hf_" => at_least(body, is_alnum, 30),
        "glpat-" => at_least(body, is_key_byte, 20),
        "eyJ" => jwt_body(body),
        _ if prefix.starts_with("gh") => exactly(body, is_alnum, 36, is_never),
        _ => at_least(body, is_dashed_byte, 10),
    }
}

/// Whether `c` is a character a key is made of: a key may not follow one.
fn is_key_char(c: char) -> bool {
    u8::try_from(c).is_ok_and(is_key_byte)
}

/// Provider keys by their documented prefix and shape (`OpenAI`, Anthropic, Stripe live keys, AWS
/// key ids, GitHub, Slack, Google, Hugging Face, GitLab) and JWTs: the whole match, and not
/// after a character a key is made of.
fn provider_keys(text: &str, version: UnicodeVersion) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    for_each_match(
        &KEY_HEAD,
        text,
        |head| !char_before(text, head.start()).is_some_and(is_key_char),
        |caps| {
            let head = caps.get_match();
            body_len(head.as_str(), after(text, head.end()), version).map_or(
                super::one_on(text, head.start()),
                |len| {
                    spans.push((head.start(), head.end() + len));
                    head.end() + len
                },
            )
        },
    );
    spans
}

// ---- PEM -----------------------------------------------------------------------------------

/// A private-key block, to its END line, or to the end of the text when a truncation cut it off,
/// so the key body never survives a cut (`docs/patterns.md`, P08: `\Z` is `\z`).
static PEM: FixedPattern = LazyLock::new(|| {
    compile(
        r"(?s)-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----.*?(?:-----END [A-Z0-9 ]*PRIVATE KEY-----|\z)",
    )
});

/// Spans of every PEM private-key block.
fn pem_blocks(text: &str) -> Vec<(usize, usize)> {
    PEM.find_iter(text).map(|m| (m.start(), m.end())).collect()
}

// ---- Bearer, Basic and a connection string's pwd ----------------------------------------

/// Whether the character before a match does not continue a word read case-insensitively: not an
/// ASCII letter or digit, and none of the characters Python's `(?i)` also reads as one (the
/// dotted and dotless `i`, the long `s`, the Kelvin sign).
fn not_after_word(text: &str, start: usize) -> bool {
    !char_before(text, start).is_some_and(|c| {
        c.is_ascii_alphanumeric() || matches!(c, '\u{130}' | '\u{131}' | '\u{17f}' | '\u{212a}')
    })
}

/// `Bearer` and a run of 19 or more token characters ending in a letter or digit (and `=` padding),
/// case-insensitively; the token is group 1. Python's `(?i)` also reads U+0130 and U+0131 as `i`,
/// which the crate does not, so the class spells them (`docs/patterns-semantics.md`, item 6).
/// The space class is Python's `\s`, which `(?i)` does not touch, so it stands outside the flag.
static BEARER: Pattern = Versioned::new(|version| {
    let space = case_sensitive(space_class(version));
    compile(&format!(
        r"(?i)bearer{space}+([a-z0-9._~+/\u{{130}}\u{{131}}-]{{19,}}[a-z0-9\u{{130}}\u{{131}}]=*)"
    ))
});

/// The token of every `Bearer <token>` of 20 or more characters, which is what keeps "the bearer
/// token" and "Bearer of bad news" out.
fn bearer_values(text: &str, version: UnicodeVersion) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    for_each_match(
        BEARER.get(version),
        text,
        |found| not_after_word(text, found.start()),
        |caps| {
            spans.extend(caps.get(1).map(|token| (token.start(), token.end())));
            caps.get_match().end()
        },
    );
    spans
}

/// `Basic`, space, base64 (group 1), then the end of the text or a character that does not
/// continue the base64 (`docs/patterns.md`, P11). The match takes the terminator; the search resumes where the
/// base64 ends.
/// The space class stands outside the `(?i)` flag, as in [`BEARER`].
static BASIC: Pattern = Versioned::new(|version| {
    let space = case_sensitive(space_class(version));
    compile(&format!(
        r"(?i)bas[i\u{{130}}\u{{131}}]c{space}+([A-Za-z0-9+/\u{{130}}\u{{131}}]{{4,}}={{0,2}})(?:\z|[^A-Za-z0-9+/=\u{{130}}\u{{131}}])"
    ))
});

/// The value of a base64 character, or `None` outside the alphabet.
fn sextet(b: u8) -> Option<u32> {
    match b {
        b'A'..=b'Z' => Some(u32::from(b - b'A')),
        b'a'..=b'z' => Some(u32::from(b - b'a') + 26),
        b'0'..=b'9' => Some(u32::from(b - b'0') + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// Whether `b64` decodes to printable ASCII text of a user, a colon and a password, as a `Basic` header carries:
/// strict base64 (nothing outside the alphabet, padded to a multiple of four), every byte a
/// printable ASCII character, and a colon among them.
fn is_user_pass(b64: &str) -> bool {
    let (mut pending, mut bits, mut colon) = (0, 0, false);
    for b in b64.trim_end_matches('=').bytes() {
        let Some(value) = sextet(b) else {
            return false;
        };
        pending = pending * 64 + value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            let unit = 1 << bits;
            let byte = pending / unit;
            pending %= unit;
            if !(0x20..=0x7e).contains(&byte) {
                return false;
            }
            colon |= byte == u32::from(b':');
        }
    }
    b64.len().is_multiple_of(4) && colon
}

/// The base64 of every `Basic <base64>` that decodes to printable text of a user, a colon and a password, which
/// keeps "Basic training" out. A match that does not is still consumed, as in the reference.
fn basic_values(text: &str, version: UnicodeVersion) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    for_each_match(
        BASIC.get(version),
        text,
        |found| not_after_word(text, found.start()),
        |caps| {
            // Group 1 is not optional, so every match has it; the match's own end stands in for
            // it, so a missing group could never stall the search.
            let value = caps.get(1);
            spans.extend(
                value
                    .filter(|v| is_user_pass(v.as_str()))
                    .map(|v| (v.start(), v.end())),
            );
            value.map_or(caps.get_match().end(), |v| v.end())
        },
    );
    spans
}

/// The name `pwd` and its `=`, case-insensitively: a connection string's password.
static PWD_HEAD: FixedPattern = LazyLock::new(|| compile(r"(?i:pwd)[=]"));

/// Whether the text at `at` reads as a path or a variable and so is no password, since `PWD` is
/// also the shell's working directory. It starts with one of `/ ~ . $ %`, a backslash or a backtick, or
/// is a drive letter, a colon and a slash.
fn is_working_directory(text: &str, at: usize) -> bool {
    char_at(text, at).is_some_and(|c| {
        "/~.$%\\`".contains(c)
            || (c.is_ascii_alphabetic()
                && char_at(text, at + 1) == Some(':')
                && matches!(char_at(text, at + 2), Some('\\' | '/')))
    })
}

/// The value of every `pwd` written with no space around the `=`: the value only, after an
/// optional quote. The match is not after `$` and starts a name (`docs/patterns.md`, P10).
fn connection_passwords(text: &str, version: UnicodeVersion) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    for_each_match(
        &PWD_HEAD,
        text,
        |head| char_before(text, head.start()) != Some('$') && name_start_ok(text, head.start()),
        |caps| {
            let head = caps.get_match();
            let quoted = char_at(text, head.end()).is_some_and(|c| c == '"' || c == '\'');
            let value = head.end() + usize::from(quoted);
            let end = run_end(text, value, |c| is_value_stop(c, version));
            if end > value && !is_working_directory(text, value) {
                spans.push((value, end));
                end
            } else {
                super::one_on(text, head.start())
            }
        },
    );
    spans
}

// ---- values that a name vouches for ---------------------------------------------------------

/// Whether `value` holds an `sk-` key under a name that vouches for it: any run of 20 or more
/// word characters between the dashes of the key, digits or not.
pub(super) fn has_vouched_key(value: &str) -> bool {
    value.match_indices("sk-").any(|(at, head)| {
        !char_before(value, at).is_some_and(is_key_char)
            && after(value, at + head.len())
                .split(|c| !u8::try_from(c).is_ok_and(is_key_byte))
                .next()
                .is_some_and(|run| run.split('-').any(|segment| segment.len() >= 20))
    })
}

/// Whether `value` is, whole, a run of 16 or more characters of `A-Za-z0-9._~+/=-` that holds a
/// letter and a digit: a character-class proxy for "high entropy", not a measured entropy.
pub(super) fn is_high_entropy(value: &str) -> bool {
    value.len() >= 16
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._~+/=-".contains(&b))
        && value.bytes().any(|b| b.is_ascii_alphabetic())
        && value.bytes().any(|b| b.is_ascii_digit())
}
