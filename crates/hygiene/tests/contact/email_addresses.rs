//! `find_emails`: the head of an address (its local part and `@`) and what may stand before it.
use dagnam_hygiene::find_emails;

use super::{assert_none, assert_reads};

#[test]
fn addresses_are_found_exactly() {
    for (index, (text, needle)) in [
        ("mail jane@example.com now", "jane@example.com"),
        ("a.b+c@mail.example.co.uk", "a.b+c@mail.example.co.uk"),
        ("x@y.co", "x@y.co"),
        ("1@2.3", "1@2.3"),
        ("A@B.C", "A@B.C"),
        ("a@b.c.d.e.f", "a@b.c.d.e.f"),
        ("(a@b.c)", "a@b.c"),
        ("<a@b.c>", "a@b.c"),
        ("a b@c.d", "b@c.d"),
        ("pw@ftp.example.com", "pw@ftp.example.com"),
    ]
    .into_iter()
    .enumerate()
    {
        assert_reads(find_emails, &format!("positive {index}"), text, &[needle]);
    }
}

#[test]
fn the_local_part_may_start_with_any_of_its_characters() {
    for (index, text) in [
        ".a@b.c",
        "-a@b.c",
        "+a@b.c",
        "_a@b.c",
        "a.@b.c",
        "ab..c@d.e",
    ]
    .into_iter()
    .enumerate()
    {
        assert_reads(find_emails, &format!("local {index}"), text, &[text]);
    }
}

#[test]
fn near_misses_are_not_addresses() {
    for (index, text) in [
        "a@b",
        "a@b.",
        "a@.b.c",
        "a@..",
        "a@.",
        "a@b..c",
        "\"a\"@b.c",
        "a@@b.c",
        "@b.c",
        "a@b c.d",
        "a\u{345}@b.c",
        "a@b.\u{345}c",
        "no address here",
        "",
    ]
    .into_iter()
    .enumerate()
    {
        assert_none(find_emails, &format!("near miss {index}"), text);
    }
}

#[test]
fn a_run_of_local_characters_is_one_head() {
    assert_reads(find_emails, "after space", "x  a@b.c", &["a@b.c"]);
    assert_reads(
        find_emails,
        "after comma",
        "a@b.c, d@e.f;g@h.i",
        &["a@b.c", "d@e.f", "g@h.i"],
    );
    // A head cannot start inside a word: the first `@` ends the first head and what follows
    // it is a domain, not a local part.
    assert_reads(find_emails, "two ats", "a@b.c@d.e", &["a@b.c"]);
}

#[test]
fn a_domain_that_fails_leaves_the_text_after_the_at_for_the_next_head() {
    // `b.` is no domain, and `b.@c.d` is a head whose domain is `c.d`.
    assert_eq!(find_emails("a@b.@c.d"), vec![(2, 8)]);
    assert_eq!(find_emails("a@a@b.co"), vec![(2, 8)]);
}

#[test]
fn a_head_that_stands_in_the_middle_of_a_run_is_refused_and_the_text_after_it_is_read() {
    // The domain ends at the double dot, so the rest of the run is searched from its middle:
    // every position inside it follows a head character and is refused, up to the `@`.
    assert_reads(find_emails, "double dot", "a@b.c..x@y.z", &["a@b.c"]);
    assert_reads(
        find_emails,
        "double dot, then a head",
        "a@b.c.. d@e.f",
        &["a@b.c", "d@e.f"],
    );
    assert_reads(
        find_emails,
        "long run",
        &format!("a@b.c..{}@d.e", "x".repeat(40)),
        &["a@b.c"],
    );
    assert_reads(
        find_emails,
        "after the at",
        "a@b.c..x@y.z q@r.st",
        &["a@b.c", "q@r.st"],
    );
}

#[test]
fn word_characters_are_the_references_not_the_crates() {
    // A superscript digit is a word character; a combining mark is not.
    assert_reads(find_emails, "superscript", "\u{b2}@b.c", &["\u{b2}@b.c"]);
    assert_reads(
        find_emails,
        "accented",
        "\u{e9}@\u{fc}.co",
        &["\u{e9}@\u{fc}.co"],
    );
    assert_none(find_emails, "mark in local", "a\u{345}@b.c");
    assert_reads(
        find_emails,
        "mark ends the domain",
        "a@b.c\u{345}",
        &["a@b.c"],
    );
    assert_reads(
        find_emails,
        "IDN",
        "a@\u{fc}.\u{65e5}\u{672c}",
        &["a@\u{fc}.\u{65e5}\u{672c}"],
    );
}

#[test]
fn a_character_the_pinned_unicode_version_made_a_letter_ends_in_the_domain() {
    // U+2EBF0 is a letter since Unicode 15.1 and U+1C89 since 16.0: the table is the pinned one.
    let text = "a@b.\u{2ebf0}";
    assert_eq!(find_emails(text), vec![(0, text.len())]);
    assert_none(find_emails, "U+1C89", "a@b.\u{1c89}");
}
