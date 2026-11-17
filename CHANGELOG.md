# Changelog

All notable changes to `dagnam-core` are listed here, newest first. The version is the one in
`pyproject.toml`; a release is the tag `core/vX.Y.Z` (see RELEASE.md).

## 0.1.1

Release fix; the code is the same as 0.1.0. The 0.1.0 tag built but its source distribution did not
hold the `LICENSE` file its metadata names, so the index refused it and nothing was released on PyPI
under that number.

- The source distribution now includes `LICENSE`, and the release workflow checks that every file
  the sdist metadata names is in the archive before anything is uploaded.

## 0.1.0

First release. Tagged, never published to PyPI (see 0.1.1).

- `dagnam_core.hygiene`: PII findings (`findings`), the 128-value MinHash signature
  (`minhash_signature`) and near-duplicate detection (`near_duplicates`), each releasing the GIL
  while it runs.
- `dagnam_core.hygiene.CORE_API == 1`: the binding contract version.
- Wheels for Linux (x86_64 and aarch64, glibc 2.28 and musl 1.2), macOS (x86_64 and arm64) and
  Windows (x86_64): one `abi3` wheel per platform for CPython 3.12 and later, plus a free-threaded
  wheel for CPython 3.14t (version-specific, not `abi3`). An sdist for everything else.
- Published by trusted publishing from a tag, with build provenance attestations; the release is
  exactly the 15 expected files, each wheel tested on its own platform before upload.
