# Changelog

All notable changes to `dagnam-core` are listed here, newest first. The version is the one in
`pyproject.toml`; a release is the tag `core/vX.Y.Z` (see RELEASE.md).

## 0.1.0

First release.

- `dagnam_core.hygiene`: PII findings (`findings`), the 128-value MinHash signature
  (`minhash_signature`) and near-duplicate detection (`near_duplicates`), each releasing the GIL
  while it runs.
- `dagnam_core.hygiene.CORE_API == 1`: the binding contract version.
- Wheels for Linux (x86_64 and aarch64, glibc 2.28 and musl 1.2), macOS (x86_64 and arm64) and
  Windows (x86_64): one `abi3` wheel per platform for CPython 3.12 and later, plus a free-threaded
  wheel for CPython 3.14t (version-specific, not `abi3`). An sdist for everything else.
- Published by trusted publishing from a tag, with build provenance attestations; the release is
  exactly the 15 expected files, each wheel tested on its own platform before upload.
