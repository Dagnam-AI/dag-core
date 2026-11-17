"""The core's character classes against the running interpreter's `re`, on every code point.

Run against the BUILT wheel on each CPython the package supports (3.12, 3.13, 3.14 and the
free-threaded 3.14t carry Unicode 15.0.0, 15.1.0, 16.0.0 and 16.0.0): each leg proves the table
it selects. The core exposes no predicate, so each class is probed through a finder that reads
nothing else of the character: an address `a@b.<c>` for `\\w`, a phone number of ten `<c>` for
`\\d`, and a phone number with one `<c>` among its digits for `\\s`.
"""

import re
import subprocess
import sys
import textwrap
import unicodedata

import pytest
from dagnam_core import hygiene

SUPPORTED = {"15.0.0", "15.1.0", "16.0.0"}
SURROGATES = range(0xD800, 0xE000)
CODE_POINTS = [c for c in range(0x110000) if c not in SURROGATES]
CHUNK = 1 << 16
WORD, DIGIT, SPACE = re.compile(r"\w"), re.compile(r"\d"), re.compile(r"\s")


def test_the_package_reports_the_interpreters_unicode_and_supports_it() -> None:
    assert hygiene.UNICODE_VERSION == unicodedata.unidata_version
    assert hygiene.UNICODE_VERSION in SUPPORTED
    assert hygiene.UNICODE_SUPPORTED is True


def _members(
    codes: list[int], probe: str, code: str, width: int, sep: str, at: int, size: int
) -> set[int]:
    """The code points whose probe text `findings` reports as one finding of class `code`.

    `probe` has `{c}` where the character goes; a probe is `width` code points before `sep`.
    """
    text = "".join(probe.format(c=chr(c)) + sep for c in codes)
    found = {
        start // width
        for start, end, kind in hygiene.findings(text)
        if kind == code and start % width == at and end - start == size
    }
    return {codes[i] for i in found}


def _first_difference(got: set[int], want: set[int]) -> str:
    """The counts and the first code point on which two sets differ, never the sets."""
    first = min(got ^ want, default=None)
    return f"{len(got)} members, {len(want)} expected; first difference {first!r}"


def _check(oracle: set[int], probe_of: str, code: str, size: int, sep: str, at: int) -> None:
    width = len(probe_of.format(c="x")) + len(sep)
    for low in range(0, len(CODE_POINTS), CHUNK):
        codes = CODE_POINTS[low : low + CHUNK]
        got = _members(codes, probe_of, code, width, sep, at, size)
        want = {c for c in codes if c in oracle}
        if got != want:
            pytest.fail(_first_difference(got, want))


def test_every_code_points_word_class_is_the_interpreters() -> None:
    # `[\w.\-]*` is the domain run: a hyphen is in it by itself, and no other non-word code point.
    oracle = {c for c in CODE_POINTS if WORD.fullmatch(chr(c))} | {ord("-")}
    _check(oracle, "a@b.{c}", "PII_EMAIL", size=5, sep=" ", at=0)


def test_every_code_points_digit_class_is_the_interpreters() -> None:
    oracle = {c for c in CODE_POINTS if DIGIT.fullmatch(chr(c))}
    # Ten of the character after a plus: a phone number only when each is a decimal digit.
    _check(oracle, "+" + "{c}" * 10, "PII_PHONE", size=11, sep="\n", at=0)


def test_every_code_points_space_class_is_the_interpreters() -> None:
    # The body of a number takes digits, spaces and `.()-`; the first two classes are the ones
    # under test, so the oracle holds every code point the body takes.
    body = {c for c in CODE_POINTS if SPACE.fullmatch(chr(c)) or DIGIT.fullmatch(chr(c))}
    oracle = body | set(map(ord, ".()-"))
    _check(oracle, "+12345{c}67890", "PII_PHONE", size=12, sep="\n", at=0)


def test_the_classes_are_not_vacuous() -> None:
    # Tie the oracle to something visible: the examples that moved between Unicode versions.
    assert bool(WORD.fullmatch("\U0002ebf0")) == (hygiene.findings("a@b.\U0002ebf0") != [])
    assert bool(WORD.fullmatch("Ᲊ")) == (hygiene.findings("a@b.Ᲊ") != [])
    assert bool(DIGIT.fullmatch("\U00010d40")) == (hygiene.findings("+" + "\U00010d40" * 10) != [])


def test_an_interpreter_without_tables_imports_and_refuses_every_call() -> None:
    """The unsupported case, with `unicodedata` standing in for an interpreter this wheel was not
    built for (CPython 3.15 and later carry a Unicode version the core has no tables for)."""
    script = textwrap.dedent(
        """
        import sys, types
        fake = types.ModuleType("unicodedata")
        fake.unidata_version = "99.1.2"
        sys.modules["unicodedata"] = fake
        from dagnam_core import hygiene
        assert hygiene.CORE_API == 2
        assert hygiene.UNICODE_VERSION == "99.1.2"
        assert hygiene.UNICODE_SUPPORTED is False
        secret = "SENTINEL jane@example.com"
        calls = [
            lambda: hygiene.findings(secret),
            lambda: hygiene.minhash_signature([secret]),
            lambda: hygiene.near_duplicates([secret], 0.5),
        ]
        for call in calls:
            try:
                call()
            except RuntimeError as error:
                assert str(error) == "dagnam-core has no character tables for Unicode 99.1.2", error
                assert "SENTINEL" not in repr(error.args)
            else:
                raise SystemExit("an unsupported interpreter was served")
        print("refused")
        """
    )
    done = subprocess.run(
        [sys.executable, "-I", "-c", script], capture_output=True, text=True, check=False
    )
    assert (done.returncode, done.stdout.strip(), done.stderr) == (0, "refused", "")


@pytest.mark.parametrize("version", sorted(SUPPORTED))
def test_a_supported_version_is_served_not_refused(version: str) -> None:
    script = (
        "import sys, types\n"
        f"fake = types.ModuleType('unicodedata'); fake.unidata_version = {version!r}\n"
        "sys.modules['unicodedata'] = fake\n"
        "from dagnam_core import hygiene\n"
        "assert hygiene.UNICODE_SUPPORTED is True and hygiene.UNICODE_VERSION == "
        f"{version!r}\n"
        "assert hygiene.findings('mail a@b.co now') == [(5, 11, 'PII_EMAIL')]\n"
        "print('served')\n"
    )
    done = subprocess.run(
        [sys.executable, "-I", "-c", script], capture_output=True, text=True, check=False
    )
    assert (done.returncode, done.stdout.strip(), done.stderr) == (0, "served", "")
