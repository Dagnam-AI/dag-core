# Reference corpus

JSON Lines cases written by the Python reference implementation of the hygiene
code (`dagnam-contracts`). The Rust crate is tested against them: every case is a
fact the reference produced, so a disagreement is a bug in the port. Every expected
value in a file was computed by the reference; none was typed by hand.

- Written by `python/tests/hygiene/export_corpus.py` in
  [dag-contracts](https://github.com/Dagnam-AI/dag-contracts) at the commit that merges
  that last changed the exporter's cases, `8ee5f42` (`dagnam-contracts` after 0.6.0: the
  credentials finder reads a name inside a refused value, and the exporter's case table holds those
  shapes), under Python 3.13.14. `minhash.jsonl`, `near_dedup.jsonl` and the 15.1.0 line of
  `classes.jsonl` are byte-for-byte what the earlier export wrote; `findings.jsonl` gained one
  `PII_PHONE` case and `credentials.jsonl` 4,374 cases.
- Regenerate with `uv run python tests/hygiene/export_corpus.py <dir>` from the
  `python/` folder of that checkout; it prints the counts below. The output is seeded
  and sorted, so the same commit writes the same bytes.
- Never hand-edit these files. A change to the reference is a new export, committed
  with the commit that wrote it.
- Files are UTF-8, one case per line, lines end in `\n` only (`.gitattributes` pins it).
  Split on `\n`, never on Unicode line breaks: texts hold U+2028, U+0085 and the like
  unescaped.

## Cases

| file | line | cases |
|---|---|---|
| `minhash.jsonl` | `{"shingles": [str], "signature": [str]}`: the 128 values are decimal strings, because they exceed 2^53. `shingles` may repeat a value; hash the set | 739 |
| `near_dedup.jsonl` | `{"texts": [str], "threshold": float, "duplicate_indices": [int], "pairs": [[int, int, float]]}` | 180 |
| `findings.jsonl` | `{"text": str, "spans": [[int, int, str]]}`: offsets in code points (not UTF-16 units, bytes or graphemes) and the class code | 3796 |
| `credentials.jsonl` | `{"parts": [str], "spans": [[int, int, str]]}`: the text is `"".join(parts)`, the spans are what the reference reports on that text (see below) | 14075 |
| `classes.jsonl` | `{"unicode": str, "word": [[int, int]], "space": [[int, int]], "digit": [[int, int]]}`: one line per Unicode version (15.0.0, 15.1.0, 16.0.0, in that order), the inclusive code-point ranges that Python's `re` matches with `\w`, `\s` and `\d` under the interpreter that has it, ascending, with adjacent runs merged and the surrogates (U+D800 to U+DFFF) skipped | 3 |

`classes.jsonl` is computed with `re.fullmatch` over every code point, one line per interpreter:
Python 3.12 (Unicode 15.0.0) has 748 word ranges (137,936 code points), 10 space ranges (29) and
64 digit ranges (680); Python 3.13.14 (15.1.0) 749 (138,558), 10 (29) and 64 (680); Python 3.14
(16.0.0, the same under 3.14t) 771 (142,940), 10 (29) and 71 (760). It holds no text, so the
credential policy below does not touch it. The Rust crate's tables are generated from the same
interpreters (`scripts/gen_py_classes.py`, whose `--jsonl` writes a line), and a test compares
every predicate of every version with this file on every code point. The other corpus files were
exported under Python 3.13.14, so their tests read under 15.1.0.

`findings.jsonl` holds, per class code: PII_CA_SIN 218, PII_DATE_OF_BIRTH 266,
PII_EMAIL 543, PII_EU_VAT 537, PII_IBAN 510, PII_IN_AADHAAR 529, PII_IN_PAN 118,
PII_IP_ADDRESS 715, PII_NATIONAL_ID 54, PII_PAYMENT_CARD 329, PII_PHONE 854,
PII_UK_NINO 137. It includes 1,902 windows recovered from texts that held a credential
nearby: the window around each finding that is not a credential, cut back to hold no
credential region, with the spans the reference reports on the window itself.

No credential-shaped string, fake or not, is committed. "Credential-shaped" is wider than
"the reference finds it", because the reference reads a credential in context and a text
it passes (an authorization header with a short value, a password assignment with a
short value) is as readable in a file. A text with a *credential region* is kept out of
`minhash.jsonl`, `near_dedup.jsonl` and `findings.jsonl` (the exporter dropped 2,093
findings texts and windows, 175 MinHash sets and 1,919 seeded near-dedup rows for it, and
2 more for not being UTF-8) and lives only as fragments in `credentials.jsonl`. A region
is the union of: every `PII_SECRET` span; every provider prefix or marker (case-folded)
with the non-blank run around it, including the words that open an authorization header;
every case-folded occurrence of a value of six or more characters that the reference
reports as a secret anywhere (a value made only of letters, or only of digits, that also
stands free in three or more ordinary texts, such as a common word or `123456`, is left
out of this set); every secret-like name (case-insensitive) followed by a separator (a
colon, an equals sign, `:=`, `=>` or `->`, with any run of blanks, quotes and backslashes
around it) and a value of three or more non-blank characters, found from every
position of the text so that a second name inside the first one's value is a region of its
own (the names are secret, secret key, API key, access key or token, client secret,
credential, password or passwd, `pwd`, passphrase, auth or authorization, token, private key and
bearer, each with up to sixteen word characters or hyphens after it); and every URL whose
authority holds credentials, with or without a colon (an optional scheme of word
characters, `+`, `.` or `-`, then `://`, any characters but a slash or a blank, up to the
last `@`).

