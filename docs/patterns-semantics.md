# Pattern semantics: Python versus the `regex` crate

Companion to [patterns.md](patterns.md), which holds the per-pattern entries ("semantics N" there
means item N below). In both files `\d`, `\w`, `\s`, `\b` and `[a-z]` under `(?i)` always mean
**Python's** meaning; this file says how the port spells each of them.

## Reference interpreter and tables

The reference is **Python 3.13.14, Unicode 15.1.0**: the interpreter that wrote the corpus, and
the version the corpus tests read under. Python's own classes follow the interpreter's Unicode
database, so the core ships the tables of **all three** versions CPython 3.12 (15.0.0), 3.13
(15.1.0) and 3.14 (16.0.0, free-threaded 3.14t too) carry, and every function takes the
`UnicodeVersion` to read under (the Python package selects it from `unicodedata.unidata_version`
at import; see [divergences.md](divergences.md)). The port does **not** use the `regex` crate's
built-in `\w`, `\s`, `\d`, `\b` or its case-folding for the cases below; it supplies its own
tables, **generated from the interpreter that has each version** by the checked-in script
`scripts/gen_py_classes.py`. The script computes each class with `re.fullmatch` over every code
point (surrogates skipped), checks every code point against the definition in items 1 to 3
(`str.isalnum()` or `_`, `str.isspace()`, category `Nd`), refuses any interpreter whose Unicode
database is none of the three, and writes `crates/hygiene/data/py_classes_<major>_<minor>.txt`
(`py_classes_15_0.txt`, `py_classes_15_1.txt`, `py_classes_16_0.txt`): one `[word]`, `[space]` and
`[digit]` section of inclusive hex ranges (`lo-hi`, lowercase, many to a line, so the 500-line cap
holds). `scripts/gen_py_classes.py --check`, run under each of the interpreters (`uv run --python
3.12|3.13|3.14|3.14t`), fails when its file, or its line of `corpus/classes.jsonl`, is not what
the interpreter writes. The gate (`scripts/check.sh`) runs it under its 3.12 venv, so it checks the
15.0.0 table; it does not fetch 3.13 or 3.14 (it must work offline). CI's wheel job runs it under
3.12, 3.13 and 3.14, and the release workflow under 3.12 and 3.14t, so the 15.1.0 and 16.0.0 tables
are checked there and by hand. A table is added when a new CPython brings a new Unicode version.

The file is parsed by the compiler (`crates/hygiene/src/table.rs`, a `const fn` run in a `static`
initialiser): a malformed table (missing, empty or repeated section, a token that is not
`lo-hi`, an unsorted or overlapping range, a surrogate) is a **build error** on every platform,
never an empty class at run time. It reads LF and CRLF alike, and `.gitattributes` pins the file
to LF.

`crates/hygiene/src/unicode.rs` builds everything from those ranges:

- the predicates of item 11, named `is_py_word(version, c)`, `is_py_space` and `is_py_digit`;
- the class spellings `word_class(version)`, `space_class` and `digit_class`, strings (formatted
  from the compile-time table on first use, one per version) of the form
  `[\u{30}-\u{39}...]`, bracketed classes of explicit ranges; and `case_sensitive(class)`
  (`text.rs`), which wraps one in `(?-i:...)`.

Every pattern that holds a class is compiled once per version, on first use of that version.
Near-duplicate detection (`near_duplicates`, `minhash_signature`) splits on the space only and
reads no class, so it takes no version.

Bracket-body spellings, as valid nested `regex` syntax (W is a class above, with its brackets):
`\W` is `[^W]`; `[^\W_]` (letters and numbers, no underscore) is `[W&&[^_]]`; a class of W and
other members, such as Python's `[\w.+-]`, is a union `[W.+-]` outside `(?i)` and the
alternation `(?:(?-i:W)|[.+-])` under it. The `(?-i:...)` wrapper goes round a whole standalone
class, never inside a bracket class, where it is not a group.

The case-fold additions of item 6 are not part of the table.

Why not the crate: measured against the crate's tables (`regex` 1.13.1, `regex-syntax` 0.8.11,
Unicode 16), 4,382 code points differ in `\w` and 80 in `\d` from Python 3.13; 5,004 and 80 from
Python 3.12 (Unicode 15.0); none from Python 3.14 (Unicode 16.0). (Re-measured on the Python
side: 3.13 against 3.12 differ in 622 `\w` code points, U+2EBF0 to U+2EE5D, and no `\d`; 3.13
against 3.14 in 4,382 `\w` and 80 `\d`.) `\s` is the same 29 code points on all three
interpreters. The crate's own `\w` also differs in kind (item 1), and its `\s` lacks U+001C to
U+001F.

The package's `requires-python` is `>=3.12`, so the Python backend itself answers with whatever
interpreter runs it. The core therefore reads under the tables of that interpreter's own Unicode
version (the three above), selected when the package is imported, so the two backends agree on
every code point. An interpreter with another version is refused, not served with the wrong
tables (see `divergences.md`).

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
    A repetition of the **word** class is a different matter: a hypothetical
    `[W.-]{1,64}@[W.-]{1,255}` needs about 14.8 MB and is refused by the 10 MiB default
    `size_limit` (measured in the review of the table). Later tasks must not repeat the full word
    class a bounded number of times: use a single linear scan, or a smaller class, or measure the
    compiled pattern first.
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
