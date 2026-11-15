//! Data-hygiene primitives: PII redaction and near-duplicate detection.
#![forbid(unsafe_code)]

mod cluster;
mod minhash;

pub use cluster::{BANDS, Pair, SHINGLE_SIZE, near_duplicates};
pub use minhash::{NUM_PERMUTATIONS, minhash_signature};

/// The binding contract version, bumped whenever a binding changes meaning.
///
/// This is the one instrumented function the empty crate carries so that the 100% coverage
/// stage of the gate measures something; the Python module exports it as `CORE_API`.
#[must_use]
pub const fn core_api() -> u32 {
    1
}