The constants in `crates/hygiene/tests/main.rs` (the case counts and the per-class
minimums) and these numbers move together with every re-export, in the same commit.

## `credentials.jsonl`

Each line is `{"parts": [str, ...], "spans": [[start, end, code]]}`. The text is
`"".join(parts)`; `spans` is what the reference's `findings` returns for that text
(code-point offsets and the class code, as in `findings.jsonl`), so a case may also hold
a phone number or an address. Cases are sorted by assembled text and not repeated. About
about one in four (3,241) have no `PII_SECRET` span: near misses that carry a credential
marker or a name shape (a provider prefix with too short a body, a name and separator with
a value the reference does not read as a secret, a PEM header with no key).

How a text is stored: every credential region is cut into pieces of at most four
characters (none holding a whole marker), and the text around the regions stays in whole
plain parts. Where the written line would still read as a secret to the reference (a
plain part that ends in a quoted name and its separator, followed by pieces), the pieces
reach further around the region, up to the whole text. Part boundaries carry no meaning:
a port derives nothing from them, only the joined text and the spans count. Nothing in
the file is a credential contiguously, and the exporter's tests check: the raw bytes of
all four files for every marker, and for every pool value that a JSON line writes as it
is (a value that is printable, `str.isprintable`, with no quote or backslash; any other,
such as one holding a comma between two quotes, is formed by the line's own punctuation
and is checked only in the decoded strings); every decoded string for every pool value; every decoded string
and raw line for every name shape (a wider one than the exporter's, found at every
position) and every URL with userinfo; and the reference over every raw line (JSON
punctuation included). The exporter's own sources, in the public repository that holds
it, are checked for the same markers and shapes and for any `name:value` pair in a
string literal, so they hold none either.

The cases come from the credential tables the Python suite pins, every text dropped from
the other files for holding a credential, windows around them (widened to hold a
credential whole, so none is cut in half), and generated ones: each provider format at
the shortest valid length, one shorter, longer; assignments (names x separators x values,
quoted and bare); URLs with userinfo; PEM blocks (with and without the END line);
JWT-like shapes; several secrets in one text, adjacent and overlapping ones; non-ASCII
neighbours. Cases per format (a case counts for every format it holds): `assignment_bare`
5,828, `assignment_quoted` 887, `aws_key_id` 293, `basic` 240, `bearer` 348, `github` 632,
`gitlab` 125, `google` 163, `huggingface` 146, `jwt` 127, `openai_anthropic` 1,471, `pem`
234, `pwd` 1,134, `slack` 374, `stripe` 283, `url_password` 1,090; no `PII_SECRET` span
3,241. The format labels are exporter-side heuristics (the span's prefix and the text
before it), not the rule that fired in the reference, so a small number of spans carry a
neighbouring label such as `openai_anthropic`, `google` or `github`; counts are
approximate.

The credentials finder is compared with the whole file (`crates/hygiene/tests/credentials/`):
every `PII_SECRET` span of a case is found exactly on the whole text (14,630 of them), inside a
wider region or by the piece the scan reads again (687 and 34), and the finder reports nothing
the reference does not. The file's shape and counts are pinned in `crates/hygiene/tests/main.rs`
(cases with a secret at least 10,834, without 3,241, with several 2,773). It holds the shapes of
the credentials fix of `8ee5f42`: a name inside a value an ambiguous name refused
(`<name>=<20 letters><joiner><name>=<value>`, 13 joiners, the names and separators, four value
kinds and four wrappers), runs of repeated names, and a name that starts inside a value that is
taken whole.

## `near_dedup.jsonl`

`texts` are the strings the reference reduced rows to: each case was computed on
`[{"t": text} for text in texts]`, and `row_text` of each of those rows is the text
itself (an exporter test checks it). So a port shingles a stored text directly, split
on one U+0020; it does not normalise again. The cases are:

- groups of 40 seeded rows and one of 100, in which a few texts are overwritten at
  seeded positions by near-copies of others (an exact copy, two words swapped, the
  middle word replaced, a word dropped, a word appended), at every threshold;
- groups of 0, 1, 2 and 3 texts, one with several empty texts, one of one- and
  two-word texts (fewer than a shingle);
- chains (A like B, B like C, A not like C), in three orders;
- pairs whose estimate reaches the threshold but that share no LSH band, so the
  reference never compares them;
- a pair that lands exactly on the threshold 0.875 (112 of 128 positions agree).

Thresholds: 0.3, 0.5, 0.875, 0.9, 0.99, 1.0.

## `minhash.jsonl`

The distinct shingle sets of the pinned tables, the planted inputs and the empty text
(an all-`2^64 - 1` signature), plus sets that are no text's shingles: duplicates
(`["a", "a"]`), an astral shingle, one with a tab, a 5,000-character shingle and a
3,000-shingle set.
