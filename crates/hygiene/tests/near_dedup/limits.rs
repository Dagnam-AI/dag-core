//! The size limits at their real values: each is read one past the limit, before anything of the
//! batch is hashed, and in the reference's order. The messages are the reference's.
use dagnam_hygiene::{MAX_TEXT_CHARS, MAX_TEXTS, MAX_TOTAL_CHARS, NearDedupError, near_duplicates};

/// `count` texts of one distinct word each: no two share a shingle, so no pair is found.
fn distinct(count: usize) -> Vec<String> {
    (0..count).map(|i| format!("w{i}")).collect()
}

fn run(texts: &[String]) -> Result<(), NearDedupError> {
    let borrowed: Vec<&str> = texts.iter().map(String::as_str).collect();
    near_duplicates(&borrowed, 0.9).map(|_| ())
}

#[test]
fn the_most_texts_are_taken_and_one_more_is_refused() {
    assert_eq!(run(&distinct(MAX_TEXTS)), Ok(()));
    let error = run(&distinct(MAX_TEXTS + 1)).unwrap_err();
    assert_eq!(error, NearDedupError::TooManyTexts { count: 50_001 });
    assert_eq!(
        error.to_string(),
        "50001 texts is more than MAX_TEXTS (50000)"
    );
}

#[test]
fn a_text_of_the_most_characters_is_taken_and_one_more_is_refused() {
    assert_eq!(run(&["a".repeat(MAX_TEXT_CHARS)]), Ok(()));
    let error = run(&["a".repeat(MAX_TEXT_CHARS + 1)]).unwrap_err();
    assert_eq!(error, NearDedupError::TextTooLong { chars: 1_000_001 });
    assert_eq!(
        error.to_string(),
        "a text of 1000001 characters is more than MAX_TEXT_CHARS (1000000)"
    );
}

#[test]
fn a_text_is_measured_in_characters_not_bytes() {
    // Two bytes each: over the limit in bytes, at it in characters.
    assert_eq!(run(&["\u{e9}".repeat(MAX_TEXT_CHARS)]), Ok(()));
    assert_eq!(
        run(&["\u{1f600}".repeat(MAX_TEXT_CHARS + 1)]),
        Err(NearDedupError::TextTooLong {
            chars: MAX_TEXT_CHARS + 1
        })
    );
}

#[test]
fn the_most_characters_in_all_are_taken_and_one_more_is_refused() {
    let whole = MAX_TOTAL_CHARS / MAX_TEXT_CHARS;
    // Each text starts with its own letter so that no two are the same shingle.
    let mut texts: Vec<String> = (0..whole)
        .map(|i| {
            format!(
                "{}{}",
                char::from(b'a' + u8::try_from(i).unwrap()),
                "x".repeat(MAX_TEXT_CHARS - 1)
            )
        })
        .collect();
    assert_eq!(run(&texts), Ok(()));
    texts.push("z".to_owned());
    let error = run(&texts).unwrap_err();
    assert_eq!(error, NearDedupError::TooManyChars);
    assert_eq!(
        error.to_string(),
        "the texts hold more than MAX_TOTAL_CHARS (20000000) characters"
    );
}

#[test]
fn the_threshold_is_read_before_any_size() {
    let texts = distinct(MAX_TEXTS + 1);
    let borrowed: Vec<&str> = texts.iter().map(String::as_str).collect();
    assert_eq!(
        near_duplicates(&borrowed, 0.0),
        Err(NearDedupError::Threshold)
    );
}

#[test]
fn the_count_is_read_before_the_length_of_any_text() {
    let mut texts = distinct(MAX_TEXTS + 1);
    texts[0] = "a".repeat(MAX_TEXT_CHARS + 1);
    assert_eq!(
        run(&texts),
        Err(NearDedupError::TooManyTexts {
            count: MAX_TEXTS + 1
        })
    );
}

#[test]
fn a_long_text_is_read_before_the_total_it_would_pass() {
    let texts = ["a".repeat(MAX_TEXT_CHARS), "b".repeat(MAX_TEXT_CHARS + 1)];
    assert_eq!(
        run(&texts),
        Err(NearDedupError::TextTooLong {
            chars: MAX_TEXT_CHARS + 1
        })
    );
}
