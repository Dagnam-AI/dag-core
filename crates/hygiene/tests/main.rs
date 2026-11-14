//! Integration tests for `dagnam-hygiene`.
// Test binary: failing loudly is the point, and clippy.toml does not cover helper functions.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

#[test]
fn the_core_api_version_starts_at_one() {
    assert_eq!(dagnam_hygiene::core_api(), 1);
}
