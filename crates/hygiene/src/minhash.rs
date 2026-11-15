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

/// The fixed `(a, b)` pairs: `blake2b("a{i}") mod p` (1 when that is 0) and `blake2b("b{i}")` (reduced with the sum).
static COEFFICIENTS: LazyLock<Vec<(u128, u128)>> = LazyLock::new(|| {
    (0..NUM_PERMUTATIONS)
        .map(|i| {
            let a = u128::from(hash64(&format!("a{i}"))) % MERSENNE;
            // `b` needs no reduction of its own: the reduction of `a * h + b` absorbs it.
            let b = u128::from(hash64(&format!("b{i}")));
            (a.max(1), b)
        })
        .collect()
});

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
    // `a < 2^61` and `h < 2^64`, so `a * h + b` fits `u128`; the minimum is below `p < 2^64`.
    for (slot, &(a, b)) in signature.iter_mut().zip(COEFFICIENTS.iter()) {
        *slot = hashed
            .iter()
            .map(|h| (a * h + b) % MERSENNE)
            .min()
            .map_or(u64::MAX, |least| u64::try_from(least).unwrap_or(u64::MAX));
    }
    signature
}
