//! Data-hygiene primitives: PII redaction and near-duplicate detection.
#![forbid(unsafe_code)]

mod candidates;
mod cluster;
mod contact;
mod credentials;
mod detectors;
mod limits;
mod minhash;
mod spans;
mod table;
mod text;
mod unicode;

pub use candidates::BANDS;
pub use cluster::{Batch, Pair, SHINGLE_SIZE, near_duplicates, near_duplicates_within};
pub use contact::{find_emails, find_phones};
pub use credentials::find_secrets;
pub use detectors::{
    DETECTORS, Detector, find_ca_sins, find_dates_of_birth, find_eu_vat, find_ibans,
    find_in_aadhaar, find_in_pans, find_ip_addresses, find_national_ids, find_payment_cards,
    find_uk_ninos, iban_mod97_ok, is_ip_address, luhn_ok, verhoeff_ok,
};
pub use limits::{
    Limits, MAX_COMPARISONS, MAX_PAIRS, MAX_TEXT_CHARS, MAX_TEXTS, MAX_TOTAL_CHARS, NearDedupError,
    check_text_count, check_text_lengths,
};
pub use minhash::{NUM_PERMUTATIONS, minhash_signature};
pub use spans::findings;
pub use text::{case_sensitive, char_at, char_before, to_char_offsets};
pub use unicode::{
    UnicodeVersion, digit_class, is_py_digit, is_py_space, is_py_word, space_class, word_class,
};

/// The binding contract version, bumped whenever a binding changes meaning.
///
/// This is the one instrumented function the empty crate carries so that the 100% coverage
/// stage of the gate measures something; the Python module exports it as `CORE_API`.
#[must_use]
pub const fn core_api() -> u32 {
    2
}
