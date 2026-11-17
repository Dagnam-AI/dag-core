#!/usr/bin/env bash
# The whole gate. Run from the repo root on a clean, committed tree.
set -euo pipefail
cd "$(dirname "$0")/.." || exit 1
export PATH="$HOME/.cargo/bin:$PATH"

# --- clean tree: the gate describes the commit only if nothing else is on disk
dirty=$(git status --porcelain)
if [ -n "$dirty" ]; then
  echo "the working tree is not clean; commit or remove these first:"
  echo "$dirty"
  exit 1
fi
# --- end clean tree

# pyo3 needs a Python >= 3.12 to type-check the bindings; the gate venv is also the wheel test env.
uv venv --python 3.12 .venv-gate --clear
export PYO3_PYTHON="$PWD/.venv-gate/bin/python"
uv pip install --python .venv-gate \
  "maturin>=1.13,<2" "pytest>=9,<10" "ruff>=0.17,<0.18" "pyright>=1.1.414,<1.2" "mypy>=2.4,<3"

cargo fmt --all -- --check

# --- line cap: 500 lines per tracked text file (lockfiles and fuzz corpora excepted)
over=""
while IFS= read -r f; do
  [ -f "$f" ] || continue
  n=$(awk 'END{print NR}' "$f")
  if [ "$n" -gt 500 ]; then over+="$f $n"$'\n'; fi
done < <(git ls-files | grep -v -E '^(Cargo\.lock|fuzz/Cargo\.lock)$|(^|/)corpus/' || true)
if [ -n "$over" ]; then echo "over 500 lines:"; echo "$over"; exit 1; fi
# --- end line cap

cargo clippy --locked --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
cargo test --locked --workspace --doc # nextest does not run doctests
cargo deny --locked check
# fuzz/ is outside the workspace, so the checks above never read its lockfile.
cargo deny --manifest-path fuzz/Cargo.toml --locked check
cargo machete fuzz
cargo machete
cargo llvm-cov nextest --locked -p dagnam-hygiene \
  --fail-under-lines 100 --fail-under-regions 100 --fail-under-functions 100

# --- mutants: needs origin/main to diff against; CI fetches full history
if ! git rev-parse --verify -q origin/main >/dev/null; then
  echo "origin/main is missing, so the mutation stage cannot diff; run: git fetch origin"
  exit 1
fi
diff_file=$(mktemp)
trap 'rm -f "$diff_file"' EXIT
git diff origin/main...HEAD -- crates/hygiene >"$diff_file"
if [ -s "$diff_file" ]; then
  # A mutant that stops a loop advancing can fill memory in seconds and take the runner down;
  # memguard kills such a process, and a test that is killed has caught its mutant.
  scripts/memguard.sh "${DAGNAM_MUTANT_RSS_MB:-3000}" "${DAGNAM_MUTANT_TOTAL_MB:-10000}" \
    cargo mutants -p dagnam-hygiene --in-diff "$diff_file" --cargo-arg=--locked \
    --jobs "${DAGNAM_MUTANTS_JOBS:-3}"
else
  echo "no change under crates/hygiene against origin/main; nothing to mutate"
fi
# --- end mutants

shellcheck scripts/*.sh

# scripts/gen_py_classes.py is linted and type-checked here, and its `--check` runs under this
# venv's interpreter, Python 3.12 (Unicode 15.0): the 15.0 table and its corpus/classes.jsonl line
# are what that interpreter writes. The 15.1 and 16.0 tables need 3.13 and 3.14, which the gate
# does not fetch (it must work offline); CI's wheel job runs `--check` under 3.12, 3.13 and 3.14,
# and the release workflow under 3.12 and 3.14t. The Rust tests compare every table with
# corpus/classes.jsonl on every code point in all cases.
.venv-gate/bin/ruff check python tests scripts .github/scripts
.venv-gate/bin/ruff format --check python tests scripts .github/scripts
.venv-gate/bin/python scripts/gen_py_classes.py --check
rm -rf target/wheels
.venv-gate/bin/maturin build --locked --release --out target/wheels
uv pip install --python .venv-gate --force-reinstall target/wheels/dagnam_core-*.whl
.venv-gate/bin/pyright --pythonpath .venv-gate/bin/python
(cd tests && ../.venv-gate/bin/python -m mypy.stubtest dagnam_core.hygiene dagnam_core._native) # stub matches the wheel
(cd tests && ../.venv-gate/bin/python -m pytest -q .)
.venv-gate/bin/python -m unittest discover -s .github/scripts # the release file checks
