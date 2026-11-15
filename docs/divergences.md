# Divergences

Places where the compiled core deliberately behaves differently from the Python code it
replaces, each with the reason and the test that pins it. A divergence that is not listed here
is a bug.

| Input | Python | Core | Reason | Pinned by |
|---|---|---|---|---|
| Text with a code point whose `\w` or `\d` status differs between the Unicode version of the reference interpreter (Python 3.13.14, Unicode 15.1.0) and the interpreter running the Python backend. On 3.12 (Unicode 15.0.0): 622 `\w` code points, U+2EBF0 to U+2EE5D (CJK Extension I). On 3.14 (Unicode 16.0.0): 4,382 `\w` and 80 `\d` code points, for example U+1C89 and U+10D40. Example: `a@b.` followed by U+2EBF0 | 3.12: no address; 3.13 and 3.14: one address, span 0 to 5 | With U+2EBF0: one address, span 0 to 5. With U+1C89: no address. (The 3.13 tables, on every interpreter.) | The core's `\w`, `\s`, `\d` tables are generated from the pinned reference interpreter, not from the crate's Unicode tables and not from whatever interpreter runs the Python backend (`requires-python` is `>=3.12`). The tables move when the pin moves. With `a@b.` then U+1C89, Python gives no address on 3.12 and 3.13 and one on 3.14. | To be added with the table generator: one corpus case per range above, and the generator's per-code-point check against `re`. |
