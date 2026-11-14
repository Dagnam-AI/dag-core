# dag-core

Compiled core for the Dagnam.AI libraries: a Cargo workspace plus a maturin/PyO3 Python
package, `dagnam-core`. It is an internal dependency; install `dagnam-contracts` or `dagnam`
instead of importing it directly.

- `crates/hygiene` (`dagnam-hygiene`): pure Rust data-hygiene primitives, no Python.
- `crates/py` (`dagnam-core-py`): the PyO3 bindings, the only crate that names Python.
- `python/dagnam_core`: the Python package and its type stubs.

`dagnam_core.hygiene.CORE_API` is the binding contract version. It is bumped whenever a
binding changes meaning, and the consuming packages check it.

## Gate

```sh
scripts/check.sh
```

Runs fmt, the 500-line cap, clippy, rustdoc, `cargo deny`, `cargo machete`, 100% coverage on
`dagnam-hygiene`, mutation testing on the diff against `origin/main`, shellcheck, then builds
the wheel and runs the Python tests against it. It needs rustup, `cargo-llvm-cov`,
`cargo-nextest`, `cargo-mutants`, `cargo-deny`, `cargo-machete`, `shellcheck` and `uv`.

Licence: Apache-2.0.
