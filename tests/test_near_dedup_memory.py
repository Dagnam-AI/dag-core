"""What `near_duplicates` costs in memory through Python, in a process of its own for each case.

The peak resident set of the process is the figure a caller pays: it holds the Rust work and the
Python result together. A process is started per case because a peak cannot be reset.
"""

import subprocess
import sys
from typing import Any

import pytest
from dagnam_core import hygiene

_PROBE = """
import sys
from dagnam_core import hygiene

if sys.platform == "win32":
    # The peak working set (`PeakWorkingSetSize`), the Windows counterpart of the peak resident set.
    import ctypes
    from ctypes import wintypes

    class Counters(ctypes.Structure):
        _fields_ = [
            ("cb", wintypes.DWORD),
            ("PageFaultCount", wintypes.DWORD),
            ("PeakWorkingSetSize", ctypes.c_size_t),
            ("WorkingSetSize", ctypes.c_size_t),
            ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
            ("QuotaPagedPoolUsage", ctypes.c_size_t),
            ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
            ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
            ("PagefileUsage", ctypes.c_size_t),
            ("PeakPagefileUsage", ctypes.c_size_t),
        ]

    kernel32 = ctypes.WinDLL("kernel32")
    kernel32.GetCurrentProcess.restype = wintypes.HANDLE
    kernel32.K32GetProcessMemoryInfo.argtypes = [
        wintypes.HANDLE, ctypes.POINTER(Counters), wintypes.DWORD
    ]
    kernel32.K32GetProcessMemoryInfo.restype = wintypes.BOOL

    def peak():
        counters = Counters()
        counters.cb = ctypes.sizeof(Counters)
        if not kernel32.K32GetProcessMemoryInfo(
            kernel32.GetCurrentProcess(), ctypes.byref(counters), counters.cb
        ):
            raise ctypes.WinError()
        return counters.PeakWorkingSetSize
else:
    import resource

    def peak():
        # Bytes on macOS, kibibytes on Linux.
        rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
        return rss if sys.platform == "darwin" else rss * 1024

{setup}
before = peak()
{call}
print(before, peak())
"""


def _peaks(setup: str, call: str) -> tuple[int, int]:
    code = _PROBE.format(setup=setup, call=call)
    # Both numbers come from the child itself, so the bound on their difference does not depend
    # on how large the interpreter's own working set is on the platform.
    out = subprocess.run(
        [sys.executable, "-c", code], capture_output=True, text=True, check=True
    ).stdout
    before, after = out.split()
    return int(before), int(after)


def test_a_refused_count_costs_no_copy_of_the_list() -> None:
    # 10 million items is 80 MB of pointers; collecting them before the count is read would
    # raise the peak by that much. Items that are not `str` show the count is read first.
    before, after = _peaks(
        "texts = [None] * 10_000_000",
        "try:\n    hygiene.near_duplicates(texts, 0.9)\n"
        "except hygiene.NearDedupLimitError:\n    pass",
    )
    assert after - before < 16 * 1024 * 1024


def test_a_count_past_the_most_is_refused_before_the_items_are_read() -> None:
    not_texts: list[Any] = [1] * (hygiene.MAX_TEXTS + 1)
    with pytest.raises(hygiene.NearDedupLimitError, match="MAX_TEXTS"):
        hygiene.near_duplicates(not_texts, 0.9)


def test_the_pairs_of_identical_texts_share_their_numbers() -> None:
    duplicates, pairs = hygiene.near_duplicates(["a b c d"] * 30, 0.9)
    assert len(pairs) == 435
    firsts = {id(pair[0]) for pair in pairs}
    assert len(firsts) == 29  # one int per text index, however many pairs it is in
    assert len({id(pair[2]) for pair in pairs}) == 1
    assert {id(pair[1]) for pair in pairs} <= {id(pair[0]) for pair in pairs} | {
        id(i) for i in duplicates
    }
    assert duplicates == list(range(1, 30))


def test_three_thousand_identical_texts_stay_near_the_size_of_their_result() -> None:
    # 4,498,500 pairs. Each is one tuple (64 bytes) and its slot in the list (8): 72 bytes. The
    # bound leaves room for the interpreter, the texts and the signatures.
    before, after = _peaks(
        "texts = ['a b c d'] * 3000",
        "hygiene.near_duplicates(texts, 0.9)",
    )
    assert after - before < 4_498_500 * 90 + 32 * 1024 * 1024
