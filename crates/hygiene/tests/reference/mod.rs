//! The finders under the Unicode of the reference interpreter (Python 3.13, Unicode 15.1.0), the
//! one every corpus case was exported under. The tests of one finder read through these so each
//! keeps its plain `fn(&str)` shape; the tests that vary the version call the crate directly.
use dagnam_hygiene::UnicodeVersion;

/// The version of the reference interpreter.
pub(crate) const VERSION: UnicodeVersion = UnicodeVersion::V15_1;

macro_rules! spans_under_reference {
    ($($name:ident),+ $(,)?) => {
        $(
            pub(crate) fn $name(text: &str) -> Vec<(usize, usize)> {
                dagnam_hygiene::$name(text, VERSION)
            }
        )+
    };
}

spans_under_reference!(
    find_ca_sins,
    find_dates_of_birth,
    find_emails,
    find_eu_vat,
    find_ibans,
    find_in_aadhaar,
    find_in_pans,
    find_ip_addresses,
    find_national_ids,
    find_payment_cards,
    find_phones,
    find_secrets,
    find_uk_ninos,
);

pub(crate) fn findings(text: &str) -> Vec<(usize, usize, &'static str)> {
    dagnam_hygiene::findings(text, VERSION)
}

pub(crate) fn luhn_ok(digits: &str) -> bool {
    dagnam_hygiene::luhn_ok(digits, VERSION)
}

pub(crate) fn verhoeff_ok(digits: &str) -> bool {
    dagnam_hygiene::verhoeff_ok(digits, VERSION)
}

pub(crate) fn iban_mod97_ok(iban: &str) -> bool {
    dagnam_hygiene::iban_mod97_ok(iban, VERSION)
}

pub(crate) fn is_ip_address(candidate: &str, before: &str) -> bool {
    dagnam_hygiene::is_ip_address(candidate, before, VERSION)
}
