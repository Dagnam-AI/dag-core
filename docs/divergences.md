# Divergences

Places where the compiled core deliberately behaves differently from the Python code it
replaces, each with the reason and the test that pins it. A divergence that is not listed here
is a bug.

No divergence is listed: on every supported interpreter the core and the Python reference agree
on every input the corpus and the property tests reach. The one difference of shape, which is not a
difference of answer, is where the size limits of a near-duplicate batch are read (below).

## Near-duplicate limits: two readings, two places

The reference reads `MAX_TEXTS`, `MAX_TEXT_CHARS` and `MAX_TOTAL_CHARS` twice: on the raw rows as
they were sent, before any is normalised (so a row of a hundred megabytes costs nothing), and again
on the normalised texts, which casefolding can lengthen. The core is given the normalised texts
only, so it reads the second one, with the reference's message for it, and the first stays in the
caller that has the rows. `MAX_COMPARISONS` and `MAX_PAIRS` are the core's own, counted exactly as
the reference counts them. Pinned by `crates/hygiene/tests/near_dedup/` and `tests/test_near_dedup_limits.py`.

## Unicode version of the interpreter

Python's `\w`, `\d` and `\s` follow the Unicode database of the interpreter that runs them
(CPython 3.12: 15.0.0, 3.13: 15.1.0, 3.14 and the free-threaded 3.14t: 16.0.0), and a few
thousand code points are classified differently between them. The core ships a table for each of
the three versions and uses the one that matches `unicodedata.unidata_version` of the interpreter
that imports `dagnam_core.hygiene`, so there is no divergence on a supported interpreter:
`hygiene.UNICODE_VERSION` is that string and `hygiene.UNICODE_SUPPORTED` is `True`.

An interpreter whose Unicode version is none of the three (CPython 3.15 and later will carry a
newer one; 3.11 and earlier are not supported by the package at all) is **not served with
another version's tables**: `UNICODE_SUPPORTED` is `False`, and `findings`, `minhash_signature`
and `near_duplicates` raise `RuntimeError("dagnam-core has no character tables for Unicode
X.Y.Z")`, with no input text in the message. A caller falls back to the pure-Python reference.
A new CPython that changes the Unicode version needs a release of the core with its table
(`scripts/gen_py_classes.py`, run under that interpreter).

Pinned by `tests/test_unicode_tables.py` (every code point's `\w`, `\d` and `\s` as the core sees
it against `re` of the running interpreter, on each CPython leg; the unsupported-version
refusal), by `crates/hygiene/tests/text/classes.rs` (every predicate against `corpus/classes.jsonl`, one line
per version, on every code point), by `crates/hygiene/tests/text/versions.rs` (the finders differ exactly where
the versions do), and by `scripts/gen_py_classes.py --check` (each table is what its interpreter
writes).
