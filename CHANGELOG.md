# Changelog

All notable changes to `dagnam-core` are listed here, newest first. The version is the one in
`pyproject.toml`; a release is the tag `core/vX.Y.Z` (see RELEASE.md).

## Unreleased: 0.2.0

- The character classes follow the interpreter. Python's `\w`, `\d` and `\s` differ between
  Unicode 15.0.0 (CPython 3.12), 15.1.0 (3.13) and 16.0.0 (3.14, 3.14t), so the core now ships a
  table for each and selects the one that matches `unicodedata.unidata_version` when
  `dagnam_core.hygiene` is imported. The 622 `\w` code points on 3.12 and the 4,382 `\w` and 80 `\d`
  on 3.14 that the 0.1 line classified as on 3.13 now agree with the running interpreter.
- New: `hygiene.UNICODE_VERSION` (the interpreter's version string) and `hygiene.UNICODE_SUPPORTED`
  (whether the core has its tables). On an interpreter with another Unicode version `findings`,
  `minhash_signature` and `near_duplicates` raise `RuntimeError("dagnam-core has no character
  tables for Unicode X.Y.Z")` instead of answering under the wrong tables.
- `hygiene.CORE_API` is 2 (was 1): the functions now carry that precondition. A process that replaces
  `unicodedata` with another module before importing the package is unsupported: the package trusts
  `unicodedata.unidata_version` as the interpreter's own.
- Rust: `dagnam_hygiene::UnicodeVersion` (`V15_0`, `V15_1`, `V16_0`); every finder, `findings`,
  `luhn_ok`, `verhoeff_ok`, `iban_mod97_ok` and `is_ip_address` take the version, the predicates
  and class spellings are `is_py_word(version, c)` and `word_class(version)` (the `*_CLASS`
  statics are gone). Near-duplicate detection reads no character class and takes none.
- Wheels for older Linux: the glibc wheels are now `manylinux_2_17` (manylinux 2014, was
  `manylinux_2_28`), so they install on CentOS/RHEL 7, Amazon Linux 2 and Ubuntu 16.04 and 18.04
  as well. The musl wheels are unchanged (`musllinux_1_2`).
- No Rust build for anyone: every release also carries a pure-Python stub wheel
  (`py3-none-any`). Where no native wheel fits (free-threaded CPython 3.13, PyPy 3.12 and later, Windows arm64,
  a glibc older than 2.17, ...) pip and uv install the stub, which is preferred to the sdist, and
  `import dagnam_core.hygiene` raises `ImportError("dagnam-core has no compiled build for this
  interpreter or platform; callers fall back to their pure-Python code")`, so callers use their
  Python implementation. Supported platforms keep their native wheel. There is no free-threaded 3.13 wheel: PyO3 0.29 and
  later dropped free-threaded CPython older than 3.14, and 0.28.3, the newest release that supports
  it, carries RUSTSEC-2026-0177, so the core stays on PyO3 0.30. The release is 16 files
  (was 15), and the release workflow now installs with pip and uv from those files alone to prove
  the selection.
- `scripts/gen_py_classes.py` writes the table of the running interpreter's version and `--check`
  verifies its table file and its `corpus/classes.jsonl` line.

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
- `dagnam_core.hygiene.CORE_API == 1`: the binding contract version (2 from 0.2.0).
- Wheels for Linux (x86_64 and aarch64, glibc 2.28 and musl 1.2), macOS (x86_64 and arm64) and
  Windows (x86_64): one `abi3` wheel per platform for CPython 3.12 and later, plus a free-threaded
  wheel for CPython 3.14t (version-specific, not `abi3`). An sdist for everything else.
- Published by trusted publishing from a tag, with build provenance attestations; the release is
  exactly the 15 expected files, each wheel tested on its own platform before upload.
