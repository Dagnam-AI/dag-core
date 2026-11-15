//! The registry: its order, its flags, and that each entry finds its own class.
use dagnam_hygiene::DETECTORS;

use super::fill;

/// What each class finds in a text made for it, as `(code, template, value)`.
const SAMPLES: [(&str, &str, &str); 10] = [
    ("PII_PAYMENT_CARD", "card {}.", "4111 1111 1111 1111"),
    ("PII_NATIONAL_ID", "ssn {}.", "078-05-1120"),
    ("PII_IBAN", "iban {}.", "NL91ABNA0417164300"),
    ("PII_IP_ADDRESS", "from {}.", "203.0.113.9"),
    ("PII_DATE_OF_BIRTH", "DOB: {}", "14/03/1987"),
    ("PII_UK_NINO", "NI {}.", "AB123456C"),
    ("PII_CA_SIN", "SIN {}.", "130 692 544"),
    ("PII_IN_AADHAAR", "uid {}.", "2345 6789 0124"),
    ("PII_IN_PAN", "PAN {}.", "ABCPE1234F"),
    ("PII_EU_VAT", "VAT {}.", "DE123456789"),
];

#[test]
fn the_registry_holds_these_classes_in_the_references_relative_order() {
    let codes: Vec<&str> = DETECTORS.iter().map(|d| d.code).collect();
    // The reference's order, less the contact and credential classes that are added with
    // their finders.
    assert_eq!(
        codes,
        [
            "PII_PAYMENT_CARD",
            "PII_NATIONAL_ID",
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
fn none_of_these_classes_is_loose() {
    assert!(DETECTORS.iter().all(|d| !d.loose));
}

#[test]
fn each_entry_finds_exactly_the_sample_of_its_own_class_and_no_other() {
    let samples: Vec<(&str, String, (usize, usize))> = SAMPLES
        .iter()
        .map(|(code, template, value)| {
            let (text, span) = fill(template, value);
            (*code, text, span)
        })
        .collect();
    for (detector, (code, text, span)) in DETECTORS.iter().zip(&samples) {
        assert_eq!(detector.code, *code);
        assert_eq!((detector.find)(text), vec![*span], "{code}");
        for (other, text, _) in samples.iter().filter(|sample| sample.0 != *code) {
            assert!((detector.find)(text).is_empty(), "{code} on {other}");
        }
    }
}
