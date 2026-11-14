//! Integration tests for `dagnam-hygiene`.

#[test]
fn the_core_api_version_starts_at_one() {
    assert_eq!(dagnam_hygiene::core_api(), 1);
}
