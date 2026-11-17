#!/usr/bin/env bash
# memguard.sh LIMIT_MB TOTAL_MB COMMAND [ARG...]
#
# Runs COMMAND and, once a second, looks at the command and every process under it. A process above LIMIT_MB resident
# is SIGKILLed; if the whole tree is above TOTAL_MB, the largest process is. A runaway `cargo mutants`
# mutant (a loop that now never advances and pushes forever) is the case this exists for: its test
# fails by being killed, which is what a caught mutant is. macOS does not enforce `ulimit -v`, and a
# 24 GB laptop panicked twice on 2026-11-14 with two such mutants at 30 GB each.
# Exit status is COMMAND's. Kills are logged to stderr as "memguard: killed ...".
# A copy of info/scripts/memguard.sh. check.sh runs `cargo mutants` through it (limits
# DAGNAM_MUTANT_RSS_MB and DAGNAM_MUTANT_TOTAL_MB), locally and on CI. It reads processes with
# `ps -A -o pid=,ppid=,rss=,comm=`, which macOS and Linux (procps) both take.
set -u
if [ "$#" -lt 3 ]; then
  echo "usage: memguard.sh LIMIT_MB TOTAL_MB COMMAND [ARG...]" >&2
  exit 2
fi
limit_mb=$1
total_mb=$2
shift 2

"$@" &
root=$!

# Prints "rss_kb pid command" for every descendant of $root, largest first.
tree() {
  ps -A -o pid=,ppid=,rss=,comm= | awk -v root="$root" '
    { pid[NR]=$1; ppid[$1]=$2; rss[$1]=$3; name[$1]=$4; n=NR }
    function under(p,   d) { for (d=0; p && d<64; d++) { if (p==root) return 1; p=ppid[p] } return 0 }
    END { for (i=1; i<=n; i++) { p=pid[i]; if (under(p)) print rss[p], p, name[p] } }
  ' | sort -rn
}

watch() {
  while kill -0 "$root" 2>/dev/null; do
    snapshot=$(tree)
    if [ -n "$snapshot" ]; then
      top_kb=$(printf '%s\n' "$snapshot" | head -1 | cut -d' ' -f1)
      sum_kb=$(printf '%s\n' "$snapshot" | awk '{ s += $1 } END { print s + 0 }')
      if [ "$top_kb" -gt $((limit_mb * 1024)) ] || [ "$sum_kb" -gt $((total_mb * 1024)) ]; then
        victim=$(printf '%s\n' "$snapshot" | head -1)
        echo "memguard: killed ${victim#* } (${top_kb} KB resident, tree ${sum_kb} KB)" >&2
        kill -9 "$(printf '%s' "$victim" | cut -d' ' -f2)" 2>/dev/null
      fi
    fi
    sleep 1
  done
}

watch &
watcher=$!
wait "$root"
status=$?
kill "$watcher" 2>/dev/null
exit "$status"
