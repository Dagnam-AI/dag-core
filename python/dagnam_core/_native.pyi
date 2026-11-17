from collections.abc import Iterable, Sequence

__all__ = ["CORE_API", "findings", "minhash_signature", "near_duplicates"]

CORE_API: int

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

    Memory grows with the square of the number of identical texts (a run of equal rows yields a
    pair for every two of them); limits on that are not set yet, so a caller should bound the
    batch. The call releases the GIL and cannot be interrupted: a Ctrl+C is seen when it returns.
    """
