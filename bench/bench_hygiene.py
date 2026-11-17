"""Admission benchmark: the Python reference against the compiled core, case by case.

Run in the environment that has the built `dagnam_core` wheel installed (a release wheel, on a
quiet machine: the load average is printed at the start and the end so the ledger can show it):

    python bench/bench_hygiene.py --reference <path to dag-contracts/python>

Each case runs the reference's own call and the native seam over the same input, checks the two
agree (outside the timing), and reports the best of `--repeat` runs of each. `--scale` shrinks
every case (a smoke test; the admission runs use the default of 1).

The cases are the three of the admission brief (records through `findings`, the worst-case set,
near-dedup) with the worst case as one row per shape, each shape built to 1 MB. The shapes are the
reference's own hardest ones: chains of findings that each hide the next, and long runs of one
character that every pattern reads. The text is names, emails, phone numbers, card numbers cut
from the corpus at run time, and plain prose; it holds nothing that looks like a credential.
"""

import argparse
import json
import os
import random
import sys
import time
from collections.abc import Callable
from pathlib import Path
from typing import Any

CORPUS = Path(__file__).resolve().parent.parent / "corpus" / "findings.jsonl"
WORDS_PER_ROW = 40
VOCABULARY = 5000
NAMES = ["Ada Lovelace", "Grace Hopper", "Alan Turing", "Edsger Dijkstra", "Barbara Liskov"]
PROSE = (
    "The assistant answers questions about the account politely, keeps replies short, and "
    "asks a clarifying question when the request is ambiguous. It never guesses a figure. "
)
# (name, head, unit, tail): the text is the head, as many units as fit, then the tail. None of
# them holds a credential. The first three are chains in which each finding hides the next.
SHAPES = [
    ("chain: Aadhaar keyword and number", "", "Aadhaar Number 580927525309", ""),
    ("chain: phone, phone, phone", "", "+1415555{:04d} ", ""),
    ("chain: phone glued to IBAN", "", "+44 20 7946 0958DE89 3704 0044 0532 0130 00", ""),
    ("emails, spaced", "", "a@b.co ", ""),
    ("one email, many domain labels", "a@b", ".c", ""),
    ("one giant run of at signs", "", "a@", ""),
    ("one giant run of digits", "", "1", ""),
    ("digits and spaces", "", "1 ", ""),
    ("one giant word", "", "a", ""),
    ("keyword, then a whitespace run", "SIN", " ", "x"),
]


def best_of(repeat: int, run: Callable[[], object]) -> float:
    best = float("inf")
    for _ in range(repeat):
        started = time.perf_counter()
        run()
        best = min(best, time.perf_counter() - started)
    return best


def _encodable(case: dict[str, Any]) -> bool:
    try:
        case["text"].encode("utf-8")
    except UnicodeEncodeError:
        return False
    return True


def corpus_cases() -> list[dict[str, Any]]:
    # Split on "\n" only: the texts hold U+2028 and the like unescaped.
    lines = CORPUS.read_text(encoding="utf-8").split("\n")
    cases = [json.loads(line) for line in lines if line]
    # A lone surrogate is not text the native seam accepts (it raises), so it is not benchmarked.
    return [case for case in cases if _encodable(case)]


def card_numbers(cases: list[dict[str, Any]]) -> list[str]:
    """Distinct payment-card spans, cut from the reference's own cases at run time."""
    cards = {
        case["text"][start:end]
        for case in cases
        for start, end, code in case["spans"]
        if code == "PII_PAYMENT_CARD"
    }
    return sorted(cards)


