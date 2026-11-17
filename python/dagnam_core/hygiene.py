"""Data-hygiene bindings."""

from dagnam_core._native import (
    CORE_API,
    MAX_COMPARISONS,
    MAX_PAIRS,
    MAX_TEXT_CHARS,
    MAX_TEXTS,
    MAX_TOTAL_CHARS,
    UNICODE_SUPPORTED,
    UNICODE_VERSION,
    NearDedupLimitError,
    findings,
    minhash_signature,
    near_duplicates,
)

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
