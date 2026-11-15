//! Data-hygiene primitives: PII redaction and near-duplicate detection.
#![forbid(unsafe_code)]

mod cluster;
mod detectors;
mod minhash;
mod table;
mod text;

pub use cluster::{BANDS, Pair, SHINGLE_SIZE, near_duplicates};
pub use detectors::{
    DETECTORS, Detector, find_ca_sins, find_dates_of_birth, find_eu_vat, find_ibans,
    find_in_aadhaar, find_in_pans, find_ip_addresses, find_national_ids, find_payment_cards,
    find_uk_ninos, iban_mod97_ok, is_ip_address, luhn_ok, verhoeff_ok,
};
pub use minhash::{NUM_PERMUTATIONS, minhash_signature};
pub use text::{
    DIGIT_CLASS, SPACE_CLASS, WORD_CLASS, case_sensitive, char_at, char_before, is_py_digit,
    is_py_space, is_py_word, to_char_offsets,
};

/// The binding contract version, bumped whenever a binding changes meaning.
///
/// This is the one instrumented function the empty crate carries so that the 100% coverage
/// stage of the gate measures something; the Python module exports it as `CORE_API`.
#[must_use]
pub const fn core_api() -> u32 {
    1
}
