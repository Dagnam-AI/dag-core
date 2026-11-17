"""Run against the BUILT wheel (scripts/check.sh installs it); never against the source tree."""

import decimal
import fractions
import threading
import time
from collections.abc import Callable

import dagnam_core
import pytest
from dagnam_core import hygiene

MASK64 = 2**64 - 1


def test_the_package_reports_its_version_and_api() -> None:
    assert dagnam_core.__version__ == "0.1.1"
    assert hygiene.CORE_API == 1


def test_findings_reports_an_email_span_in_code_points() -> None:
    assert hygiene.findings("mail a@b.co now") == [(5, 11, "PII_EMAIL")]


def test_findings_counts_code_points_not_bytes() -> None:
    assert hygiene.findings("é\U0001f600 mail a@b.co") == [(8, 14, "PII_EMAIL")]


def test_findings_of_clean_text_is_empty() -> None:
    assert hygiene.findings("") == []
    assert hygiene.findings("plain prose, nothing to find") == []


def test_a_lone_surrogate_is_a_unicode_encode_error() -> None:
    with pytest.raises(UnicodeEncodeError):
        hygiene.findings("a\udc80b")


def test_a_lone_surrogate_in_any_sequence_element_is_a_unicode_encode_error() -> None:
    with pytest.raises(UnicodeEncodeError):
        hygiene.minhash_signature(["ok", "a\udc80b"])
    with pytest.raises(UnicodeEncodeError):
        hygiene.near_duplicates(["ok", "a\udc80b"], 0.5)


def test_the_signature_of_no_shingles_is_all_ones() -> None:
    assert hygiene.minhash_signature([]) == (MASK64,) * 128


def test_the_signature_is_a_tuple_of_128_ints_and_ignores_repeats() -> None:
    signature = hygiene.minhash_signature(["a b c", "b c d"])
    assert isinstance(signature, tuple)
    assert len(signature) == 128
    assert all(isinstance(v, int) and v < MASK64 for v in signature)
    assert hygiene.minhash_signature(["b c d", "a b c", "a b c"]) == signature


def test_near_duplicates_pairs_equal_texts() -> None:
    texts = [
        "the quick brown fox jumps",
        "something else entirely here",
        "the quick brown fox jumps",
    ]
    assert hygiene.near_duplicates(texts, 1.0) == ([2], [(0, 2, 1.0)])


def test_near_duplicates_of_nothing_is_empty() -> None:
    assert hygiene.near_duplicates([], 0.9) == ([], [])


@pytest.mark.parametrize(
    ("threshold", "shown"),
    [(0.0, "0.0"), (-0.5, "-0.5"), (1.5, "1.5"), (float("nan"), "nan"), (float("inf"), "inf")],
)
def test_a_threshold_outside_the_range_is_the_references_value_error(
    threshold: float, shown: str
) -> None:
    with pytest.raises(ValueError, match="threshold") as caught:
        hygiene.near_duplicates(["a b c"], threshold)
    assert str(caught.value) == f"threshold must be in (0, 1]; got {shown}"


def test_the_exact_value_error_message() -> None:
    with pytest.raises(ValueError) as caught:
        hygiene.near_duplicates(["a b c"], 0.0)
    assert str(caught.value) == "threshold must be in (0, 1]; got 0.0"


def test_a_wrong_type_error_does_not_echo_the_input() -> None:
    marker = "marker-text-that-must-not-appear"
    for call in (
        lambda: hygiene.near_duplicates([marker, 5], 0.5),  # pyright: ignore[reportArgumentType]
        lambda: hygiene.minhash_signature([marker, 5]),  # pyright: ignore[reportArgumentType]
        lambda: hygiene.findings(5),  # pyright: ignore[reportArgumentType]
        lambda: hygiene.near_duplicates([marker], marker),  # pyright: ignore[reportArgumentType]
    ):
        with pytest.raises(TypeError) as caught:
            call()
        assert marker not in str(caught.value)


MARKER = "marker-text-that-must-not-appear"
BAD = f"{MARKER} \udc80"


@pytest.mark.parametrize(
    "call",
    [
        lambda: hygiene.findings(BAD),
        lambda: hygiene.minhash_signature([BAD]),
        lambda: hygiene.minhash_signature(["fine", BAD]),
        lambda: hygiene.minhash_signature(s for s in [BAD]),
        lambda: hygiene.near_duplicates([BAD], 0.5),
        lambda: hygiene.near_duplicates(["fine", BAD], 0.5),
    ],
)
def test_a_lone_surrogate_error_never_carries_the_text(call: Callable[[], object]) -> None:
    with pytest.raises(UnicodeEncodeError) as caught:
        call()
    error = caught.value
    assert error.object == ""
    for shown in (str(error), repr(error), repr(error.args), repr(error.object)):
        assert MARKER not in shown
    assert error.args[0] == "utf-8"


