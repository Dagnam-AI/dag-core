# Pattern semantics: Python versus the `regex` crate

Companion to [patterns.md](patterns.md), which holds the per-pattern entries ("semantics N" there
means item N below). In both files `\d`, `\w`, `\s`, `\b` and `[a-z]` under `(?i)` always mean
**Python's** meaning; this file says how the port spells each of them.

## Reference interpreter and tables

The reference is **Python 3.13.14, Unicode 15.1.0**: the interpreter that wrote the corpus. The
port does **not** use the `regex` crate's built-in `\w`, `\s`, `\d`, `\b` or its case-folding for
the cases below; it supplies its own tables, **generated from the reference interpreter's
`unicodedata`** by a checked-in generator script (to be written; this section is its
specification). The generator emits, as Rust source, sorted code point ranges for:

- the word set (the `\w` of item 1), the space set (item 2) and the digit set (item 3), with the
  class spellings the patterns need (`\w`, `\W`, `[^\W_]`, `\s`, `\S`, `\d`, `\D`) and the three
  predicates of item 11;
- the case-fold additions of item 6.

It runs under the pinned interpreter and checks every code point against that interpreter's `re`
before it writes anything. The tables are regenerated when the pinned interpreter moves.

Why not the crate: measured against the crate's tables (`regex` 1.13.1, `regex-syntax` 0.8.11,
Unicode 16), 4,382 code points differ in `\w` and 80 in `\d` from Python 3.13; 5,004 and 80 from
Python 3.12 (Unicode 15.0); none from Python 3.14 (Unicode 16.0). (Re-measured on the Python
side: 3.13 against 3.12 differ in 622 `\w` code points, U+2EBF0 to U+2EE5D, and no `\d`; 3.13
against 3.14 in 4,382 `\w` and 80 `\d`.) `\s` is the same 29 code points on all three
interpreters. The crate's own `\w` also differs in kind (item 1), and its `\s` lacks U+001C to
U+001F.

The package's `requires-python` is `>=3.12`, so the Python backend itself answers with whatever
interpreter runs it. The core is pinned to the reference tables; where an interpreter differs
from them, the two backends differ on those code points. That is a documented divergence (see
`divergences.md`, the row for a code point whose `\w` or `\d` status differs between interpreters), not a bug.

## Items

1. `\w` is `[\p{L}\p{N}_]` (letters, any number category, underscore), in Python terms
   `str.isalnum()` or `_`. Not Rust's `\w` (UTS #18: alphabetic, marks, decimal digits,
   connector punctuation), which for example adds combining marks such as U+0345 and drops
   superscript digits. `\W` is its complement; `[^\W_]` is `[\p{L}\p{N}]`. Verified for every code
   point against `re` and against `unicodedata` on 3.12, 3.13 and 3.14 (each against its own
   tables): no difference. The set is **generated** (above), never the crate's `\p{L}`, `\p{N}`
   or `\w`.
2. `\s` is Unicode `White_Space` **plus U+001C to U+001F**, 29 code points: U+0009 to U+000D,
   U+0020, U+0085, U+00A0, U+1680, U+2000 to U+200A, U+2028, U+2029, U+202F, U+205F, U+3000, and
   U+001C to U+001F. Python's `str.isspace()` is the same set. Verified for every code point.
3. `\d` is general category `Nd` (Arabic-Indic, fullwidth and other digits match), `\D` its
   complement. Where a guard names ASCII text (P31) compare characters, never digit values.
   (If Luhn or Verhoeff are ported, `int(char)` accepts every `Nd` digit: carry the digit value in
   the generated table.)
4. `\b` occurs only in P24; see there. Never the crate's `\b`.
5. `$` matches at the end **and before a final newline**; Rust's does not. `\Z` is `\z`. P17
   needs `\n?\z`; P08 and P24 are safe with `\z`.
