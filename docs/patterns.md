# Pattern routes

Every `re.compile` call in the hygiene reference code, with the route that ports it to the
`regex` crate. Reference code: `python/dagnam_contracts/hygiene/{contact,credentials,detectors,spans,near_dedup}.py`
in `dag-contracts` (`main` at `80177e1`), read under Python 3.13.14 (Unicode 15.1.0), the interpreter that wrote the corpus. All paths
below are relative to that repository. Python 3 `re` on `str` is the reference; the routes are
**1** `regex` with pattern rewrites only (no code check), **2** `regex` plus a named check, **3** `fancy-regex` with a step limit.

## Count and routes

34 `re.compile` calls, found by walking the syntax tree of the five files (contact 4,
credentials 15, detectors 12, spans 2, near_dedup 1). 33 are bound to module-level names (four of
them in the tuple `_SHAPES`); one is an argument inside the detector table (`detectors.py:381`).
No pattern is compiled in a loop or a function. Twelve are built from fragments or at import
(P07, P10, P12 to P15, P17, P25, P27, P28, P30, P32); the entries give each pattern as compiled.

| route | rows |
|---|---|
| 1, `regex`, pattern rewrites only | 11 |
| 2, `regex` plus a check | 23 |
| 3, `fancy-regex` | **0** |

Route 1 rows need no check in code, only a rewritten spelling: P08 `\Z` to `\z`; P17 and P24 `$`
(and P24 `\b`); P15 and P24 case-fold additions; P04, P06, P14, P33 and P34 the Python spelling
of `\w` / `\s` (generated classes). P16 and P32 need nothing beyond the generated `\d` and the
registry-built literal.

The 34 patterns hold 58 lookarounds (24 negative lookbehind, 3 positive lookbehind, 23 negative
lookahead, 8 positive lookahead) and 2 backreferences, counted on the compiled patterns. A
count of 59 quoted elsewhere is not reproduced: the compiled patterns hold 58 (60 with the two in
`_snake`, below), so the entries here are the count of record.

Not among the 34: eleven calls of `re.sub/search/split/findall` with a literal pattern (listed at
the end) and two other files of the package (`trace/envelope.py`, `prompts.py`), which are out of
scope. `rows.py`, `pii.py`, `dedup.py` and `contamination.py` use no regex.

Since `80177e1` the reference (`8ee5f42`) added one compiled pattern, `_OUTSIDE_ENTROPY` in
`credentials.py`, the class `[^A-Za-z0-9._~+/=-]` that its `_entropy_starts` scans for. It is not
ported as a pattern: `entropy_starts` (`crates/hygiene/src/credentials/shapes.rs`) reads the same
set in code (`is_entropy_char`), from the end of the value, and its tests check it against
`is_high_entropy` (P18) on every suffix. `_HIGH_ENTROPY` is now spelled from the same class string
and matches the same text. No other pattern changed; the line numbers in this file are those of
`80177e1`.

## How a route 2 check is applied

Every route 2 entry uses one or more of these. The body is the pattern with its lookarounds
removed; the checks look at the **full haystack** (never a slice that starts at the search
position), one Unicode scalar at a time, and test word, space and digit characters with the shared
predicates of semantics 11 (never `char::is_alphanumeric`, `is_whitespace` or `is_numeric`). A negative lookbehind at the very start passes.

- **LEAD guard** (negative lookbehind before the match). Run the body from the current position,
  take the leftmost match at `s`, test the characters before `s`. On failure restart the search
  at `s` plus one character (a scalar, not a byte), never at the match end. This equals Python
  because the body does not depend on the guard.
- **TAIL guard** (negative lookahead after the match). Testing the single match the engine
  returns is **not** equivalent when the body can end in several places: Python backtracks to a
  shorter end the guard accepts. In random comparison the post-check disagreed with Python for
  P01, P02, P20, P21, P25 and P30 (for example a 14-digit run with a trailing hyphen, which
  Python reports as its first 13 digits). Instead append a **consuming terminator**, a group that
  matches exactly what the guard allows (end of text, or a character outside the forbidden set,
  written out per entry), keep the body in a capture group, report that group, and resume the
  search at its end (the terminator is looked at, not consumed). Leftmost-first priority in
  `regex` then picks the path Python's backtracking picks. Entries marked "fixed end" have a
  body that can end in one place only, so a post-check is also exact.
- **INNER guard** (a lookahead inside the match, at a position the text already fixes): check the
  characters after matching; failure means no match at `s`, restart at `s` plus one character.
