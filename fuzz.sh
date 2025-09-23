#!/usr/bin/env bash
# Ultra-simple fuzz runner: hardcoded for 4 days on up to 48 cores.
# No auto-install, no fancy flags. Save logs, artifacts, and reproduce info.

# Hardcoded knobs
CORES=${CORES:-48}
# Duration: if DURATION is not set, compute from DURATION_DAYS (default 4)
DURATION_DAYS=${DURATION_DAYS:-8}
if [[ -z "${DURATION:-}" ]]; then
  DURATION=$(( DURATION_DAYS * 24 * 60 * 60 ))
fi
RSS_LIMIT_MB=${RSS_LIMIT_MB:-4096}
# Per-input timeout inside libFuzzer; keeps stuck inputs from stalling progress.
TIMEOUT=${TIMEOUT:-5}                  # libFuzzer per-run timeout
# close_fd_mask avoids leaking parent FDs to child processes in some environments.
# 3 keeps stdin/stdout/stderr open; adjust only if you know you need a different mask.
CLOSE_FD_MASK=${CLOSE_FD_MASK:-3}
# Run N independent instances per target (separate logs and artifact dirs)
INSTANCES=${INSTANCES:-10}               # run N independent instances per target
START_TS="$(date +%Y%m%d-%H%M%S)"
SUMMARY="fuzz_summary_${START_TS}.txt"

# Stay in repo root (must contain fuzz/)
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"
[[ -d fuzz ]] || { echo "fuzz/ not found"; exit 1; }

# Discover targets (require cargo-fuzz and nightly pre-installed)
mapfile -t TARGETS < <(cargo +nightly fuzz list)
((${#TARGETS[@]})) || { echo "no fuzz targets"; exit 1; }

# Workers per instance so total processes ≤ CORES
# We split the available cores across all target instances. Each cargo-fuzz process
# launches `-workers` threads. To keep the total worker threads roughly bounded by
# CORES, we divide CORES by the number of running instances. This is an approximation
# (I/O, scheduler, other processes may exist), but works well in practice and keeps
# the script simple without over-engineering orchestration.
TGT_N=${#TARGETS[@]}
TOTAL_INSTANCES=$(( TGT_N * INSTANCES ))
WORKERS=$(( CORES / (TOTAL_INSTANCES>0?TOTAL_INSTANCES:1) ))
# Ensure at least 1 worker per instance even if CORES < TOTAL_INSTANCES
(( WORKERS < 1 )) && WORKERS=1

mkdir -p fuzz/logs fuzz/artifacts
: >"$SUMMARY"

# Run a single independent instance for a given target.
# Each instance has its own artifact directory and log file so that
# crashes are not overwritten by concurrent processes.
run_target_instance() {
  local tgt="$1"
  local idx="$2"
  # Stable, compact identifier for the instance (run-01..run-10)
  local run_id
  run_id=$(printf "run-%02d" "$idx")
  local art_dir="fuzz/artifacts/${tgt}/${run_id}/"
  local log="fuzz/logs/${tgt}.${run_id}.${START_TS}.log"
  mkdir -p "$art_dir"
  echo "==> $tgt [$run_id] (workers=$WORKERS, total_duration=${DURATION}s (~$(( DURATION / 86400 ))d))" | tee -a "$SUMMARY"

  # We drive libFuzzer in chunks up to the remaining time via -max_total_time.
  # This lets us resume the loop even if a run ends early for any reason and
  # keeps all instances active until the global deadline for the instance.
  local start_ts=$(date +%s)
  local end_ts=$(( start_ts + DURATION ))
  local iter=0
  local status=0

  while :; do
    local now=$(date +%s)
    local rem=$(( end_ts - now ))
    (( rem <= 0 )) && break
    iter=$((iter + 1))

    echo "-- $tgt [$run_id]: iteration $iter, remaining ${rem}s" | tee -a "$SUMMARY"

    # Do not fail the script if cargo-fuzz exits non-zero; we record the status
    # and continue. Crashes are ignored within libFuzzer (-ignore_crashes=1),
    # so artifacts are saved but the process continues fuzzing.
    set +e
    cargo +nightly fuzz run "$tgt" -- \
      -workers=$WORKERS \
      -artifact_prefix="${art_dir}" \
      -max_total_time=$rem \
      -rss_limit_mb=$RSS_LIMIT_MB \
      -close_fd_mask=$CLOSE_FD_MASK \
      -ignore_crashes=1 \
      -print_final_stats=1 \
      -use_value_profile=1 -entropic=1 -len_control=1 -timeout=$TIMEOUT \
      2>&1 | tee -a "$log"
    status=${PIPESTATUS[0]}
    set -e

    # Collect artifacts from this iteration. These file name prefixes are the
    # standard ones produced by libFuzzer. We only scan the instance directory.
    local found=()
    while IFS= read -r -d '' f; do found+=("$f"); done < <(find "$art_dir" -maxdepth 1 -type f \
      \( -name 'crash-*' -o -name 'oom-*' -o -name 'timeout-*' \) -print0 2>/dev/null)

    {
      echo "target: $tgt"
      echo "instance: $run_id"
      echo "iteration: $iter"
      echo "status: $status"
      echo "log: $log"
      if ((${#found[@]})); then
        echo "artifacts:"; for f in "${found[@]}"; do echo "  - $f"; done
      else
        echo "artifacts: (none)"
      fi
    } >>"$SUMMARY"

    # For each new artifact, try to minimize (tmin). If minimization does not
    # produce a .min file, keep a copy of the original as .min so the reproduce
    # command in the summary always works.
    if ((${#found[@]})); then
      for f in "${found[@]}"; do
        local min="${f}.min"
        if [[ ! -f "$min" ]]; then
          cargo +nightly fuzz tmin "$tgt" "$f" -- -timeout=$TIMEOUT -runs=200000 -artifact_prefix="${art_dir}" 2>&1 | tee -a "$log" || true
          [[ -f "$min" ]] || cp -f "$f" "$min" || true
          {
            echo "reproduce (orig): cargo +nightly fuzz reproduce $tgt $f -- -timeout=$TIMEOUT"
            echo "reproduce (min):  cargo +nightly fuzz reproduce $tgt ${min} -- -timeout=$TIMEOUT"
          } >>"$SUMMARY"
        fi
      done
    fi
  done
}

# Launch all targets with multiple instances concurrently; total procs ~ TOTAL_INSTANCES * WORKERS ≤ CORES
for t in "${TARGETS[@]}"; do
  for i in $(seq 1 "$INSTANCES"); do
    run_target_instance "$t" "$i" &
  done
done
wait

echo "Done. Summary: $SUMMARY"
