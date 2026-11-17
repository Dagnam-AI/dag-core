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
- A credential after a value an ambiguous name refuses is found, as in the reference (R1,
  `dagnam-contracts` after 0.6.0). `token`, `auth`, `key` and `authorization` refuse a value that is
  not a credential on its own, and the scan of bare values used to jump past the whole refused run,
  so `<name>=<16+ letters>?<name>=...` lost the assignment after the `?` (also `|`, `#`, `:-`, `:_`,
  `!` and `@`, and `/`, `+`, `.`, `=` when the second value has no digit). The walk now resumes right
  after the refused name, each name inside a refused value is judged on its own, and a name inside
  a credential taken is passed over only when its own value starts before that credential's end.
  Time and memory do not grow with the number of names in a run (`entropy_starts` reads the one
  verdict that needs the rest of the value, from the end, once per refused value). `findings`
  agrees with the reference on the whole regenerated corpus (14,075 credential cases, 14,630
  `PII_SECRET` spans found exactly) and on a differential of 1,370,000 random texts (0 mismatches).
- New: `NearDedupLimitError`, a `ValueError` subclass in `dagnam_core.hygiene`, and the five
  limits it names, exported as module attributes: `MAX_TEXTS` (50,000), `MAX_TEXT_CHARS`
  (1,000,000, characters in one text), `MAX_TOTAL_CHARS` (20,000,000), `MAX_COMPARISONS`
  (40,000,000 candidate pairs compared, verified or not, counted before each comparison) and
  `MAX_PAIRS` (20,000,000 near-duplicate pairs, counted before each is kept). Same numbers and
  same messages as the reference; no message holds a word of the batch. `near_duplicates` reads
  them in the reference's order after the threshold: the count, then each text's length from
  the `str` object (before any text is copied out of Python) and the running total, then the
  walk, which counts the pairs and holds nothing; the result is built only for a batch that has
  passed. The count of a sequence is read before any item is collected, so a list of ten million
  items is refused without a copy of it. The reference reads the same size limits first on the
  raw rows, before it normalises them: that reading stays in the caller, which has the rows.
  `hygiene.CORE_API` stays 2.
- `near_duplicates` no longer holds the candidate pairs. They are streamed in the reference's
  order (the lower text ascending, each one's partners ascending, each pair once) from a pointer
  per text and band, where the 0.1 line built an ordered set of all of them. The Rust crate alone,
  peak resident memory of a release build: 3,000 identical texts 115 MB, was 274 MB; 3,000 texts
  that share a word 7 MB, was 73 MB. The result is unchanged.
- The Python result is built from shared objects: one `int` per text index and one `float` per
  distinct similarity (at most 129), each pair a tuple of three of them, read from the core a
  chunk at a time so the pairs are never all held in Rust and in Python together. A pair costs
  about 73 bytes through Python (the tuple and its slot in the list). Peak resident memory of a
  fresh process, release wheel, measured on the whole call: 3,000 identical texts (4,498,500
  pairs) 362 MB, and exactly `MAX_PAIRS` pairs 1,488 MB; the pure-Python reference measured 367 MB
  and 1,471 MB on the same two batches, so the core is at the reference's footprint (1% above it
  at `MAX_PAIRS`), not under it.
- Rust: `dagnam_hygiene::near_duplicates` returns `Result<_, NearDedupError>` (was `Option`; the
  threshold is `NearDedupError::Threshold`); new `near_duplicates_within(texts, threshold,
  Limits)`, `Limits`, `Batch` (the same checks, then the pairs read in chunks), the `MAX_*`
  constants and `check_text_count` / `check_text_lengths`, which a binding calls on the sizes it
  reads off its own strings before it copies them. The two limit errors carry the limit in force,
  so their message names a smaller `Limits` value, as the reference does with a patched constant.
- The reference corpus is re-exported from `dagnam-contracts` at `8ee5f42` (the credentials and
  findings files; the others are byte-identical): see `corpus/README.md`.

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
