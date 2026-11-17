//! The password of a URL with userinfo: the text between the colon after the user and the `@`
//! that opens the host.
//!
//! The head (`://`, the user, the colon) is a pattern; the password is walked in code, because
//! a password written as a repeated group makes a regex engine keep a stack entry per character
//! (hundreds of megabytes for a megabyte token). The walk takes the same characters in runs, in
//! memory that does not grow with the token.
use super::{after, for_each_match, run_end};
use crate::detectors::{Pattern, compile};
use crate::text::{char_at, char_before};
use crate::unicode::{
    UnicodeVersion, Versioned, is_py_digit, is_py_space, is_py_word, space_class,
};

/// `://`, a run of anything but whitespace and the URL's delimiters, and the colon that ends the
/// user (`docs/patterns.md`, P05). The run cannot hold a colon, so the first colon after `://`
/// ends the match. The two guards (not after whitespace; not a port followed by a separator)
/// are checks in code.
static URL_HEAD: Pattern = Versioned::new(|version| {
    let space = space_class(version);
    compile(&format!(r#"://[^{space}/?#@:"'`<>]*:"#))
});

/// What the password's walk reads past without stopping: a quote, a backtick or an angle bracket
/// is part of a password only with a letter, a digit, an underscore or `%` after it.
const QUOTE_CHARS: &str = "\"'`<>";

/// A password run: anything but whitespace and the characters that end or quote one.
fn is_plain_stop(c: char, version: UnicodeVersion) -> bool {
    is_py_space(version, c) || "/?#@\"'`<>".contains(c)
}

/// `[\w<extra>]`: a letter, a digit, an underscore or `extra`.
fn is_word_or(c: char, extra: char, version: UnicodeVersion) -> bool {
    is_py_word(version, c) || c == extra
}

/// Whether a quote at `position` stays inside the password: a letter, digit, underscore or `%`
/// follows it.
fn quote_belongs(text: &str, position: usize, version: UnicodeVersion) -> bool {
    char_at(text, position).is_some_and(|c| QUOTE_CHARS.contains(c))
        && char_at(text, position + 1).is_some_and(|next| is_word_or(next, '%', version))
}

/// Whether the `@` at `at` may be inside the password: a word character or `[` follows it. The
/// walk in `password_end` then decides whether another `@` is reached.
fn at_opens_the_host(text: &str, at: usize, version: UnicodeVersion) -> bool {
    char_at(text, at + 1).is_some_and(|c| is_word_or(c, '[', version))
}

/// The `@` that ends the password that begins at `start`, if there is one: the last `@` the walk
/// reaches that is not followed by whitespace, after at least one character.
fn password_end(text: &str, start: usize, version: UnicodeVersion) -> Option<usize> {
    let mut position = start;
    let mut end = None;
    // Each pass moves one character on, so the text's length bounds the walk.
    for _ in 0..=text.len() {
        position = run_end(text, position, |c| is_plain_stop(c, version));
        let goes_on = if char_at(text, position) == Some('@') {
            if position > start
                && char_at(text, position + 1).is_none_or(|c| !is_py_space(version, c))
            {
                end = Some(position);
            }
            at_opens_the_host(text, position, version)
        } else {
            quote_belongs(text, position, version)
        };
        if !goes_on {
            break;
        }
        position += 1;
    }
    end
}

/// Whether the text after the colon at `end` does not read as a port followed by a separator
/// (`:8080,` is a port, some text and maybe an address, not a password): one to five digits and
/// then one of `,;|&>` is exactly that.
fn is_not_a_port(text: &str, end: usize, version: UnicodeVersion) -> bool {
    let rest = after(text, end);
    let digits = rest
        .chars()
        .take_while(|&c| is_py_digit(version, c))
        .take(6)
        .count();
    let separated = rest
        .chars()
        .nth(digits)
        .is_some_and(|c| ",;|&>".contains(c));
    !((1..=5).contains(&digits) && separated)
}

/// Spans of the passwords of every URL with userinfo in `text`.
pub(super) fn url_password_spans(text: &str, version: UnicodeVersion) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    for_each_match(
        URL_HEAD.get(version),
        text,
        |head| {
            !char_before(text, head.start()).is_some_and(|c| is_py_space(version, c))
                && is_not_a_port(text, head.end(), version)
        },
        |caps| {
            let head = caps.get_match();
            password_end(text, head.end(), version).map_or(
                super::one_on(text, head.start()),
                |end| {
                    spans.push((head.end(), end));
                    // The password ends at an `@`, which no URL head can start with.
                    end
                },
            )
        },
    );
    spans
}
