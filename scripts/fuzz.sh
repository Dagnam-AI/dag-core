#!/usr/bin/env bash
# Runs every fuzz target for FUZZ_SECONDS (default 300) on the nightly toolchain, one after the
# other, and exits non-zero on the first crash. Fuzz artifacts and corpora stay out of git.
# Heavy: run it under `nice` and, on a shared machine, a memory watchdog passed as FUZZ_WRAPPER
# (a command and its arguments, for example FUZZ_WRAPPER="nice -n 15"); never on a loaded
# machine. `-rss_limit_mb` caps libFuzzer's own memory either way.
set -euo pipefail
cd "$(dirname "$0")/.." || exit 1
export PATH="$HOME/.cargo/bin:$PATH"

seconds="${FUZZ_SECONDS:-300}"
read -r -a wrapper <<<"${FUZZ_WRAPPER:-}"
targets=(findings near_duplicates)
status=0
summary=""
for target in "${targets[@]}"; do
  echo "== fuzzing $target for ${seconds}s"
  log=$(mktemp)
  # ${wrapper[@]+...} keeps an empty wrapper valid on the bash 3.2 that macOS ships.
  if ${wrapper[@]+"${wrapper[@]}"} cargo +nightly fuzz run "$target" -- \
    -max_total_time="$seconds" -max_len=65536 -rss_limit_mb=2048 2>&1 | tee "$log"; then
    result=ok
  else
    result=CRASH
    status=1
  fi
  # libFuzzer's closing stats line: "Done N runs in S second(s)".
  runs=$(grep -E '^Done [0-9]+ runs' "$log" | tail -n 1 || true)
  summary+="$target: $result ${runs:-no completion line}"$'\n'
  rm -f "$log"
  [ "$status" -eq 0 ] || break
done
echo "== summary"
printf '%s' "$summary"
exit "$status"
