#!/usr/bin/env bash
# Run all cargo-fuzz targets (nightly) and summarize crashes.
# Env:
#   THREADS     (default 1)          # max parallel targets
#   FUZZ_TIME   (seconds, default 60)
#   RSS_LIMIT_MB(default 1024)
#   CLOSE_FD_MASK (default 3)
#   QUIET       (0/1, default 0)     # only trims libFuzzer chatter + our echoes
#   MINIMIZE    (0/1, default 0)     # run tmin on crashes
#   REPRODUCE   (0/1, default 0)     # write reproduce cmd into summary
#   MAX_PROCS   (default 48)         # hard cap: THREADS * workers <= MAX_PROCS
# Usage:
#   THREADS=4 FUZZ_TIME=345600 MAX_PROCS=48 ./fuzz.sh -- -workers=10 -jobs=10
set -euo pipefail

# --- Defaults ---
THREADS="${THREADS:-1}"
FUZZ_TIME="${FUZZ_TIME:-60}"
RSS_LIMIT_MB="${RSS_LIMIT_MB:-1024}"
CLOSE_FD_MASK="${CLOSE_FD_MASK:-3}"
QUIET="${QUIET:-0}"
MINIMIZE="${MINIMIZE:-0}"
REPRODUCE="${REPRODUCE:-0}"
MAX_PROCS="${MAX_PROCS:-48}"

CALLER_CWD="${PWD}"
START_TS="$(date "+%Y%m%d-%H%M%S")"
START_ISO="$(date -Iseconds)"
START_EPOCH="$(date +%s)"

# --- Parse pass-through libFuzzer args after "--" ---
EXTRA_ARGS=( )
pass_through=false
for arg in "$@"; do
  if [[ "$arg" == "--" ]]; then pass_through=true; continue; fi
  $pass_through && EXTRA_ARGS+=("$arg")
done

# --- Locate repo root (script dir must contain fuzz/) ---
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"
[[ -d fuzz ]] || { echo "[error] 'fuzz' dir not found. Run from repo root."; exit 1; }

# --- Toolchains & cargo-fuzz availability ---
if command -v rustup >/dev/null 2>&1; then
  rustup toolchain list | grep -q "^nightly" || {
    [[ "$QUIET" == "1" ]] || echo "Installing Rust nightly…"
    rustup toolchain install nightly --profile minimal -y >/dev/null 2>&1 || rustup toolchain install nightly --profile minimal -y
  }
fi
command -v cargo-fuzz >/dev/null 2>&1 || {
  [[ "$QUIET" == "1" ]] || echo "Installing cargo-fuzz…"
  if [[ "$QUIET" == "1" ]]; then
    cargo +nightly install cargo-fuzz -q
  else
    cargo +nightly install cargo-fuzz
  fi
}

