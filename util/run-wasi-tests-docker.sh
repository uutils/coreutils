#!/usr/bin/env bash

# spell-checker:ignore mktemp wasip wasmtime UUTESTS rustup

# Run the WASI integration tests in an Ubuntu 24.04 container. Mirrors the
# "Run integration tests via wasmtime" step of .github/workflows/wasi.yml
# (the unit-test step cross-compiles to wasm and runs under wasmtime on any
# host, so macOS already covers it). Keep the selector list below in sync
# with that workflow.
#
# The gap this closes: integration tests are host-built and many are gated
# with #[cfg(not(target_vendor = "apple"))] / #[cfg(target_os = "linux")],
# so macOS silently excludes them.

set -euo pipefail

command -v docker >/dev/null 2>&1 || {
    echo "error: docker not found in PATH" >&2
    exit 1
}
docker info >/dev/null 2>&1 || {
    echo "error: docker daemon not reachable" >&2
    exit 1
}

ME="${0}"
ME_resolved="$(readlink -f -- "${ME}" 2>/dev/null || python3 -c 'import os,sys;print(os.path.realpath(sys.argv[1]))' "${ME}" 2>/dev/null || true)"
if [[ -z "${ME_resolved}" || ! -f "${ME_resolved}" ]]; then
    echo "error: could not resolve script path (neither 'readlink -f' nor python3 available)" >&2
    exit 1
fi
ME_dir="$(dirname -- "${ME_resolved}")"
REPO_main_dir="$(dirname -- "${ME_dir}")"

HOST_LOG_DIR="$(mktemp -d -t wasi-coreutils-XXXXXX)"
HOST_LOG="${HOST_LOG_DIR}/wasi-test-output.log"

# Report the log location on every exit path (including docker failure).
trap 'echo; echo "Full log saved to ${HOST_LOG}"' EXIT

# Source is mounted read-only; only the log dir is writable by the container.
docker run --rm -i \
    -v "${REPO_main_dir}:/src:ro" \
    -v "${HOST_LOG_DIR}:/host-tmp" \
    ubuntu:24.04 bash -se <<'EOF'
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq curl rsync ca-certificates build-essential pkg-config libssl-dev xz-utils >/dev/null

curl -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal --target wasm32-wasip1 >/dev/null
. "$HOME/.cargo/env"

curl -sSf https://wasmtime.dev/install.sh | bash >/dev/null
export PATH="$HOME/.wasmtime/bin:$PATH"

mkdir -p /work
rsync -a --exclude='target' --exclude='target-linux' --exclude='.git' /src/ /work/
cd /work
mkdir -p target-linux
export CARGO_TARGET_DIR=/work/target-linux

LOG=/host-tmp/wasi-test-output.log
: > "$LOG"

echo "=== Building WASI binary ===" | tee -a "$LOG"
RUSTFLAGS="--cfg wasi_runner" \
    cargo build --target wasm32-wasip1 --no-default-features --features feat_wasm 2>&1 | tee -a "$LOG"

# `set +e` around the pipeline so a test failure still reaches the summary.
echo "=== Running integration tests via wasmtime ===" | tee -a "$LOG"
set +e
RUSTFLAGS="--cfg wasi_runner" \
UUTESTS_BINARY_PATH="$CARGO_TARGET_DIR/wasm32-wasip1/debug/coreutils.wasm" \
UUTESTS_WASM_RUNNER=wasmtime \
    cargo test --test tests -- \
        test_base32:: test_base64:: test_basenc:: test_basename:: \
        test_cat:: test_comm:: test_cut:: test_dirname:: test_echo:: \
        test_expand:: test_factor:: test_false:: test_fold:: \
        test_head:: test_link:: test_nl:: test_numfmt:: \
        test_od:: test_paste:: test_printf:: test_shuf:: test_sort:: \
        test_sum:: test_tail:: test_tee:: test_touch:: test_tr:: \
        test_true:: test_truncate:: test_unexpand:: test_unlink:: test_wc:: \
        2>&1 | tee -a "$LOG"
test_status=${PIPESTATUS[0]}
set -e

echo "" | tee -a "$LOG"
echo "=== Failure summary (from saved log) ===" | tee -a "$LOG"
grep -E "FAILED|^failures:|test result" "$LOG" || echo "no FAILED / failures: / test result lines found"
exit "$test_status"
EOF
