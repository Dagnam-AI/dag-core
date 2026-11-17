# dag-core

Compiled core for the Dagnam.AI libraries: a Cargo workspace plus a maturin/PyO3 Python
package, `dagnam-core`. It is an internal dependency: install `dagnam-contracts` or `dagnam`
instead of importing it directly, and expect its API to change without notice.

Native wheels are published for the platforms below. Each has one `abi3` wheel for CPython 3.12
and later, plus a free-threaded wheel for CPython 3.14t (version-specific, not `abi3`). Every other
interpreter or platform installs a pure-Python stub wheel instead of building the sdist, so a
plain install never needs a Rust toolchain (`--no-binary` asks for the sdist and does need one).
There is no free-threaded 3.13 wheel on purpose: PyO3 0.29 and later dropped free-threaded CPython
older than 3.14, and 0.28.3, the newest release that supports it, carries RUSTSEC-2026-0177, so the
core does not go back to it. A pip user who names a newer target (`pip install --platform
manylinux_2_28_x86_64 --target DIR ...`) must name `manylinux2014_<arch>` instead, or pip finds no
native wheel and takes the stub.

| platform | wheel tag | runs on |
|---|---|---|
| Linux x86_64, aarch64 (glibc) | `manylinux_2_17` (manylinux 2014) | glibc 2.17 and later: CentOS/RHEL 7, Amazon Linux 2, Ubuntu 16.04 and later, Debian 8 and later |
| Linux x86_64, aarch64 (musl) | `musllinux_1_2` | musl 1.2 and later: Alpine 3.13 and later |
| macOS x86_64 | `macosx_*_x86_64` | the macOS version in the wheel's tag and later |
| macOS arm64 | `macosx_*_arm64` | the macOS version in the wheel's tag and later |
| Windows x86_64 | `win_amd64` | 64-bit Windows |
| any other interpreter or platform (free-threaded 3.13, PyPy 3.12 and later, Windows arm64, a glibc older than 2.17, ...) | `py3-none-any` stub | installs, but `import dagnam_core.hygiene` raises `ImportError`; `dagnam-contracts` then uses its pure-Python code |

Releases: [RELEASE.md](RELEASE.md), [CHANGELOG.md](CHANGELOG.md).

- `crates/hygiene` (`dagnam-hygiene`): pure Rust data-hygiene primitives, no Python.
- `crates/py` (`dagnam-core-py`): the PyO3 bindings, the only crate that names Python.
- `python/dagnam_core`: the Python package and its type stubs.

`dagnam_core.hygiene.CORE_API` is the binding contract version. It is bumped whenever a
binding changes meaning, and the consuming packages check it.

The character classes (`\w`, `\d`, `\s`) follow the Unicode database of the interpreter, as
Python's `re` does: the core ships the tables of Unicode 15.0.0, 15.1.0 and 16.0.0 (CPython 3.12,
3.13, 3.14 and 3.14t) and selects the one matching `unicodedata.unidata_version` at import.
`hygiene.UNICODE_VERSION` is that version and `hygiene.UNICODE_SUPPORTED` says whether the core has
its tables; on an interpreter it does not, the three functions raise `RuntimeError` and the caller
uses the pure-Python reference (see [docs/divergences.md](docs/divergences.md)). A process that
replaces `unicodedata` with another module before importing the package is unsupported.

## Gate

```sh
scripts/check.sh
```

Runs fmt, the 500-line cap, clippy, rustdoc, `cargo deny`, `cargo machete`, 100% coverage on
`dagnam-hygiene`, mutation testing on the diff against `origin/main`, shellcheck, then builds
the wheel and runs the Python tests against it. It needs rustup, `cargo-llvm-cov`,
`cargo-nextest`, `cargo-mutants`, `cargo-deny`, `cargo-machete`, `shellcheck` and `uv`.

Licence: Apache-2.0.
