#!/usr/bin/env bash
# Run all cargo-fuzz targets using the nightly toolchain.
#
# Usage examples:
#   THREADS=1 ./fuzz.sh                # run sequentially (default)
#   THREADS=4 FUZZ_TIME=90 ./fuzz.sh   # run in parallel with 4 jobs, 90s each
#   QUIET=1 ./fuzz.sh                  # be less verbose (also affects libFuzzer)
#   ./fuzz.sh -- -only_ascii=1         # pass extra args to libFuzzer after "--"
#
# Environment variables:
#   THREADS        How many targets to run in parallel (default: 1)
#   FUZZ_TIME      libFuzzer -max_total_time per target in seconds (default: 60)
#   RSS_LIMIT_MB   libFuzzer -rss_limit_mb (default: 1024)
#   CLOSE_FD_MASK  libFuzzer -close_fd_mask (default: 3)
#   QUIET          If set to 1, reduce cargo and libFuzzer verbosity (default: 0)
#
set -euo pipefail

# Defaults
THREADS="${THREADS:-1}"
FUZZ_TIME="${FUZZ_TIME:-60}"
RSS_LIMIT_MB="${RSS_LIMIT_MB:-1024}"
CLOSE_FD_MASK="${CLOSE_FD_MASK:-3}"
QUIET="${QUIET:-0}"

# Capture caller working directory and start timestamps for stats logging
CALLER_CWD="${PWD}"
START_TS="$(date "+%Y%m%d-%H%M%S")"
START_ISO="$(date -Iseconds)"
START_EPOCH="$(date +%s)"

# Extra args to pass through to libFuzzer (anything after "--")
EXTRA_ARGS=( )
pass_through=false
for arg in "$@"; do
  if [[ "$arg" == "--" ]]; then
    pass_through=true
    continue
  fi
  if $pass_through; then
    EXTRA_ARGS+=("$arg")
  fi
done

# Ensure we run from repository root (this script resides there)
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

if [[ ! -d fuzz ]]; then
  echo "[error] 'fuzz' directory not found. Are you in the repository root?" >&2
  exit 1
fi

# Ensure nightly toolchain exists; prefer not to change user's default.
if command -v rustup >/dev/null 2>&1; then
  if ! rustup toolchain list | grep -q "^nightly"; then
    [[ "$QUIET" == "1" ]] || echo "Installing Rust nightly toolchain..."
    rustup toolchain install nightly --profile minimal -y >/dev/null 2>&1 || rustup toolchain install nightly --profile minimal -y
  fi
else
  [[ "$QUIET" == "1" ]] || echo "[warn] rustup not found; relying on cargo +nightly to fetch toolchain if available." >&2
fi

# Ensure cargo-fuzz is installed.
if ! command -v cargo-fuzz >/dev/null 2>&1; then
  [[ "$QUIET" == "1" ]] || echo "Installing cargo-fuzz..."
  if [[ "$QUIET" == "1" ]]; then
    cargo +nightly install cargo-fuzz -q
  else
    cargo +nightly install cargo-fuzz
  fi
fi

# Discover fuzz targets from fuzz/Cargo.toml [[bin]] sections
mapfile -t TARGETS < <(awk '
  BEGIN { inbin=0 }
  /^\[\[bin\]\]/ { inbin=1; next }
  /^\[\[/ { if ($0 !~ /^\[\[bin\]\]/) inbin=0 }
  inbin && $1 ~ /^name$/ && $2 == "=" {
    # name = "target"
    match($0, /"([^"]+)"/, m)
    if (m[1] != "") print m[1]
  }
' fuzz/Cargo.toml)

if [[ ${#TARGETS[@]} -eq 0 ]]; then
  echo "[error] No fuzz targets found in fuzz/Cargo.toml" >&2
  exit 1
fi

if [[ "$QUIET" != "1" ]]; then
  echo "Found fuzz targets: ${TARGETS[*]}"
fi

# Determine cargo quiet flag
CARGO_QUIET=( )
if [[ "$QUIET" == "1" ]]; then
  CARGO_QUIET+=("--quiet")
fi

# If quiet and user didn't specify -verbosity, add low-verbosity flags for libFuzzer
if [[ "$QUIET" == "1" ]]; then
  has_verbosity_flag=0
  for a in "${EXTRA_ARGS[@]}"; do
    if [[ "$a" == -verbosity=* || "$a" == "-verbosity" ]]; then
      has_verbosity_flag=1; break
    fi
  done
  if [[ $has_verbosity_flag -eq 0 ]]; then
    EXTRA_ARGS+=("-verbosity=0" "-print_final_stats=1")
  fi
fi

run_one() {
  local tgt="$1"
  if [[ "$QUIET" != "1" ]]; then
    echo "\n=== Running fuzz target '$tgt' for ${FUZZ_TIME}s ==="
  fi
  cargo +nightly fuzz run "${CARGO_QUIET[@]}" "$tgt" -- \
    -max_total_time="${FUZZ_TIME}" \
    -rss_limit_mb="${RSS_LIMIT_MB}" \
    -close_fd_mask="${CLOSE_FD_MASK}" \
    "${EXTRA_ARGS[@]}"
}

# Export variables/functions for subshells when running in parallel
export -f run_one
export FUZZ_TIME RSS_LIMIT_MB CLOSE_FD_MASK QUIET
export CARGO_QUIET
export EXTRA_ARGS

if [[ "$THREADS" -le 1 ]]; then
  # Sequential
  for t in "${TARGETS[@]}"; do
    run_one "$t"
  done
else
  # Parallel limited by THREADS
  printf '%s\n' "${TARGETS[@]}" | xargs -P "$THREADS" -n 1 -I {} bash -lc 'run_one "$@"' _ {}
fi

# Always write stats file to the caller's directory
END_ISO="$(date -Iseconds)"
END_EPOCH="$(date +%s)"
DURATION_SEC=$(( END_EPOCH - START_EPOCH ))
LOG_FILE="${CALLER_CWD}/fuzz_run_${START_TS}.stats.txt"
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
  echo "extra_args: ${EXTRA_ARGS[*]}"
  echo "hostname:   $(hostname 2>/dev/null || true)"
  echo "os_kernel:  $(uname -srm 2>/dev/null || true)"
  echo "cpu_count:  $(command -v nproc >/dev/null 2>&1 && nproc || getconf _NPROCESSORS_ONLN 2>/dev/null || echo unknown)"
  echo "git_commit: $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
  echo "rustc:      $(rustc +nightly --version 2>/dev/null || rustc --version 2>/dev/null || echo unavailable)"
  echo "cargo:      $(cargo --version 2>/dev/null || echo unavailable)"
  echo "cargo-fuzz: $(cargo-fuzz --version 2>/dev/null || echo unavailable)"
} >"${LOG_FILE}" 2>/dev/null || true

if [[ "$QUIET" != "1" ]]; then
  echo "\nAll fuzz targets completed. Stats written to: ${LOG_FILE}"
fi