- **Backreference**: expanded into its possible values (an alternation); the alternatives exclude
  each other on the text, so their order does not matter.

Each route 2 description was compared with the Python original on tens of thousands of random
inputs with no difference (terminator form for every TAIL guard); the corpus remains the proof.

## Semantics

How `\d`, `\w`, `\s`, `\b`, `$`, `(?i)` and offsets are spelled in Rust, the reference
interpreter and its generated tables (one per Unicode version), and the character predicates the code checks use are in
[patterns-semantics.md](patterns-semantics.md). "Semantics N" in the entries is item N there.

## Fragments

Quoted from the reference code; entries below refer to them by name. Where the source writes `\"` inside a raw string, the regex means `"`, and this document writes `"`.

```
_NAMES          api[_-]?key|access[_-]?key|private[_-]?key|client[_-]?secret|secret(?:[_-]?access)?(?:[_-]?key)?|password|passwd|authorization|token|auth|key
_NAME_START     (?:(?<![A-Za-z0-9])|(?<=[a-z0-9])(?=[A-Z]))
_VALUE_CHAR     [^\s"'`,;&]
_NAME_TO_NUMBER \s*(?:[:#]\s*)?
_DAY            (?:0?[1-9]|[12]\d|3[01])
_MON            (?:0?[1-9]|1[0-2])
_MONTH          (?:jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|june?|july?|aug(?:ust)?|sep(?:t(?:ember)?)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\.?
_DATE           (?:{_DAY}[/.-]{_MON}|{_MON}[/.-]{_DAY})[/.-](?:19|20)?\d{2}
                |(?:19|20)\d{2}[/.-]{_MON}[/.-]{_DAY}
                |{_DAY}(?:st|nd|rd|th)?\s+{_MONTH},?\s+(?:19|20)\d{2}
                |{_MONTH}\s+{_DAY}(?:st|nd|rd|th)?,?\s+(?:19|20)\d{2}      (one pattern, four lines here)
_EU_VAT_FORMATS (state prefix: format after it, in the order of the pattern)
  AT U\d{8}          BE [01]\d{9}        BG \d{9,10}        CY \d{8}[A-Z]       CZ \d{8,10}
  DE \d{9}           DK \d{8}            EE \d{9}           EL \d{9}            ES [A-Z0-9]\d{7}[A-Z0-9]
  FI \d{8}           FR [A-HJ-NP-Z0-9]{2}\d{9}              HR \d{11}           HU \d{8}
  IE \d{7}[A-W][A-I]?|\d[A-Z+*]\d{5}[A-W]                   IT \d{11}           LT \d{9}|\d{12}
  LU \d{8}           LV \d{11}           MT \d{8}           NL \d{9}B\d{2}      PL \d{10}
  PT \d{9}           RO [1-9]\d{5,9}     SE \d{10}01        SI \d{8}            SK \d{10}
```

`_NAME_START` reads: the character before is not an ASCII letter or digit, **or** it is `[a-z0-9]`
and the first character of the match is `[A-Z]` (a camelCase hump). It is the LEAD guard of
P10, P12 and P13 and needs the first character of the match.

## Index

| # | name | where | route |
|---|---|---|---|
| P01 | `_PHONE_CANDIDATE` | `contact.py:20` | 2 |
| P02 | `_PHONE_NEXT` | `contact.py:40` | 2 |
| P03 | `_EMAIL_HEAD` | `contact.py:74` | 2 |
| P04 | `_DOMAIN_RUN` | `contact.py:75` | 1 |
| P05 | `_URL_HEAD` | `credentials.py:130` | 2 |
| P06 | `_PLAIN_RUN` | `credentials.py:131` | 1 |
| P07 | `_SHAPES[0]` provider keys | `credentials.py:203` | 2 |
| P08 | `_SHAPES[1]` PEM block | `credentials.py:222` | 1 |
| P09 | `_SHAPES[2]` bearer | `credentials.py:226` | 2 |
| P10 | `_SHAPES[3]` connection `pwd=` | `credentials.py:229` | 2 |
| P11 | `_BASIC` | `credentials.py:234` | 2 |
| P12 | `_QUOTED_ASSIGNMENT` | `credentials.py:239` | 2 |
| P13 | `_ASSIGNMENT_HEAD` | `credentials.py:249` | 2 |
| P14 | `_VALUE_RUN` | `credentials.py:250` | 1 |
| P15 | `_NEXT_ASSIGNMENT` | `credentials.py:251` | 1 |
| P16 | `_DIGIT` | `credentials.py:252` | 1 |
| P17 | `_MEMBER_NAME` | `credentials.py:278` | 1 |
| P18 | `_HIGH_ENTROPY` | `credentials.py:281` | 2 |
| P19 | `_VOUCHED_KEY` | `credentials.py:283` | 2 |
| P20 | `_CARD_CANDIDATE` | `detectors.py:68` | 2 |
| P21 | `_IBAN_CANDIDATE` | `detectors.py:151` | 2 |
| P22 | `_IPV4_CANDIDATE` | `detectors.py:174` | 2 |
| P23 | `_IPV6_CANDIDATE` | `detectors.py:175` | 2 |
| P24 | `_VERSION_BEFORE` | `detectors.py:183` | 1 |
| P25 | `_DATE_OF_BIRTH` | `detectors.py:239` | 2 |
| P26 | `_UK_NINO` | `detectors.py:244` | 2 |
| P27 | `_CA_SIN` | `detectors.py:258` | 2 |
| P28 | `_IN_AADHAAR` | `detectors.py:298` | 2 |
| P29 | `_IN_PAN` | `detectors.py:323` | 2 |
| P30 | `_EU_VAT` | `detectors.py:356` | 2 |
| P31 | national id (US SSN), inline | `detectors.py:381` | 2 |
| P32 | `PLACEHOLDER` | `spans.py:46` | 1 |
| P33 | `_CAN_HOLD_A_FINDING` | `spans.py:71` | 1 |
| P34 | `_PUNCTUATION` (not ported, stays in Python) | `near_dedup.py:63` | 1 |

## contact.py

**P01 `_PHONE_CANDIDATE`** (`contact.py:20`, no flags). Route 2. Classes: `\d`, `\s`.
```
(?<![\d+])(?<![\d][ -])\+?\d(?:[\d\s.()-]{7,20})\d(?![\d]|[ -]\d)
```
- Lookarounds: negative lookbehind `(?<![\d+])`; negative lookbehind `(?<![\d][ -])`; negative lookahead `(?![\d]|[ -]\d)`. No backreference.
- Body: `\+?\d(?:[\d\s.()-]{7,20})\d`.
- LEAD: the character before `s` is not a digit and not `+`, and it is not the case that the character two before is a digit while the one before is a space or hyphen (fewer than two characters before: that half passes).
- TAIL (retry needed): the next character is not a digit, and not a space or hyphen followed by a digit. Terminator: `(?:\z|[^\d\- ]|[ -](?:\z|\D))`.

**P02 `_PHONE_NEXT`** (`contact.py:40`, no flags). Route 2. Classes: `\d`, `\s`.
```
[ -]?(?P<s>\+?\d(?:[\d\s.()-]{7,20})\d(?![\d]|[ -]\d))
```
- Lookarounds: one negative lookahead, as in P01. No lookbehind, no backreference.
- Called as `.match(text, position, stop)`: anchored at `position`, the text ends at `stop` (semantics 8). The leading `[ -]?` is outside the group and is taken when present.
- TAIL (retry needed): P01's terminator, after group `s`; report the span of `s`.

**P03 `_EMAIL_HEAD`** (`contact.py:74`, no flags). Route 2. Classes: `\w`.
```
(?<![\w.+-])[\w.+-]+@
```
- Lookarounds: one negative lookbehind. No tail guard: the run excludes `@`, so the body ends at the first `@` after it.
- LEAD: the character before `s` is not `\w`, `.`, `+` or `-`. The caller searches again from the middle of a run after a domain ends at `..`, so the guard does fail in practice; every position inside a run of those characters fails it, so a failed guard may skip to the end of the run (optional, linear time).

**P04 `_DOMAIN_RUN`** (`contact.py:75`, no flags). Route 1 (generated `\w`). Classes: `\w`.
```
[\w.-]*
```
Used as the (possibly empty) run that begins at `start`.

## credentials.py

**P05 `_URL_HEAD`** (`credentials.py:130`, no flags). Route 2. Classes: `\s`, `\d`.
```
(?<!\s)://[^\s/?#@:"'`<>]*:(?!\d{1,5}[,;|&>])
```
- Lookarounds: negative lookbehind `(?<!\s)`; negative lookahead `(?!\d{1,5}[,;|&>])`.
- LEAD: the character before `s` (the `:` of `://`) is not whitespace.
- TAIL (fixed end: the run excludes `:`, so the first `:` after `://` ends the body): let `r` be the number of consecutive digits right after the final `:`. The guard fails exactly when `1 <= r <= 5` and the character after those digits is one of `,` `;` `|` `&` `>`. Passes at end of text. The caller uses the end of the match (after the colon). Called as `.search(text, position)`.

**P06 `_PLAIN_RUN`** (`credentials.py:131`, no flags). Route 1 (generated `\s`). Classes: `\s`.
```
[^\s/?#@"'`<>]*
```
The run that begins at a position (may be empty).

**P07 `_SHAPES[0]`, provider keys** (`credentials.py:203`, no flags; group `s` is the span). Route 2.
Classes: `\d` in one lookahead.
```
(?<![A-Za-z0-9_-])(?P<s>sk-(?:[A-Za-z0-9_-]*-)?(?:(?=[A-Za-z_]*\d)|(?=[a-z0-9_]*[A-Z])(?=[A-Z0-9_]*[a-z]))[A-Za-z0-9_]{20,}[A-Za-z0-9_-]*|[rs]k_live_[A-Za-z0-9]{16,}|(?:AKIA|ASIA)[A-Z0-9]{16}(?![A-Za-z0-9])|gh[pousr]_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{22,}|(?:xoxe\.)?xox[abepr]-[A-Za-z0-9-]{10,}|AIza[A-Za-z0-9_-]{35}(?![A-Za-z0-9_-])|hf_[A-Za-z0-9]{30,}|glpat-[A-Za-z0-9_-]{20,}|eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]*)
```
- Lookarounds (6): negative lookbehind before the whole alternation; in the first alternative three positive lookaheads `(?=[A-Za-z_]*\d)`, `(?=[a-z0-9_]*[A-Z])`, `(?=[A-Z0-9_]*[a-z])`; negative lookaheads `(?![A-Za-z0-9])` (key-id alternative) and `(?![A-Za-z0-9_-])` (the `AIza` alternative).
- The alternatives start with different literals, so at one start only one can apply; one regex with the alternatives below, then per-alternative checks.
- LEAD (all alternatives): the character before `s` is not `[A-Za-z0-9_-]`.
- First alternative, regex `sk-[A-Za-z0-9_-]*` (the whole run `R` after `sk-`). Check: split `R` at every `-` into segments and look at them **from the last to the first**; accept at the first segment `S` with at least 20 characters (all in `[A-Za-z0-9_]` by construction) such that (a) `S` contains an ASCII digit, or (b) `S` is the last segment and the character right after `R` is a `\d` (a non-ASCII decimal digit, since ASCII digits would be in `R`), or (c) `S` contains an ASCII uppercase and an ASCII lowercase letter. If one is accepted the match is the whole of `sk-` plus `R`; if none, no match at `s`, restart at `s` plus one character. (Python's lookaheads see only up to the next `-`, which is why the segment, not the run, is tested.)
- The `AKIA|ASIA` alternative (fixed length 20) and the `AIza` alternative (fixed length 39): post-check that the next character is not in the class named by the lookahead. Fixed end.
- All other alternatives: plain regex, no check.

**P08 `_SHAPES[1]`, PEM block** (`credentials.py:222`, flags `(?s)`). Route 1 (rewrite: `\Z`). Classes: none.
```
(?s)(?P<s>-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----.*?(?:-----END [A-Z0-9 ]*PRIVATE KEY-----|\Z))
```
No lookaround. Rewrite `\Z` as `\z` (semantics 5). Lazy `.*?` has the same meaning; without an END line the match runs to the end of the text.

**P09 `_SHAPES[2]`, bearer** (`credentials.py:226`, flags `(?i)`). Route 2. Classes: `\s`. The body classes `[a-z0-9._~+/-]` and `[a-z0-9]` take U+0130 and U+0131 (semantics 6).
```
(?i)(?<![a-z0-9])bearer\s+(?P<s>[a-z0-9._~+/-]{19,}[a-z0-9]=*)
```
- Lookarounds: one negative lookbehind. No tail guard.
- LEAD: the character before `s` is not in `[a-z0-9]` read case-insensitively: ASCII letters and digits, and U+0130, U+0131, U+017F, U+212A (semantics 6).

**P10 `_SHAPES[3]`, connection `pwd=`** (`credentials.py:229`, no flags). Route 2. Classes: `\s` (in `_VALUE_CHAR`).
```
(?<!\$)_NAME_START(?i:pwd)=["']?(?![/~.$%\\`]|[A-Za-z]:[\\/])(?P<s>_VALUE_CHAR+)
```
- Lookarounds (5): negative lookbehind `(?<!\$)`; the three of `_NAME_START` (negative lookbehind, positive lookbehind, positive lookahead); negative lookahead `(?![/~.$%\\`]|[A-Za-z]:[\\/])`.
- Body: `(?i:pwd)=["']?(?P<s>[^\s"'`,;&]+)`.
- LEAD: the character before the match is not `$`, and `_NAME_START` holds.
- INNER (fixed position: right after `=` and the optional quote, which the value class excludes, so skipping the quote cannot rescue a failure): the first character of the value is not one of `/ ~ . $ % \` or a backtick, and the value does not begin with an ASCII letter, `:` and then `\` or `/`. On failure no match at `s`.

**P11 `_BASIC`** (`credentials.py:234`, flags `(?i)`). Route 2. Classes: `\s`. The body class `[A-Za-z0-9+/]` and the word `basic` take U+0130 and U+0131 (semantics 6).
```
(?i)(?<![a-z0-9])basic\s+(?P<s>[A-Za-z0-9+/]{4,}={0,2})(?![A-Za-z0-9+/=])
```
- Lookarounds: one negative lookbehind, one negative lookahead.
- LEAD: as P09.
- TAIL: the next character is not in `[A-Za-z0-9+/=]` read case-insensitively (semantics 6). Terminator `(?:\z|[^A-Za-z0-9+/=\u{130}\u{131}])` under the same flag (the additions apply to the negated class too; U+212A and U+017F are folded by the crate). Fixed end in effect (a shorter end is always followed by a class character).

**P12 `_QUOTED_ASSIGNMENT`** (`credentials.py:239`, no flags; `(?i:...)` scoped to the name). Route 2. Classes: `\s`.
```
_NAME_START(?P<n>(?i:_NAMES))["']\s*[:=]\s*(?:"(?P<s>[^"\n]+)"|'(?P<t>[^'\n]+)')
```
- Lookarounds (3): those of `_NAME_START`. No backreference.
- LEAD: `_NAME_START`.

**P13 `_ASSIGNMENT_HEAD`** (`credentials.py:249`, no flags). Route 2. Classes: `\s`.
```
_NAME_START(?P<n>(?i:_NAMES))["']?\s*[:=]\s*["']?
```
- Lookarounds (3): those of `_NAME_START`.
- LEAD: `_NAME_START`. Called as `.search(text, position)` and the caller uses the group `n` and the match end.

**P14 `_VALUE_RUN`** (`credentials.py:250`, no flags). Route 1 (generated `\s`). Classes: `\s`.
```
[^\s"'`,;&]*
```

**P15 `_NEXT_ASSIGNMENT`** (`credentials.py:251`, no flags; `(?i:...)` scoped). Route 1 (rewrites: anchored call, case-fold additions of semantics 6 for the `i` in the names). Classes: `\s`.
```
:\s*(?i:_NAMES)["']?\s*[:=]
```
Called as `.match(text, colon)`: an anchored search at `colon`.

**P16 `_DIGIT`** (`credentials.py:252`, no flags). Route 1 (generated `\d`). Classes: `\d`.
```
\d
```
Called as `.search(text, start, end)`: the text ends at `end` (semantics 8).

**P17 `_MEMBER_NAME`** (`credentials.py:278`, no flags). Route 1 (rewrite: `$`). Classes: none (`$`, `^`).
```
(?:^|[^a-z0-9])(?P<n>_NAMES)$
```
No lookaround. `$` becomes `\n?\z` (semantics 5); the input is a lower-cased key. `^` is the start of the text.

**P18 `_HIGH_ENTROPY`** (`credentials.py:281`, no flags). Route 2. Classes: `\s`, `\d`.
```
(?=[^\s]*[A-Za-z])(?=[^\s]*\d)[A-Za-z0-9._~+/=-]{16,}
```
- Lookarounds: two positive lookaheads, both at position 0. Only used as `.fullmatch(value)`.
- Check: regex `\A[A-Za-z0-9._~+/=-]{16,}\z`, then the value contains an ASCII letter and an ASCII digit. (The whole value is made of class characters, so `[^\s]*` never leaves it and `\d` can only be `0-9`.) Compared with the original on 120,000 random values (ASCII, fullwidth and Arabic-Indic digits, spaces, newlines, combining marks): no difference.

**P19 `_VOUCHED_KEY`** (`credentials.py:283`, no flags). Route 2. Classes: none.
```
(?<![A-Za-z0-9_-])sk-(?:[A-Za-z0-9_-]*-)?[A-Za-z0-9_]{20,}
```
- Lookarounds: one negative lookbehind. Only used as `.search(value) is not None`.
- LEAD: the character before the match is not `[A-Za-z0-9_-]`; the regex itself is unchanged. (Same as: after `sk-`, some `-` separated segment of the run has 20 or more characters.)

## detectors.py

**P20 `_CARD_CANDIDATE`** (`detectors.py:68`, no flags). Route 2. Classes: `\d`.
```
(?<![\d-])(?:\d[ -]?){12,18}\d(?![\d-])
```
- Lookarounds: negative lookbehind `(?<![\d-])`, negative lookahead `(?![\d-])`.
- Body: 13 to 19 digits, adjacent digits separated by at most one space or hyphen.
- LEAD: the character before `s` is not a digit and not `-`.
- TAIL (retry needed): terminator `(?:\z|[^\d-])`. Retry is real: a run of digits ending in a space-separated tail lets a shorter end through where the longest meets a hyphen.

**P21 `_IBAN_CANDIDATE`** (`detectors.py:151`, no flags). Route 2. Classes: `\d`.
```
(?<![A-Za-z0-9])[A-Z]{2}\d{2}(?: ?[A-Z0-9]{4}){2,7}(?: ?[A-Z0-9]{1,4})?(?![A-Za-z0-9])
```
- Lookarounds: negative lookbehind, negative lookahead.
- LEAD: the character before `s` is not an ASCII letter or digit.
- TAIL (retry needed): terminator `(?:\z|[^A-Za-z0-9])`; a shorter end followed by a space is accepted where the longest is followed by a letter or digit.

**P22 `_IPV4_CANDIDATE`** (`detectors.py:174`, no flags). Route 2. Classes: `\w`, `\d`.
```
(?<![\w.])(?:\d{1,3}\.){3}\d{1,3}(?![\w]|\.\w)
```
- Lookarounds: negative lookbehind, negative lookahead.
- LEAD: the character before `s` is not `\w` and not `.`.
- TAIL: the next character is not `\w`, and not `.` followed by `\w`. Terminator `(?:\z|[^\w.]|\.(?:\z|\W))`. Fixed end (a shorter end is followed by a digit).

**P23 `_IPV6_CANDIDATE`** (`detectors.py:175`, no flags). Route 2. Classes: `\w`, `\d`.
```
(?<![\w:.])(?:[0-9A-Fa-f]{0,4}:){2,7}(?:\d{1,3}(?:\.\d{1,3}){3}|[0-9A-Fa-f]{0,4})(?![\w:]|\.\w)
```
- Lookarounds: negative lookbehind, negative lookahead.
- LEAD: the character before `s` is not `\w`, `:` or `.`.
- TAIL: the next character is not `\w` and not `:`, and not `.` followed by `\w`. Terminator `(?:\z|[^\w:.]|\.(?:\z|\W))`. Use the terminator form (no counter-example to a post-check was found, none was proven impossible).

**P24 `_VERSION_BEFORE`** (`detectors.py:183`, flags `(?i)`). Route 1 (rewrites: `\b`, `$`, `\w` as a generated class inside `(?-i:...)`, case-fold additions). Classes: `\s`, `\w`, `\b`.
```
(?i)(?:[\s(;][A-Za-z][\w.+-]*/|\bv|\bver\.?|version|build|firmware|release)\s*$
```
No lookaround. Used as `.search(before)` for truthiness only. Python's `\b` before the word character `v` means "start, or the previous character is not `\w`": write `\bv` as `(?:^|(?-i:NW))v` with `NW` the generated `\W` class (semantics 1), and the same for `\bver`; consuming that character is harmless for an existence test. `[\w.+-]*` becomes `(?:(?-i:W)|[.+-])*` with `W` the generated `\w` class. **Every `\w` and `\W` here must sit inside `(?-i:...)`**: under `(?i)` the crate case-folds a written-out `[\p{L}\p{N}_]` and pulls in U+0345, which Python's `\w` never matches. Failing inputs without the wrapper: U+0345 followed by `v` (Python matches, Rust does not) and a space, `a`, U+0345, `/` (Python does not match, Rust does). P24 is the only pattern with `\w`, `\W` or `\b` under `(?i)` (checked on every compiled pattern; `\s` and `\d` there are unaffected, see semantics 6). `$` as `\z` is safe because `\s*` already takes a final newline. Case-insensitivity: the `i` in `version`, `build`, `firmware` and `[A-Za-z]` take the additions of semantics 6.

**P25 `_DATE_OF_BIRTH`** (`detectors.py:239`, flags `(?i)`). Route 2. Classes: `\s`, `\d` throughout.
```
(?i)(?<![A-Za-z])(?:d\.?o\.?b\.?|date\s+of\s+birth|birth\s*date|birthday|born(?:\s+on)?)[\s:=,-]*(?:(?:is|was)\s+)?(?P<s>_DATE)(?![\d/.-]?\d)
```
- Lookarounds: negative lookbehind, negative lookahead. `_DATE` is a fragment.
- LEAD: the character before the match is not in `[A-Za-z]` read case-insensitively (semantics 6).
- TAIL (retry needed, and not a corner case: `_DAY` and `_MON` alternatives end at different places, so for a two-digit day the first alternative stops after one digit and only the later one passes): the next character is not a digit and not `/`, `.` or `-` followed by a digit. Terminator `(?:\z|[^\d/.-]|[/.-](?:\z|\D))` after group `s`.

**P26 `_UK_NINO`** (`detectors.py:244`, no flags). Route 2. Classes: `\d`.
```
(?<![A-Za-z0-9])(?P<s>(?!BG|GB|KN|NK|NT|TN|ZZ)[A-CEGHJ-PR-TW-Z][A-CEGHJ-NPR-TW-Z] ?\d{2} ?\d{2} ?\d{2} ?[A-D])(?![A-Za-z0-9])
```
- Lookarounds (3): negative lookbehind; negative lookahead at the start of `s`; negative lookahead at the end.
- Body: `[A-CEGHJ-PR-TW-Z][A-CEGHJ-NPR-TW-Z] ?\d{2} ?\d{2} ?\d{2} ?[A-D]`.
- LEAD: the character before `s` is not an ASCII letter or digit.
- INNER: the first two characters of `s` are not one of `BG GB KN NK NT TN ZZ` (case-sensitive).
- TAIL (fixed end): the next character is not an ASCII letter or digit.

**P27 `_CA_SIN`** (`detectors.py:258`, no flags; `(?i:...)` scoped). Route 2. Classes: `\s`, `\d`.
```
(?<![A-Za-z])(?:SIN|NAS|(?i:social\s+insurance|num[ée]ro\s+d['’]assurance\s+sociale))(?:\s+(?i:number|no\.?|#))?_NAME_TO_NUMBER(?P<s>[1-79]\d{2}(?P<sep>[ -]?)\d{3}(?P=sep)\d{3})(?![\d-])
```
- Lookarounds: negative lookbehind, negative lookahead. **Backreference** `(?P=sep)`.
- Backreference: `s` becomes `[1-79]\d{2}(?:\d{3}\d{3}| \d{3} \d{3}|-\d{3}-\d{3})` (no separator, a space, or a hyphen).
- LEAD: the character before the match is not an ASCII letter (the `(?i:...)` groups do not widen this test).
- TAIL (fixed end): the next character is not a digit and not `-`.

**P28 `_IN_AADHAAR`** (`detectors.py:298`, no flags; `(?i:...)` scoped). Route 2. Classes: `\d`, `\s`.
```
(?<![\d-])(?P<s>[2-9]\d{3}(?P<sep>[ -])\d{4}(?P=sep)\d{4})(?![\d-])|(?<![A-Za-z])(?i:aadhaa?r(?:\s+(?:no\.?|number))?)_NAME_TO_NUMBER(?P<bare>[2-9]\d{11})(?!\d)
```
- Lookarounds (4): each of the two alternatives has a negative lookbehind and a negative lookahead. **Backreference** `(?P=sep)` (first alternative).
- Backreference: first alternative's `s` becomes `[2-9]\d{3}(?: \d{4} \d{4}|-\d{4}-\d{4})`.
- LEAD, by the alternative that matched (group `s` or `bare`; the alternatives start with a digit and a letter, so one cannot rescue the other): alternative 1, the character before is not a digit and not `-`; alternative 2, it is not an ASCII letter.
- TAIL (fixed end), same alternative: 1, not a digit and not `-`; 2, not a digit. The reported span is the group that matched.

**P29 `_IN_PAN`** (`detectors.py:323`, no flags). Route 2. Classes: `\d`.
```
(?<![A-Za-z0-9])[A-Z]{3}[ABCFGHJLPT][A-Z]\d{4}[A-Z](?![A-Za-z0-9])
```
- Lookarounds: negative lookbehind, negative lookahead. LEAD and TAIL (fixed end): the character before and the character after are not ASCII letters or digits.

**P30 `_EU_VAT`** (`detectors.py:356`, no flags). Route 2. Classes: `\d` throughout.
```
(?<![A-Za-z0-9])(?:<state> ?(?:<format>)|...)(?![A-Za-z0-9])
```
One alternative per entry of `_EU_VAT_FORMATS`, in that order (see Fragments).
- Lookarounds: negative lookbehind, negative lookahead.
- LEAD: the character before `s` is not an ASCII letter or digit.
- TAIL (retry needed: `LT` is `\d{9}|\d{12}`, and the first alternative must give way): terminator `(?:\z|[^A-Za-z0-9])`.

**P31 national id (US SSN)** (`detectors.py:381`, anonymous argument; no flags). Route 2. Classes: `\d`.
```
(?<![\d-])(?!000|666|9\d\d)\d{3}-(?!00)\d{2}-(?!0000)\d{4}(?![\d-])
```
- Lookarounds (5): negative lookbehind; negative lookaheads `(?!000|666|9\d\d)`, `(?!00)`, `(?!0000)`; negative lookahead `(?![\d-])`.
- Body: `\d{3}-\d{2}-\d{4}`.
- LEAD: the character before `s` is not a digit and not `-`.
- INNER (fixed positions): the first three characters are not the ASCII text `000` or `666` and do not start with the ASCII `9`; the next two are not `00`; the last four are not `0000`. Compare characters, so a non-ASCII zero digit passes (as in Python).
- TAIL (fixed end): the next character is not a digit and not `-`.

## spans.py and near_dedup.py

**P32 `PLACEHOLDER`** (`spans.py:46`, no flags). Route 1 (built from the registry). Classes: none.
```
\[REDACTED:PII_[A-Z0-9_]+\]|<SECRET>
```
Built at import: `re.escape` of the redaction template `[REDACTED:{code}]` with the escaped `{code}` replaced by `PII_[A-Z0-9_]+`, joined with `|` to the escaped placeholder of every detector that has one (today only `<SECRET>`). Build it from the detector table in Rust with `regex::escape`; do not paste the literal.

**P33 `_CAN_HOLD_A_FINDING`** (`spans.py:71`, no flags). Route 1 (generated class). Classes: `\W` inside a class.
```
[^\W_]|@
```
Used as a boolean `.search`. Spelled as the generated word class without `_`, plus `@` (semantics 1; in set terms `[\p{L}\p{N}]|@`, but never the crate's own `\p{L}`).

**P34 `_PUNCTUATION`** (`near_dedup.py:63`, flags `re.UNICODE`, which is the default for `str`). Route 1 (generated classes). Classes: `\w`, `\s`. Not ported: stays in Python.
```
[^\w\s]
```
If it is ever ported: the complement of the generated word and space classes (semantics 1 and 2).

## Calls with a literal pattern, outside the 34

Python compiles these on first use. None is a route problem; the port must cover them.

| where | call | pattern | note |
|---|---|---|---|
| `contact.py:29` | `sub` | `\D` | remove non-digits (`\D` is not `\p{Nd}`) |
| `contact.py:33` | `search` | `[\s.()-]` | `\s` per semantics 2 |
| `credentials.py:322` | `sub` | `(?<=[a-z0-9])(?=[A-Z])` | `_snake`: insert `_` between `[a-z0-9]` and `[A-Z]`; two lookarounds, no regex needed (a loop over characters); its `.lower()` is Python's full Unicode lower-casing (U+0130 becomes two characters), not an ASCII loop (semantics 12) |
| `credentials.py:381` | `split`, `maxsplit=1` | `[^a-z0-9]+` | the text before the first character outside `[a-z0-9]` |
| `detectors.py:80` | `sub` | `[ -]` | remove spaces and hyphens |
| `detectors.py:97` | `sub` | `\D` | as above |
| `detectors.py:148` | `findall` | `([A-Z]{2})(\d{2})` | over a constant ASCII string: build the table once |
| `detectors.py:198` | `search` | `\d` | `\p{Nd}` |
| `detectors.py:208` | `split` | `:+` | drop empty parts |
| `detectors.py:209` | `search` | `[A-Fa-f]` | |
| `detectors.py:315` | `sub` | `\D` | as above |
