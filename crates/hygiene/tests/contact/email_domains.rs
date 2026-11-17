//! `find_emails`: where the domain of an address ends.
use crate::reference::find_emails;

use super::assert_reads;

/// The addresses found in `text`, as the strings they span.
fn read(text: &str) -> Vec<&str> {
    find_emails(text)
        .into_iter()
        .map(|(start, end)| {
            text.get(start..end)
                .expect("a span is on character boundaries")
        })
        .collect()
}

#[test]
fn a_domain_ends_at_a_double_dot_or_a_trailing_dot() {
    assert_eq!(
        read("a@b.c..d a@b.c. a@.b.c a@b a@b-.c"),
        ["a@b.c", "a@b.c", "a@b-.c"]
    );
}

#[test]
fn a_domain_ends_at_the_first_character_that_is_not_a_label_character() {
    for (index, (text, found)) in [
        ("a@b.c/x", "a@b.c"),
        ("a@b.c:80", "a@b.c"),
        ("a@b.c?x", "a@b.c"),
        ("a@b.c+d", "a@b.c"),
        ("a@b.c@d.e", "a@b.c"),
        ("a@b.c\u{2003}d", "a@b.c"),
    ]
    .into_iter()
    .enumerate()
    {
        assert_reads(find_emails, &format!("stop {index}"), text, &[found]);
    }
}

#[test]
fn hyphens_and_underscores_belong_to_labels_wherever_they_are() {
    for (index, text) in ["a@b-.c", "a@-b.c", "a@b.c-", "a@b.c_", "a@b_c.d-e"]
        .into_iter()
        .enumerate()
    {
        assert_reads(find_emails, &format!("label {index}"), text, &[text]);
    }
}

#[test]
fn trailing_dots_are_not_part_of_the_domain() {
    assert_eq!(read("a@b.c."), ["a@b.c"]);
    assert_eq!(read("a@b.c...d"), ["a@b.c"]);
    assert_eq!(read("a@b.c.. "), ["a@b.c"]);
}

#[test]
fn a_domain_needs_a_dot_after_its_first_character() {
    assert_eq!(read("a@b."), Vec::<&str>::new());
    assert_eq!(read("a@.b"), Vec::<&str>::new());
    assert_eq!(read("a@b.c"), ["a@b.c"]);
    assert_eq!(read("a@bc.d"), ["a@bc.d"]);
}
