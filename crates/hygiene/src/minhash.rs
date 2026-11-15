//! `MinHash` signatures, bit-identical to the Python reference.
use std::sync::LazyLock;

use blake2::{Blake2b, Digest, digest::consts::U8};

/// Signature length.
pub const NUM_PERMUTATIONS: usize = 128;
/// The Mersenne prime 2^61 - 1 every permutation works modulo.
const MERSENNE: u128 = (1 << 61) - 1;

/// The first eight bytes of the `BLAKE2b` digest of `text`, big-endian.
fn hash64(text: &str) -> u64 {
    u64::from_be_bytes(Blake2b::<U8>::digest(text.as_bytes()).into())
}

/// The fixed `(a, b)` pairs: `blake2b("a{i}") mod p` (1 when that is 0) and `blake2b("b{i}")`
/// unreduced, because the reduction of `a * h + b` absorbs it.
static COEFFICIENTS: LazyLock<[(u128, u128); NUM_PERMUTATIONS]> = LazyLock::new(|| {
    std::array::from_fn(|i| {
        let a = u128::from(hash64(&format!("a{i}"))) % MERSENNE;
        (a.max(1), u128::from(hash64(&format!("b{i}"))))
    })
});

/// One permutation of a shingle hash: `(a * h + b) mod p`.
///
/// `a < 2^61` and `h < 2^64`, so the sum fits `u128`.
#[allow(clippy::cast_possible_truncation)] // the value is below p < 2^61 by the modulo
fn permute(a: u128, b: u128, h: u128) -> u64 {
    ((a * h + b) % MERSENNE) as u64
}

/// `MinHash` signature of the shingles; every value is `u64::MAX` when there are none.
///
/// A repeated shingle changes no minimum, so the set the reference builds is not rebuilt.
pub fn minhash_signature<'a>(
    shingles: impl IntoIterator<Item = &'a str>,
) -> [u64; NUM_PERMUTATIONS] {
    let hashed: Vec<u128> = shingles
        .into_iter()
        .map(|shingle| u128::from(hash64(shingle)))
        .collect();
    let mut signature = [u64::MAX; NUM_PERMUTATIONS];
    for (slot, &(a, b)) in signature.iter_mut().zip(COEFFICIENTS.iter()) {
        // No shingles: no minimum, and the signature stays all `u64::MAX`.
        *slot = hashed
            .iter()
            .map(|&h| permute(a, b, h))
            .min()
            .unwrap_or(u64::MAX);
    }
    signature
}