def test_minhash_takes_any_iterable_of_str_but_not_a_str() -> None:
    expected = hygiene.minhash_signature(["a b c", "b c d"])
    assert hygiene.minhash_signature(frozenset(["a b c", "b c d"])) == expected
    assert hygiene.minhash_signature({"a b c", "b c d"}) == expected
    assert hygiene.minhash_signature(s for s in ["b c d", "a b c"]) == expected
    assert hygiene.minhash_signature(iter(["a b c", "b c d"])) == expected
    with pytest.raises(TypeError, match="not a str"):
        hygiene.minhash_signature("a b c")
    with pytest.raises(TypeError):
        hygiene.minhash_signature(5)  # pyright: ignore[reportArgumentType]


@pytest.mark.parametrize(
    ("threshold", "shown"),
    [
        (0, "0"),
        (-1, "-1"),
        (2, "2"),
        (False, "False"),
        (-0.0, "-0.0"),
        (fractions.Fraction(3, 2), "3/2"),
        (fractions.Fraction(10**20 + 1, 10**20), "100000000000000000001/100000000000000000000"),
        (decimal.Decimal("1.5"), "1.5"),
        (10**400, str(10**400)),
        (-(10**400), str(-(10**400))),
        (float("-inf"), "-inf"),
    ],
)
def test_the_value_error_formats_the_threshold_as_the_reference_does(
    threshold: object, shown: str
) -> None:
    with pytest.raises(ValueError) as caught:
        hygiene.near_duplicates(["a b c"], threshold)  # pyright: ignore[reportArgumentType]
    assert str(caught.value) == f"threshold must be in (0, 1]; got {shown}"


@pytest.mark.parametrize(
    "threshold",
    [1, True, 0.5, fractions.Fraction(1, 2), decimal.Decimal("0.5"), fractions.Fraction(1, 1)],
)
def test_a_threshold_of_any_numeric_type_in_range_is_accepted(threshold: object) -> None:
    texts = ["the quick brown fox jumps", "the quick brown fox jumps"]
    assert hygiene.near_duplicates(texts, threshold) == ([1], [(0, 1, 1.0)])  # pyright: ignore[reportArgumentType]


def test_a_nan_threshold_is_refused_and_a_decimal_nan_raises_as_the_reference_does() -> None:
    with pytest.raises(decimal.InvalidOperation):
        hygiene.near_duplicates(["a"], decimal.Decimal("NaN"))  # pyright: ignore[reportArgumentType]


def test_the_threshold_is_checked_before_any_text_is_read() -> None:
    with pytest.raises(ValueError, match="threshold"):
        hygiene.near_duplicates([BAD], 0.0)
    with pytest.raises(ValueError, match="threshold"):
        hygiene.near_duplicates(5, 0.0)  # pyright: ignore[reportArgumentType]


def test_near_duplicates_refuses_a_bare_str_for_the_texts() -> None:
    with pytest.raises(TypeError):
        hygiene.near_duplicates("a b c", 0.5)  # pyright: ignore[reportArgumentType]


# The GIL tests. Counting increments proves nothing: a thread that waits for the GIL during a
# call that holds it is handed the GIL as the call returns, and counts for a whole switch
# interval before the test reads its counter. So the spinner stamps the clock and the test asks
# for a stamp strictly inside the middle half of the call, which only a released GIL allows.


def _stamps_inside(call: Callable[[], object]) -> tuple[int, float]:
    """Run `call` while a thread stamps the clock; (stamps in the middle half, call seconds)."""
    stamps: list[float] = []
    started = threading.Event()
    stop = threading.Event()

    def spin() -> None:
        started.set()
        n = 0
        while not stop.is_set():
            n += 1
            if n % 1000 == 0:
                stamps.append(time.perf_counter())

    thread = threading.Thread(target=spin)
    thread.start()
    try:
        assert started.wait(5)
        time.sleep(0.01)  # let the spinner settle into its loop
        first = time.perf_counter()
        call()
        last = time.perf_counter()
    finally:
        stop.set()
        thread.join()
    quarter = (last - first) / 4
    inside = [t for t in stamps if first + quarter < t < last - quarter]
    return len(inside), last - first


def _assert_the_gil_is_released(call: Callable[[], object]) -> None:
    for _ in range(3):
        inside, took = _stamps_inside(call)
        assert took > 0.02, f"the call took {took:.4f} s: too short to tell"
        assert inside > 0, "no thread ran in the middle of the call: the GIL was held"


def test_the_gil_is_released_while_findings_runs() -> None:
    text = ("mail a@b.co or call 415-555-0132 today. " * 60_000)[: 2 * 1024 * 1024]
    _assert_the_gil_is_released(lambda: hygiene.findings(text))


def test_the_gil_is_released_while_minhash_signature_runs() -> None:
    shingles = [f"w{i} w{i + 1} w{i + 2}" for i in range(500_000)]
    _assert_the_gil_is_released(lambda: hygiene.minhash_signature(shingles))


def test_the_gil_is_released_while_near_duplicates_runs() -> None:
    texts = [" ".join(f"w{(i * 7 + j * 13) % 5000}" for j in range(40)) for i in range(8_000)]
    _assert_the_gil_is_released(lambda: hygiene.near_duplicates(texts, 0.9))