6. `(?i)`. Python's case-insensitive match adds, to the ASCII letters, **U+0130 and U+0131 for
   `i`, U+212A for `k` and U+017F for `s`** (scanned over every code point up to U+2FFFF, on 3.12
   and 3.13). The `regex` crate already folds `k` with U+212A and `s` with U+017F; it does **not**
   match U+0130 or U+0131 for `i` (confirmed with `regex` 1.13.1). Add U+0130 and U+0131 by hand
   wherever an `i` (a literal, or inside `[A-Za-z]`, `[a-z0-9]` and similar) sits under `(?i)` or
   `(?i:...)`: P09, P11, P12, P13, P15, P24, P25, P27, in body classes as well as guards. The
   addition applies to **negated** classes too: P11's terminator is `[^A-Za-z0-9+/=\u{130}\u{131}]`, so
   those two do not end the match. The same set applies to the code checks
   that stand in for a lookaround in a `(?i)` pattern (P09, P11, P25: ASCII letters and digits,
   U+0130, U+0131, U+017F, U+212A). A corpus case pins each of the four.
   **`\w` and `\W` under `(?i)`** (P24 is the only pattern that has them; checked on every
   compiled pattern): Python's `\w` ignores the flag, the crate's case-folds a written-out
   `[\p{L}\p{N}_]` and pulls in code points whose case-fold partner is a letter (for example U+0345, and U+A7CB and U+A7DC from the crate's Unicode 16 tables). Spell every `\w`, `\W` and
   `[\w...]` in P24 as the generated class wrapped in `(?-i:...)`; a class that mixes `\w` with
   other members becomes an alternation (`(?:(?-i:W)|[.+-])`). Inputs that fail otherwise:
   `U+0345` then `v` (Python: boundary before `v`, so P24 matches; unwrapped Rust: no match) and
   a space, `a`, `U+0345`, `/` (Python: no match, since U+0345 is not `\w`; unwrapped Rust:
   match). `\s` and `\d` under `(?i)` (P09, P11, P24, P25, P27, P28) are unaffected: no `White_Space`
   or `Nd` code point has a case mapping (checked on 3.13 and 3.14).
7. Offsets: Python reports code points, `regex` bytes; convert at the binding. "One character"
   in a restart rule is one scalar. **Lone surrogates:** a Python `str` may hold U+D800 to U+DFFF,
   a Rust `&str` cannot. The binding maps each lone surrogate to U+FFFD, one for one, so
   code-point offsets are unchanged. Neither is `\w`, `\s` or `\d` in either engine, and both
   are plain characters to a negated class. The Python side uses the same mapping when it hands
   text to Rust **and** when it compares results (the reference run in a differential test reads
   the mapped text), so both sides read one string.
8. `search(text, pos)` / `finditer(text, pos)`: the text before `pos` stays visible to lookbehind
   (use `find_at`-style calls on the full haystack). `match(text, pos, endpos)` is anchored at
   `pos` and the text ends at `endpos` for every lookahead and `$` (P02, P15, P16): use an
   anchored search over `text[..endpos]`. Patterns that can match empty (P04, P06, P14) are used
   as "the run that begins at `start`", which is always a match at `start`.
9. Unicode version: see the first section. A different interpreter or a newer `regex` table
   changes the answer only for code points assigned between the versions; pin each range with a
   corpus case (for example U+2EBF0, U+2EE5D, U+1C89, U+10D40) and let the generator move with the
   pin.
10. Size: bounded repeats of `\p{Nd}` (P01, P02, P20) compile under the default size limit in
    `regex` 1.13.1 (measured with the crate's own `\p{Nd}`); the generated digit class is of the
    same size, so re-measure once with it and raise `size_limit` only if it is refused.
11. **Character tests in code** (the LEAD, TAIL and INNER checks, and anything else that asks "is
    this a word, space or digit") use one shared set of three predicates, built from the same
    generated tables as the class spellings, so the regex and the checks cannot disagree:
    `is_word(c)` is `\w` of item 1, `is_space(c)` is `\s` of item 2, `is_digit(c)` is `\d` of item
    3. Not `char::is_alphanumeric` (it counts `Other_Alphabetic` marks such as U+0345),
    `char::is_whitespace` (it lacks U+001C to U+001F) or `char::is_numeric` (it includes the `No`
    and `Nl` categories). ASCII-named guards (`[A-Za-z0-9]`, `[a-z0-9]`, `[A-Z]`) stay explicit
    ranges.
12. Lower-casing outside the patterns (`_snake` and the member-name rules call `.lower()`):
    Python's full Unicode lower-casing, for example U+0130 becomes two characters (`i` and
    U+0307), and the final-sigma rule. An ASCII loop is wrong; pin it with corpus cases.
