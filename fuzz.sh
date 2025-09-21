#!/usr/bin/env bash
# Run all cargo-fuzz targets (nightly) and summarize crashes.
# Env:
#   THREADS (default 1), FUZZ_TIME (s, default 60), RSS_LIMIT_MB (1024),
#   CLOSE_FD_MASK (3), QUIET (0/1), MINIMIZE (0/1), REPRODUCE (0/1)
#   EXTRA libFuzzer args after "--"
set -euo pipefail

THREADS="${THREADS:-1}"
FUZZ_TIME="${FUZZ_TIME:-60}"
RSS_LIMIT_MB="${RSS_LIMIT_MB:-1024}"
CLOSE_FD_MASK="${CLOSE_FD_MASK:-3}"
QUIET="${QUIET:-0}"
MINIMIZE="${MINIMIZE:-0}"
REPRODUCE="${REPRODUCE:-0}"

CALLER_CWD="${PWD}"
START_TS="$(date "+%Y%m%d-%H%M%S")"
START_ISO="$(date -Iseconds)"
START_EPOCH="$(date +%s)"

# Pass-through libFuzzer args after "--"
EXTRA_ARGS=( )
pass_through=false
for arg in "$@"; do
  if [[ "$arg" == "--" ]]; then pass_through=true; continue; fi
  $pass_through && EXTRA_ARGS+=("$arg")
done

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"
[[ -d fuzz ]] || { echo "[error] 'fuzz' dir not found"; exit 1; }

# Toolchains & cargo-fuzz
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

# Discover targets
mapfile -t TARGETS < <(awk '
  BEGIN{inbin=0} /^\[\[bin\]\]/{inbin=1;next} /^\[\[/{if($0!~/^\[\[bin\]\]/)inbin=0}
  inbin && $1~/^name$/ && $2=="=" {match($0,/"([^"]+)"/,m); if(m[1]!="") print m[1]}
' fuzz/Cargo.toml)
((${#TARGETS[@]})) || { echo "[error] No fuzz targets in fuzz/Cargo.toml"; exit 1; }
[[ "$QUIET" != "1" ]] && echo "Found fuzz targets: ${TARGETS[*]}"

# If QUIET and no -verbosity provided, reduce libFuzzer chatter
if [[ "$QUIET" == "1" ]]; then
  has_verbosity=0
  for a in "${EXTRA_ARGS[@]}"; do [[ "$a" == -verbosity=* || "$a" == "-verbosity" ]] && has_verbosity=1; done
  ((has_verbosity==0)) && EXTRA_ARGS+=("-verbosity=0" "-print_final_stats=1")
fi

# Recommended defaults for parsers (can be overridden by EXTRA_ARGS)
DEFAULT_FUZZ_ARGS=( -use_value_profile=1 -entropic=1 -len_control=1 -timeout=5 )

# Sanitizer / backtrace for nicer crash logs
export RUST_BACKTRACE=1
export ASAN_OPTIONS="${ASAN_OPTIONS:-abort_on_error=1:alloc_dealloc_mismatch=1:detect_leaks=1}"
export UBSAN_OPTIONS="${UBSAN_OPTIONS:-print_stacktrace=1:halt_on_error=1}"

mkdir -p fuzz/logs
SUMMARY_FILE="${CALLER_CWD}/fuzz_run_${START_TS}.summary.txt"
: >"$SUMMARY_FILE"

run_one() {
  local tgt="$1"
  local log="fuzz/logs/${tgt}.${START_TS}.log"
  local art_dir="fuzz/artifacts/${tgt}/"
  mkdir -p "$art_dir"

  [[ "$QUIET" != "1" ]] && echo -e "\n=== Running '$tgt' for ${FUZZ_TIME}s ==="
  # Run and do NOT abort on crash; capture status.
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
  while IFS= read -r -d '' f; do found+=("$f"); done < <(find "$art_dir" -maxdepth 1 -type f \( -name 'crash-*' -o -name 'oom-*' -o -name 'timeout-*' \) -print0 2>/dev/null || true)

  {
    echo "target: $tgt"
    echo "status: $status"
    echo "log:    $log"
    if ((${#found[@]})); then
      echo "artifacts:"
      for f in "${found[@]}"; do echo "  - $f"; done
    else
      echo "artifacts: (none)"
    fi
    echo
  } >>"$SUMMARY_FILE"

  # Optional minimize & reproduce hints
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

export -f run_one
export START_TS FUZZ_TIME RSS_LIMIT_MB CLOSE_FD_MASK QUIET EXTRA_ARGS DEFAULT_FUZZ_ARGS SUMMARY_FILE
export ASAN_OPTIONS UBSAN_OPTIONS RUST_BACKTRACE

if (( THREADS <= 1 )); then
  for t in "${TARGETS[@]}"; do run_one "$t"; done
else
  printf '%s\n' "${TARGETS[@]}" | xargs -P "$THREADS" -n 1 -I {} bash -lc 'run_one "$@"' _ {}
fi

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
