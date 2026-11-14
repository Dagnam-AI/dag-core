#!/usr/bin/env bash
# The whole gate. Run from the repo root on a clean tree.
set -euo pipefail
cd "$(dirname "$0")/.." || exit 1
export PATH="$HOME/.cargo/bin:$PATH"

# pyo3 needs a Python >= 3.12 to type-check the bindings; the gate venv is also the wheel test env.
uv venv --python 3.12 .venv-gate --clear
export PYO3_PYTHON="$PWD/.venv-gate/bin/python"

cargo fmt --all -- --check
over=$(git ls-files | grep -v -E '^(Cargo\.lock|fuzz/Cargo\.lock|corpus/)' | while IFS= read -r f; do
  n=$(wc -l <"$f"); if [ "$n" -gt 500 ]; then echo "$f $n"; fi; done)
if [ -n "$over" ]; then echo "over 500 lines:"; echo "$over"; exit 1; fi
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo deny check
cargo machete
cargo llvm-cov nextest -p dagnam-hygiene \
  --fail-under-lines 100 --fail-under-regions 100 --fail-under-functions 100
if git rev-parse --verify -q origin/main >/dev/null; then
  git diff origin/main...HEAD -- crates/hygiene >"${TMPDIR:-/tmp}/dag-core.diff"
  if [ -s "${TMPDIR:-/tmp}/dag-core.diff" ]; then
    cargo mutants -p dagnam-hygiene --in-diff "${TMPDIR:-/tmp}/dag-core.diff" --jobs "${DAGNAM_MUTANTS_JOBS:-3}"
  fi
fi
shellcheck scripts/*.sh
uv pip install --python .venv-gate maturin pytest hypothesis
rm -rf target/wheels
.venv-gate/bin/maturin build --release --out target/wheels
uv pip install --python .venv-gate --force-reinstall target/wheels/dagnam_core-*.whl
(cd tests && ../.venv-gate/bin/python -m pytest -q .)
