#!/usr/bin/env bash
# Ultra-simple fuzz runner: hardcoded for 4 days on up to 48 cores.
# No auto-install, no fancy flags. Save logs, artifacts, and reproduce info.
set -euo pipefail

# Hardcoded knobs
CORES=48
DURATION=345600            # 4 days in seconds
RSS_LIMIT_MB=4096
TIMEOUT=5                  # libFuzzer per-run timeout
CLOSE_FD_MASK=3
START_TS="$(date +%Y%m%d-%H%M%S)"
SUMMARY="fuzz_summary_${START_TS}.txt"

# Stay in repo root (must contain fuzz/)
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"
[[ -d fuzz ]] || { echo "fuzz/ not found"; exit 1; }

# Discover targets (require cargo-fuzz and nightly pre-installed)
mapfile -t TARGETS < <(cargo +nightly fuzz list)
((${#TARGETS[@]})) || { echo "no fuzz targets"; exit 1; }

# Workers per target so total processes ≤ CORES
TGT_N=${#TARGETS[@]}
WORKERS=$(( CORES / (TGT_N>0?TGT_N:1) ))
(( WORKERS < 1 )) && WORKERS=1

mkdir -p fuzz/logs fuzz/artifacts
: >"$SUMMARY"

run_target() {
  local tgt="$1"
  local art_dir="fuzz/artifacts/${tgt}/"
  local log="fuzz/logs/${tgt}.${START_TS}.log"
  mkdir -p "$art_dir"
  echo "==> $tgt (workers=$WORKERS, duration=${DURATION}s)" | tee -a "$SUMMARY"
  set +e
  cargo +nightly fuzz run "$tgt" -- \
    -workers=$WORKERS \
    -artifact_prefix="${art_dir}" \
    -max_total_time=$DURATION \
    -rss_limit_mb=$RSS_LIMIT_MB \
    -close_fd_mask=$CLOSE_FD_MASK \
    -use_value_profile=1 -entropic=1 -len_control=1 -timeout=$TIMEOUT \
    2>&1 | tee "$log"
  local status=${PIPESTATUS[0]}
  set -e

  # List artifacts
  local found=()
  while IFS= read -r -d '' f; do found+=("$f"); done < <(find "$art_dir" -maxdepth 1 -type f \
    \( -name 'crash-*' -o -name 'oom-*' -o -name 'timeout-*' \) -print0 2>/dev/null)

  {
    echo "target: $tgt"
    echo "status: $status"
    echo "log: $log"
    if ((${#found[@]})); then
      echo "artifacts:"; for f in "${found[@]}"; do echo "  - $f"; done
    else
      echo "artifacts: (none)"
    fi
  } >>"$SUMMARY"

  # Minimize and write reproduce commands
  if ((${#found[@]})); then
    for f in "${found[@]}"; do
      local min="${f}.min"
      cargo +nightly fuzz tmin "$tgt" "$f" -- -timeout=$TIMEOUT -runs=200000 -artifact_prefix="${art_dir}" 2>&1 | tee -a "$log" || true
      [[ -f "$min" ]] || cp -f "$f" "$min" || true
      {
        echo "reproduce (orig): cargo +nightly fuzz reproduce $tgt $f -- -timeout=$TIMEOUT"
        echo "reproduce (min):  cargo +nightly fuzz reproduce $tgt ${min} -- -timeout=$TIMEOUT"
      } >>"$SUMMARY"
    done
  fi
}

# Launch all targets concurrently; total procs ~ TGT_N * WORKERS ≤ CORES
for t in "${TARGETS[@]}"; do run_target "$t" & done
wait

echo "Done. Summary: $SUMMARY"