# --- Discover fuzz targets (prefer cargo-fuzz list; fallback to parsing TOML) ---
TARGETS=()
if out="$(set +e; cargo +nightly fuzz list 2>/dev/null; echo $? )"; then
  # last line is exit code captured by echo; split safely:
  IFS=$'\n' read -r -d '' -a lines < <(printf '%s\0' "$(cargo +nightly fuzz list 2>/dev/null || true)")
  if ((${#lines[@]} > 0)); then TARGETS=("${lines[@]}"); fi
fi
if ((${#TARGETS[@]} == 0)); then
  # Fallback: parse fuzz/Cargo.toml [[bin]]
  mapfile -t TARGETS < <(awk '
    BEGIN{inbin=0}
    /^\[\[bin\]\]/{inbin=1; next}
    /^\[\[/{if($0!~/^\[\[bin\]\]/)inbin=0}
    inbin && $1~/^name$/ && $2=="=" {match($0,/"([^"]+)"/,m); if(m[1]!="") print m[1]}
  ' fuzz/Cargo.toml 2>/dev/null || true)
fi
((${#TARGETS[@]})) || { echo "[error] No fuzz targets found."; exit 1; }
[[ "$QUIET" != "1" ]] && echo "Found fuzz targets: ${TARGETS[*]}"

# --- Quiet handling: only adjust libFuzzer verbosity if user didn't set it ---
if [[ "$QUIET" == "1" ]]; then
  has_verbosity=0
  for a in "${EXTRA_ARGS[@]}"; do
    [[ "$a" == -verbosity=* || "$a" == "-verbosity" ]] && has_verbosity=1
  done
  ((has_verbosity==0)) && EXTRA_ARGS+=("-verbosity=0" "-print_final_stats=1")
fi

# --- Sensible defaults for parsers (can be overridden by EXTRA_ARGS) ---
DEFAULT_FUZZ_ARGS=( -use_value_profile=1 -entropic=1 -len_control=1 -timeout=5 )

# --- Sanitizer / backtrace for nicer crash logs ---
export RUST_BACKTRACE=1
export ASAN_OPTIONS="${ASAN_OPTIONS:-abort_on_error=1:alloc_dealloc_mismatch=1:detect_leaks=1}"
export UBSAN_OPTIONS="${UBSAN_OPTIONS:-print_stacktrace=1:halt_on_error=1}"

# --- Helpers to read/update -flag=value in EXTRA_ARGS ---
get_flag_val() {
  local name="$1"; local val=""
  for a in "${EXTRA_ARGS[@]}"; do
    if [[ "$a" == "$name="* ]]; then val="${a#*=}"; break; fi
  done
  printf '%s' "$val"
}
set_flag_val() {
  local name="$1"; local value="$2"; local found=0
  for i in "${!EXTRA_ARGS[@]}"; do
    if [[ "${EXTRA_ARGS[i]}" == "$name="* ]]; then EXTRA_ARGS[i]="$name=$value"; found=1; fi
  done
  ((found==0)) && EXTRA_ARGS+=("$name=$value")
}

# --- Cap workers/jobs so THREADS * workers <= MAX_PROCS ---
REQ_WORKERS="$(get_flag_val -workers)"; [[ -z "$REQ_WORKERS" ]] && REQ_WORKERS=0
REQ_JOBS="$(get_flag_val -jobs)";     [[ -z "$REQ_JOBS"    ]] && REQ_JOBS=0

if (( REQ_WORKERS == 0 )); then
  EFF_WORKERS=$(( MAX_PROCS / THREADS ))
  (( EFF_WORKERS < 1 )) && EFF_WORKERS=1
else
  EFF_WORKERS="$REQ_WORKERS"
endif=false  # no-op to keep shellcheck quiet

TOTAL=$(( THREADS * EFF_WORKERS ))
if (( TOTAL > MAX_PROCS )); then
  EFF_WORKERS=$(( MAX_PROCS / THREADS ))
  (( EFF_WORKERS < 1 )) && EFF_WORKERS=1
  [[ "$QUIET" != "1" ]] && echo "Capping to ${EFF_WORKERS} workers per target so ${THREADS}×${EFF_WORKERS} ≤ MAX_PROCS=${MAX_PROCS}"
fi

# Ensure both flags present and aligned
set_flag_val -workers "$EFF_WORKERS"
set_flag_val -jobs    "$EFF_WORKERS"

mkdir -p fuzz/logs
SUMMARY_FILE="${CALLER_CWD}/fuzz_run_${START_TS}.summary.txt"
: >"$SUMMARY_FILE"

# --- One target run ---
run_one() {
  local tgt="$1"
  local log="fuzz/logs/${tgt}.${START_TS}.log"
  local art_dir="fuzz/artifacts/${tgt}/"
  mkdir -p "$art_dir"

  [[ "$QUIET" != "1" ]] && echo -e "\n=== START '$tgt' for ${FUZZ_TIME}s (workers=$(get_flag_val -workers)) ==="

  set +e
  cargo +nightly fuzz run "$tgt" -- \
    -artifact_prefix="${art_dir}" \
    -max_total_time="${FUZZ_TIME}" \
    -rss_limit_mb="${RSS_LIMIT_MB}" \
    -close_fd_mask="${CLOSE_FD_MASK}" \
    "${DEFAULT_FUZZ_ARGS[@]}" \
    "${EXTRA_ARGS[@]}" \
    2>&1 | tee "$log"
  local status="${PIPESTATUS[0]}"
  set -e

  # Collect artifacts
  local found=( )
  while IFS= read -r -d '' f; do found+=("$f"); done < <(
    find "$art_dir" -maxdepth 1 -type f \( -name 'crash-*' -o -name 'oom-*' -o -name 'timeout-*' \) -print0 2>/dev/null || true
  )

  {
    echo "target:   $tgt"
    echo "status:   $status"
    echo "log:      $log"
    echo "workers:  $(get_flag_val -workers)"
    if ((${#found[@]})); then
      echo "artifacts:"
      for f in "${found[@]}"; do echo "  - $f"; done
    else
      echo "artifacts: (none)"
    fi
    echo
  } >>"$SUMMARY_FILE"

  # Optional minimize & reproduce
  if ((${#found[@]})); then
    for f in "${found[@]}"; do
      if [[ "${MINIMIZE}" == "1" ]]; then
        local min="${f}.min"
        cargo +nightly fuzz tmin "$tgt" "$f" -- -timeout=5 -runs=100000 -artifact_prefix="${art_dir}" 2>&1 | tee -a "$log"
        [[ -f "$min" ]] || cp -f "$f" "$min"
      fi
      if [[ "${REPRODUCE}" == "1" ]]; then
        {
          echo "# Reproduce:"
          echo "cargo +nightly fuzz reproduce $tgt $f -- -timeout=5"
        } >>"$SUMMARY_FILE"
      fi
    done
  fi
}

# --- Concurrency: no xargs; keep arrays intact ---
active_jobs() { jobs -r -p | wc -l; }
for tgt in "${TARGETS[@]}"; do
  # Wait until we have a free slot
  while (( $(active_jobs) >= THREADS )); do
    # Avoid set -e abort on non-zero from waited jobs
    wait -n || true
  done
  run_one "$tgt" &
done
# Wait for all background runs
wait || true

# --- Stats footer ---
END_ISO="$(date -Iseconds)"
END_EPOCH="$(date +%s)"
DURATION_SEC=$(( END_EPOCH - START_EPOCH ))
STATS_FILE="${CALLER_CWD}/fuzz_run_${START_TS}.stats.txt"
{
  echo "start_time: ${START_ISO}"
  echo "end_time:   ${END_ISO}"
  echo "duration_s: ${DURATION_SEC}"
  echo "threads:    ${THREADS}"
  echo "targets_n:  ${#TARGETS[@]}"
  echo "targets:    ${TARGETS[*]}"
  echo "fuzz_time_s:${FUZZ_TIME}"
  echo "rss_limit_mb:${RSS_LIMIT_MB}"
  echo "close_fd_mask:${CLOSE_FD_MASK}"
  echo "quiet:      ${QUIET}"
  echo "minimize:   ${MINIMIZE}"
  echo "reproduce:  ${REPRODUCE}"
  echo "max_procs:  ${MAX_PROCS}"
  echo "extra_args: ${EXTRA_ARGS[*]}"
  echo "summary:    ${SUMMARY_FILE}"
  echo "hostname:   $(hostname 2>/dev/null || true)"
  echo "os_kernel:  $(uname -srm 2>/dev/null || true)"
  echo "cpu_count:  $(command -v nproc >/dev/null 2>&1 && nproc || getconf _NPROCESSORS_ONLN 2>/dev/null || echo unknown)"
  echo "git_commit: $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
  echo "rustc:      $(rustc +nightly --version 2>/dev/null || rustc --version 2>/dev/null || echo unavailable)"
  echo "cargo:      $(cargo --version 2>/dev/null || echo unavailable)"
  echo "cargo-fuzz: $(cargo-fuzz --version 2>/dev/null || echo unavailable)"
} >"$STATS_FILE" 2>/dev/null || true

[[ "$QUIET" != "1" ]] && echo -e "\nAll done.\nSummary: ${SUMMARY_FILE}\nStats:   ${STATS_FILE}"
