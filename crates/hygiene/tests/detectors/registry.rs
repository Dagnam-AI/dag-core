//! The registry: its order, its flags, and that each entry finds its own class.
use dagnam_hygiene::DETECTORS;

use super::fill;
use crate::{CredentialCase, cases};

/// What each class finds in a text made for it, as `(code, template, value)`.
const SAMPLES: [(&str, &str, &str); 13] = [
    ("PII_EMAIL", "mail {}.", "jane@example.com"),
    ("PII_PHONE", "call {}.", "+1 415-555-0132"),
    ("PII_PAYMENT_CARD", "card {}.", "4111 1111 1111 1111"),
    ("PII_NATIONAL_ID", "ssn {}.", "078-05-1120"),
    // The credential's sample comes from the corpus, in `samples`: no file holds one.
    ("PII_SECRET", "see {}.", ""),
    ("PII_IBAN", "iban {}.", "NL91ABNA0417164300"),
    ("PII_IP_ADDRESS", "from {}.", "203.0.113.9"),
    ("PII_DATE_OF_BIRTH", "DOB: {}", "14/03/1987"),
    ("PII_UK_NINO", "NI {}.", "AB123456C"),
    ("PII_CA_SIN", "SIN {}.", "130 692 544"),
    ("PII_IN_AADHAAR", "uid {}.", "2345 6789 0124"),
    ("PII_IN_PAN", "PAN {}.", "ABCPE1234F"),
    ("PII_EU_VAT", "VAT {}.", "DE123456789"),
];

/// The credential's sample: the text of the first corpus case that is one credential from end to
/// end and that no class but `PII_SECRET` reads anything in once it stands in its template (the
/// repository refuses secret-shaped text, so no file holds one: it comes from the corpus).
fn secret_value() -> String {
    let template = SAMPLES.iter().find(|s| s.0 == "PII_SECRET").unwrap().1;
    cases::<CredentialCase>("credentials")
        .iter()
        .filter(|case| {
            let width = case.assembled().chars().count();
            case.spans == [(0, width, "PII_SECRET".to_owned())] && width <= 100
        })
        .map(CredentialCase::assembled)
        .find(|value| {
            let (text, span) = fill(template, value);
            DETECTORS.iter().all(|d| {
                let found = (d.find)(&text);
                if d.code == "PII_SECRET" {
                    found == [span]
                } else {
                    found.is_empty()
                }
            })
        })
        .unwrap()
}

/// The samples, with the credential's value taken from the corpus.
fn samples() -> Vec<(&'static str, String, (usize, usize))> {
    let secret = secret_value();
    SAMPLES
        .iter()
        .map(|(code, template, value)| {
            let value = if *code == "PII_SECRET" {
                secret.as_str()
            } else {
                value
            };
            let (text, span) = fill(template, value);
            (*code, text, span)
        })
        .collect()
}

#[test]
fn the_registry_holds_these_classes_in_the_references_order() {
    let codes: Vec<&str> = DETECTORS.iter().map(|d| d.code).collect();
    // The reference's order.
    assert_eq!(
        codes,
        [
            "PII_EMAIL",
            "PII_PHONE",
            "PII_PAYMENT_CARD",
            "PII_NATIONAL_ID",
            "PII_SECRET",
            "PII_IBAN",
            "PII_IP_ADDRESS",
            "PII_DATE_OF_BIRTH",
            "PII_UK_NINO",
            "PII_CA_SIN",
            "PII_IN_AADHAAR",
            "PII_IN_PAN",
            "PII_EU_VAT",
        ]
    );
}

#[test]
fn only_the_phone_class_is_loose() {
    let loose: Vec<&str> = DETECTORS
        .iter()
        .filter(|d| d.loose)
        .map(|d| d.code)
        .collect();
    assert_eq!(loose, ["PII_PHONE"]);
}

#[test]
fn each_entry_finds_exactly_the_sample_of_its_own_class_and_no_other() {
    let samples = samples();
    for (detector, (code, text, span)) in DETECTORS.iter().zip(&samples) {
        assert_eq!(detector.code, *code);
        assert_eq!((detector.find)(text), vec![*span], "{code}");
        // A loose class reads other classes' samples as digits (an Aadhaar number is a phone
        // number to the phone pattern): a stricter class names that text.
        for (other, text, _) in samples
            .iter()
            .filter(|sample| sample.0 != *code && !detector.loose)
        {
            assert!((detector.find)(text).is_empty(), "{code} on {other}");
        }
    }
}

#[test]
fn the_loose_phone_class_also_reads_the_digits_of_a_stricter_class() {
    let (text, span) = fill("uid {}.", "2345 6789 0124");
    let phone = DETECTORS.iter().find(|d| d.code == "PII_PHONE").unwrap();
    assert_eq!((phone.find)(&text), vec![span]);
}
