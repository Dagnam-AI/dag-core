"""Profile the Python reference's near-duplicate detection on a synthetic dataset.

Run from the `python/` folder of a dag-contracts checkout, in its venv:

    python -m cProfile -s tottime <path to this file>
"""

import random

from dagnam_contracts.hygiene.near_dedup import compute_near_duplicates

WORDS_PER_ROW = 40
VOCABULARY = 5000


def near_dedup_rows(n: int = 5000) -> int:
    """Cluster `n` rows of 40 random words; one row in ten copies an earlier one with a word changed."""
    rng = random.Random(1)
    rows: list[dict[str, str]] = []
    for i in range(n):
        if i and i % 10 == 0:
            words = rng.choice(rows)["t"].split()
            words[rng.randrange(WORDS_PER_ROW)] = f"w{rng.randrange(VOCABULARY)}"
        else:
            words = [f"w{rng.randrange(VOCABULARY)}" for _ in range(WORDS_PER_ROW)]
        rows.append({"t": " ".join(words)})
    return len(compute_near_duplicates(rows).duplicate_indices)


if __name__ == "__main__":
    print(near_dedup_rows())
