"""The limits of a `near_duplicates` batch, at their real values, on the BUILT wheel.

The numbers and the messages are those of the reference (`dagnam_contracts.hygiene`, the release
that holds `NearDedupLimitError`); they are written out here because the wheel's tests do not
install the reference.
"""

import pytest
from dagnam_core import _native, hygiene

MAX_TEXTS = 50_000
MAX_TEXT_CHARS = 1_000_000
MAX_TOTAL_CHARS = 20_000_000
MAX_COMPARISONS = 40_000_000
MAX_PAIRS = 20_000_000


def test_the_constants_are_the_references_and_both_modules_export_them() -> None:
    assert (
        hygiene.MAX_TEXTS,
        hygiene.MAX_TEXT_CHARS,
        hygiene.MAX_TOTAL_CHARS,
        hygiene.MAX_COMPARISONS,
        hygiene.MAX_PAIRS,
    ) == (MAX_TEXTS, MAX_TEXT_CHARS, MAX_TOTAL_CHARS, MAX_COMPARISONS, MAX_PAIRS)
    assert hygiene.NearDedupLimitError is _native.NearDedupLimitError


def test_the_error_is_a_value_error_of_its_own_in_the_hygiene_module() -> None:
    error = hygiene.NearDedupLimitError
    assert issubclass(error, ValueError)
    assert error is not ValueError
    assert (error.__module__, error.__name__) == ("dagnam_core.hygiene", "NearDedupLimitError")
    assert error.__doc__


def _refused(texts: list[str]) -> str:
    with pytest.raises(hygiene.NearDedupLimitError) as caught:
        hygiene.near_duplicates(texts, 0.9)
    return str(caught.value)


def test_one_text_past_the_most_is_refused_with_the_references_message() -> None:
    assert _refused([""] * (MAX_TEXTS + 1)) == "50001 texts is more than MAX_TEXTS (50000)"


def test_a_text_past_the_most_characters_is_refused_whatever_its_width() -> None:
    message = "a text of 1000001 characters is more than MAX_TEXT_CHARS (1000000)"
    assert _refused(["a" * (MAX_TEXT_CHARS + 1)]) == message
    assert _refused(["é" * (MAX_TEXT_CHARS + 1)]) == message
    assert _refused(["\U0001f600" * (MAX_TEXT_CHARS + 1)]) == message


def test_a_text_at_the_most_characters_is_taken() -> None:
    assert hygiene.near_duplicates(["é" * MAX_TEXT_CHARS], 0.9) == ([], [])


def test_texts_past_the_most_characters_together_are_refused() -> None:
    texts = ["a" * MAX_TEXT_CHARS] * (MAX_TOTAL_CHARS // MAX_TEXT_CHARS) + ["a"]
    assert _refused(texts) == ("the texts hold more than MAX_TOTAL_CHARS (20000000) characters")


def test_the_count_is_read_before_the_length_of_any_text() -> None:
    texts = ["a" * (MAX_TEXT_CHARS + 1)] + [""] * MAX_TEXTS
    assert _refused(texts) == "50001 texts is more than MAX_TEXTS (50000)"


def test_a_batch_that_is_too_large_is_refused_before_its_texts_are_read() -> None:
    # A lone surrogate is an encoding error once a text is read; a refused batch never reads it.
    with pytest.raises(hygiene.NearDedupLimitError):
        hygiene.near_duplicates(["\udc80", *[""] * MAX_TEXTS], 0.9)
    with pytest.raises(hygiene.NearDedupLimitError):
        hygiene.near_duplicates(["\udc80" * (MAX_TEXT_CHARS + 1)], 0.9)


def test_the_threshold_is_read_before_any_limit() -> None:
    with pytest.raises(ValueError, match="threshold") as caught:
        hygiene.near_duplicates([""] * (MAX_TEXTS + 1), 0.0)
    assert not isinstance(caught.value, hygiene.NearDedupLimitError)


def test_identical_texts_past_the_pair_limit_are_refused() -> None:
    # 6,400 identical texts have 20,476,800 pairs: the walk stops at the 20,000,001st.
    texts = ["a b c d"] * 6_400
    assert _refused(texts) == "more than MAX_PAIRS (20000000) near-duplicate pairs"


def test_candidates_past_the_comparison_limit_are_refused() -> None:
    # About a third of the pairs of 16,000 texts that share a word are candidates (more than 40
    # million), and none of them is a near-duplicate: the walk stops at the 40,000,001st.
    texts = [f"common w{i}" for i in range(16_000)]
    assert _refused(texts) == "more than MAX_COMPARISONS (40000000) comparisons"


def test_no_message_holds_a_word_of_the_batch() -> None:
    marker = "marker-text-that-must-not-appear"
    assert marker not in _refused([marker * (MAX_TEXT_CHARS // len(marker) + 1)])
    assert marker not in _refused([marker] * (MAX_TEXTS + 1))