def records(n: int, cards: list[str]) -> list[list[str]]:
    """`n` chat records as their string leaves: a 3 KB system prompt, a user turn, a reply."""
    rng = random.Random(1)
    system = (PROSE * (3072 // len(PROSE) + 1))[:3072]
    out: list[list[str]] = []
    for i in range(n):
        name = rng.choice(NAMES)
        user = (
            f"Hi, this is {name}. Mail me at user{i}@example.com or call 415-555-{i % 10000:04d}. "
            f"My card is {rng.choice(cards)}. Why was I charged twice on the {i % 28 + 1}th?"
        )
        reply = f"Thanks {name.split()[0]}, I have checked order {i}. {PROSE}"
        out.append([system, user, reply])
    return out


def shape_text(head: str, unit: str, tail: str, size: int) -> str:
    """`head`, then `unit` repeated to about `size` characters, then `tail`."""
    count = max(1, (size - len(head) - len(tail)) // len(unit.format(0)))
    if "{" in unit:  # a numbered unit: distinct numbers, so no two findings are the same text
        return head + "".join(unit.format(i % 10000) for i in range(count)) + tail
    return head + unit * count + tail


def every_class_glued(cases: list[dict[str, Any]], size: int) -> str:
    """One single-finding corpus sample of each class, glued end to end, repeated to `size`."""
    sample: dict[str, str] = {}
    for case in cases:
        if len(case["spans"]) == 1:
            sample.setdefault(case["spans"][0][2], case["text"])
    unit = "".join(sample[code] for code in sorted(sample))
    return unit * max(1, size // len(unit))


def near_dedup_rows(n: int) -> list[dict[str, str]]:
    """`n` rows of random words; one row in ten copies an earlier one with a word changed."""
    rng = random.Random(1)
    rows: list[dict[str, str]] = []
    for i in range(n):
        if i and i % 10 == 0:
            words = rng.choice(rows)["t"].split()
            words[rng.randrange(WORDS_PER_ROW)] = f"w{rng.randrange(VOCABULARY)}"
        else:
            words = [f"w{rng.randrange(VOCABULARY)}" for _ in range(WORDS_PER_ROW)]
        rows.append({"t": " ".join(words)})
    return rows


def load() -> str:
    return " ".join(f"{x:.2f}" for x in os.getloadavg())


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--reference", required=True, help="path to dag-contracts/python")
    parser.add_argument("--repeat", type=int, default=5, help="runs per side; the best counts")
    parser.add_argument("--scale", type=float, default=1.0, help="size factor (smoke tests only)")
    args = parser.parse_args()
    # The reference is imported from --reference, so these imports follow the path insert.
    sys.path.insert(0, args.reference)

    import dagnam_core
    from dagnam_contracts.hygiene import near_dedup, spans
    from dagnam_core import hygiene

    print(f"load average at start: {load()}")
    binaries = [
        so for so in Path(dagnam_core.__file__).parent.glob("_native*") if so.suffix != ".pyi"
    ]
    print(f"dagnam_core: {dagnam_core.__file__}")
    for so in binaries:
        print(f"native module: {so.name}, {so.stat().st_size} bytes")
    print("(confirm the wheel came from `maturin build --release`; a debug build is far slower)")

    def scaled(n: int) -> int:
        return max(1, round(n * args.scale))

    def py_findings(items: list[str]) -> list[list[tuple[int, int, str]]]:
        return [[(a, b, d.code) for a, b, d in spans.findings(s)] for s in items]

    def rs_findings(items: list[str]) -> list[list[tuple[int, int, str]]]:
        return [hygiene.findings(s) for s in items]

    cases = corpus_cases()
    chat = records(scaled(2_000), card_numbers(cases))
    size = scaled(1_000_000)
    inputs: list[tuple[str, list[str], bool]] = [
        (f"records: {len(chat)} x 3 KB system prompt", [s for r in chat for s in r], False),
    ]
    inputs += [
        (f"worst: {name}", [shape_text(head, unit, tail, size)], True)
        for name, head, unit, tail in SHAPES
    ]
    inputs.append(
        ("worst: every class glued (corpus samples)", [every_class_glued(cases, size)], True)
    )

    table: list[tuple[str, float, float, bool, bool]] = []
    for name, items, worst in inputs:
        same = py_findings(items) == rs_findings(items)
        py = best_of(args.repeat, lambda items=items: py_findings(items))
        rs = best_of(args.repeat, lambda items=items: rs_findings(items))
        table.append((name, py, rs, same, worst))

    rows = near_dedup_rows(scaled(5_000))

    def py_dedup() -> object:
        result = near_dedup.compute_near_duplicates(rows)
        pairs = [(p.kept_index, p.duplicate_index, p.similarity) for p in result.pairs]
        return result.duplicate_indices, pairs

    def rs_dedup() -> object:
        texts = [near_dedup.row_text(row) for row in rows]
        indices, pairs = hygiene.near_duplicates(texts, near_dedup.DEFAULT_THRESHOLD)
        return indices, pairs

    same = py_dedup() == rs_dedup()  # indices, pairs and similarities, exactly
    table.append(
        (
            f"near-dedup: {len(rows)} rows (Rust column includes row_text)",
            best_of(args.repeat, py_dedup),
            best_of(args.repeat, rs_dedup),
            same,
            False,
        )
    )

    print(f"\nbest of {args.repeat}, scale {args.scale}, worst-case shapes of {size} characters")
    print(f"{'case':<62}{'python s':>10}{'rust s':>10}{'ratio':>8}  agree")
    for name, py, rs, same, _ in table:
        print(f"{name:<62}{py:>10.4f}{rs:>10.4f}{py / rs:>7.1f}x  {'yes' if same else 'NO'}")
    worst_ratios = [(py / rs, name) for name, py, rs, _, worst in table if worst]
    low, low_name = min(worst_ratios)
    print(f"\nminimum worst-case ratio: {low:.1f}x ({low_name})")
    print(f"load average at end: {load()}")
    return 0 if all(row[3] for row in table) else 1


if __name__ == "__main__":
    sys.exit(main())
