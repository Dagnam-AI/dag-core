from collections.abc import Iterable, Sequence

__all__ = [
    "CORE_API",
    "MAX_COMPARISONS",
    "MAX_PAIRS",
    "MAX_TEXTS",
    "MAX_TEXT_CHARS",
    "MAX_TOTAL_CHARS",
    "UNICODE_SUPPORTED",
    "UNICODE_VERSION",
    "NearDedupLimitError",
    "findings",
    "minhash_signature",
    "near_duplicates",
]

CORE_API: int
"""The binding contract version: 2 since the tables follow the interpreter's Unicode version."""

UNICODE_VERSION: str
"""`unicodedata.unidata_version` of the interpreter that imported the package, as it spells it."""

UNICODE_SUPPORTED: bool
"""Whether the core has character tables for `UNICODE_VERSION` (15.0.0, 15.1.0 and 16.0.0).

When it is false, `findings`, `minhash_signature` and `near_duplicates` raise `RuntimeError`
(`dagnam-core has no character tables for Unicode X.Y.Z`; the message holds no input text), and a
caller falls back to the pure-Python reference.
"""

MAX_TEXTS: int
"""The most texts one `near_duplicates` batch holds: 50,000 (more raises `NearDedupLimitError`)."""

MAX_TEXT_CHARS: int
"""The most characters in one text of a batch: 1,000,000."""

MAX_TOTAL_CHARS: int
"""The most characters in all the texts of a batch together: 20,000,000."""

MAX_COMPARISONS: int
"""The most candidate pairs `near_duplicates` compares: 40,000,000.

A candidate pair is two texts that share an LSH band, counted once however many bands they share,
whether or not it turns out to be a near-duplicate. The call raises instead of making the
comparison after the last one this allows.
"""

MAX_PAIRS: int
"""The most near-duplicate pairs `near_duplicates` returns: 20,000,000.

The result lists every pair, so a batch of n identical texts has n * (n - 1) / 2 of them (about
6,300 identical texts reach this, and texts with no words are all the same text).
"""

class NearDedupLimitError(ValueError):
    """A `near_duplicates` batch is larger than the core takes (see the five `MAX_` constants).

    The message is the reference's (`dagnam_contracts.hygiene.NearDedupLimitError`) for the same
    limit and value, names the limit and its value, and holds no text of the batch.
    """

def findings(text: str) -> list[tuple[int, int, str]]:
    """Every PII finding in `text`, sorted by start, as `(start, end, class code)`.

    Offsets are code points. A `str` that is not UTF-8 (a lone surrogate) raises
    `UnicodeEncodeError` with an empty `object`: the exception never carries the text.

    The call releases the GIL, so other threads run, but it cannot be interrupted: a Ctrl+C
    is seen when the call returns.
    """

def minhash_signature(shingles: Iterable[str]) -> tuple[int, ...]:
    """The 128-value MinHash signature of the shingles; all `2**64 - 1` when there are none.

    Any iterable of `str` is taken (a set, a frozenset, a generator), but a bare `str` is
    refused with `TypeError`: iterating one yields characters, which is almost certainly a bug.
    A repeated shingle changes nothing.
    """

def near_duplicates(
    texts: Sequence[str], threshold: float
) -> tuple[list[int], list[tuple[int, int, float]]]:
    """The indices of every text that duplicates an earlier one, and each verified pair.

    Pairs are `(earlier, later, similarity)`. The lowest index of a cluster is never reported.

    `texts` must be normalised exactly like the reference's `row_text`: casefolded, punctuation
    removed, a single space between words. Only the space separates words here, so other
    whitespace changes the clusters (no error is raised).

    `threshold` is checked first, with Python's own comparison, before any text is read, and a
    value outside `(0, 1]` raises `ValueError("threshold must be in (0, 1]; got <str of it>")`
    exactly like the reference. It is then converted to a float: a `Decimal` or `Fraction` that
    is above a pair's similarity by less than a float can resolve is rounded onto it.
    A positive one so small that it rounds to 0.0 (for example `Decimal("1e-400")`) is refused,
    where the reference accepts it.

    A batch has limits, read in this order after the threshold and before any text is copied
    out of Python or any memory is allocated for the work: more than `MAX_TEXTS` texts; then, text
    by text, one of more than `MAX_TEXT_CHARS` characters or a running total of more than
    `MAX_TOTAL_CHARS`; then, while the pairs are counted (nothing is held), more than
    `MAX_COMPARISONS` candidate pairs compared (counted before each comparison, so the call stops
    instead of making the one past the limit) or more than `MAX_PAIRS` near-duplicate pairs (the
    same). Each raises `NearDedupLimitError`, a `ValueError`. The sizes are read on the texts as
    given, which the caller has normalised; a caller that holds the raw rows reads them there
    first. The result is built only after the batch has passed, and it still grows with the
    square of the identical texts. It is made of shared objects (one `int` per text index, one
    `float` per distinct similarity), so a pair costs its 3-tuple and the list slot, about 73
    bytes: 3,000 identical texts (4.5 million pairs) peak at 362 MB resident and a result of
    exactly `MAX_PAIRS` pairs at 1,488 MB, measured in a fresh process, about the pure-Python
    reference's own footprint. `MAX_PAIRS` bounds it.

    The call releases the GIL. A Ctrl+C is seen once the counting walk has finished, between the
    chunks the result is built from, not during the walk.
    """
