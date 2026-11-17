//! The finders read a text under the Unicode version they are given: the code points on which
//! the interpreters differ change what is found, and nothing else does.
use dagnam_hygiene::UnicodeVersion::{self, V15_0, V15_1, V16_0};
use dagnam_hygiene::{find_emails, find_payment_cards, find_phones, findings};

/// The address `a@b.` and a character that is a word character only from some version on: the
/// domain's last label is that character, so the address exists only where it is one.
fn address_ending_in(last: char) -> String {
    format!("a@b.{last}")
}

fn email_spans(text: &str, version: UnicodeVersion) -> Vec<(usize, usize)> {
    find_emails(text, version)
}

#[test]
fn an_address_ending_in_cjk_extension_i_exists_from_unicode_15_1() {
    // Byte spans; `findings` reports code points. The divergence the core used to document: Python 3.12 has no address, 3.13 and 3.14 one.
    let text = address_ending_in('\u{2ebf0}');
    assert_eq!(email_spans(&text, V15_0), []);
    assert_eq!(email_spans(&text, V15_1), [(0, text.len())]);
    assert_eq!(email_spans(&text, V16_0), [(0, text.len())]);
    assert_eq!(findings(&text, V15_0), []);
    assert_eq!(findings(&text, V15_1), [(0, 5, "PII_EMAIL")]);
    assert_eq!(findings(&text, V16_0), [(0, 5, "PII_EMAIL")]);
}

#[test]
fn an_address_ending_in_a_unicode_16_letter_exists_only_from_unicode_16() {
    // U+1C89, a Cyrillic letter new in Unicode 16.0: Python 3.14 reads the address, 3.12 and 3.13 not.
    let text = address_ending_in('\u{1c89}');
    assert_eq!(email_spans(&text, V15_0), []);
    assert_eq!(email_spans(&text, V15_1), []);
    assert_eq!(email_spans(&text, V16_0), [(0, text.len())]);
    assert_eq!(findings(&text, V16_0), [(0, 5, "PII_EMAIL")]);
}

#[test]
fn a_number_in_a_unicode_16_script_is_read_only_under_unicode_16() {
    // Garay digits (U+10D40 to U+10D49) are `Nd` from Unicode 16.0: a card number and a phone
    // number written in them are numbers to Python 3.14 and runs of letters-less symbols before.
    let garay = |ascii: &str| -> String {
        ascii
            .chars()
            .map(|c| match c.to_digit(10) {
                Some(value) => char::from_u32(0x10d40 + value).unwrap(),
                None => c,
            })
            .collect()
    };
    let card = garay("4111111111111111");
    assert_eq!(find_payment_cards(&card, V15_0), []);
    assert_eq!(find_payment_cards(&card, V15_1), []);
    assert_eq!(find_payment_cards(&card, V16_0), [(0, card.len())]);
    let phone = garay("415-555-0132");
    assert_eq!(find_phones(&phone, V15_1), []);
    assert_eq!(find_phones(&phone, V16_0), [(0, phone.len())]);
}

#[test]
fn text_on_which_the_versions_agree_is_read_the_same_under_each() {
    let text = "mail jane@example.com or call +44 20 7946 0958, card 4111111111111111";
    let under = |version| findings(text, version);
    assert_eq!(under(V15_0), under(V15_1));
    assert_eq!(under(V15_1), under(V16_0));
    assert_eq!(under(V16_0).len(), 3);
}
