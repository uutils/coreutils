#!/usr/bin/env bash

# spell-checker:ignore wasip wasmtime UUTESTS rustup

# Run the WASI integration-test selection from .github/workflows/wasi.yml in
# an Ubuntu 24.04 container. This includes Linux-only host-test paths that a
# native macOS run does not compile.

set -euo pipefail

usage() {
    cat <<'EOF'
Usage: run-wasi-integration-tests-docker.sh [--source PATH]

Run the WASI integration tests selected by PATH/.github/workflows/wasi.yml.
PATH defaults to the repository containing this script.
EOF
}

SOURCE_DIR=""
while (($# > 0)); do
    case "$1" in
        --source)
            if (($# < 2)); then
                echo "error: --source requires a path" >&2
                exit 2
            fi
            SOURCE_DIR="$2"
            shift 2
            ;;
        --source=*)
            SOURCE_DIR="${1#*=}"
            shift
            ;;
        -h | --help)
            usage
            exit 0
            ;;
        *)
            echo "error: unknown argument: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

SCRIPT_DIR="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
if [[ -z "${SOURCE_DIR}" ]]; then
    SOURCE_DIR="$(dirname -- "${SCRIPT_DIR}")"
elif [[ ! -d "${SOURCE_DIR}" ]]; then
    echo "error: source directory does not exist: ${SOURCE_DIR}" >&2
    exit 2
else
    SOURCE_DIR="$(CDPATH='' cd -- "${SOURCE_DIR}" && pwd -P)"
fi

if [[ ! -f "${SOURCE_DIR}/Cargo.toml" || ! -f "${SOURCE_DIR}/.github/workflows/wasi.yml" ]]; then
    echo "error: source directory is not a coreutils working tree: ${SOURCE_DIR}" >&2
    exit 2
fi

command -v docker >/dev/null 2>&1 || {
    echo "error: docker not found in PATH" >&2
    exit 1
}
docker info >/dev/null 2>&1 || {
    echo "error: docker daemon not reachable" >&2
    exit 1
}

HOST_LOG_DIR="$(mktemp -d "${TMPDIR:-/tmp}/wasi-coreutils.XXXXXX")"
HOST_LOG="${HOST_LOG_DIR}/wasi-integration-test-output.log"

# Report the log location on every exit path, including Docker failures.
trap 'echo; echo "Full log saved to ${HOST_LOG}"' EXIT

# Toolchains and build artifacts use named volumes so repeat runs only fetch
# updates. The source remains read-only and is copied into the container.
docker run --rm -i \
    --volume "${SOURCE_DIR}:/src:ro" \
    --volume "${HOST_LOG_DIR}:/host-tmp" \
    --volume uutils-coreutils-wasi-cargo:/root/.cargo \
    --volume uutils-coreutils-wasi-rustup:/root/.rustup \
    --volume uutils-coreutils-wasi-wasmtime:/root/.wasmtime \
    --volume uutils-coreutils-wasi-target:/target \
    ubuntu:24.04 bash -se <<'EOF'
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive

apt-get update -qq
apt-get install -y -qq curl rsync ca-certificates build-essential pkg-config libssl-dev xz-utils >/dev/null

if [[ ! -x /root/.cargo/bin/rustup ]]; then
    curl -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none --profile minimal >/dev/null
fi
. /root/.cargo/env
rustup toolchain install stable --profile minimal --target wasm32-wasip1 >/dev/null
rustup default stable >/dev/null

curl -sSf https://wasmtime.dev/install.sh | bash >/dev/null
export PATH="/root/.wasmtime/bin:$PATH"

mkdir -p /work
rsync -a --exclude=target --exclude='target-*' --exclude=.git /src/ /work/
cd /work
export CARGO_TARGET_DIR=/target

mapfile -t TEST_SELECTORS < <(
    awk '
        /cargo test --test tests --/ {
            capture = 1
            next
        }
        capture {
            for (i = 1; i <= NF; i++) {
                token = $i
                sub(/\\$/, "", token)
                if (token ~ /^test_[[:alnum:]_:]+$/) {
                    print token
                }
            }
            if ($0 !~ /\\[[:space:]]*$/) {
                exit
            }
        }
    ' .github/workflows/wasi.yml
)

if (("${#TEST_SELECTORS[@]}" == 0)); then
    echo "error: no WASI integration-test selectors found in .github/workflows/wasi.yml" >&2
    exit 1
fi

LOG=/host-tmp/wasi-integration-test-output.log
: > "${LOG}"

{
    echo "=== Tool versions ==="
    rustc --version
    cargo --version
    wasmtime --version
    printf 'Integration-test selectors (%d):' "${#TEST_SELECTORS[@]}"
    printf ' %s' "${TEST_SELECTORS[@]}"
    echo

    echo "=== Building WASI binary ==="
    RUSTFLAGS="--cfg wasi_runner" \
        cargo build --locked --target wasm32-wasip1 --no-default-features --features feat_wasm
} 2>&1 | tee -a "${LOG}"

# Preserve the test exit status while still writing the failure summary.
echo "=== Running WASI integration tests ===" | tee -a "${LOG}"
set +e
RUSTFLAGS="--cfg wasi_runner" \
UUTESTS_BINARY_PATH="${CARGO_TARGET_DIR}/wasm32-wasip1/debug/coreutils.wasm" \
UUTESTS_WASM_RUNNER=wasmtime \
    cargo test --locked --test tests -- "${TEST_SELECTORS[@]}" 2>&1 | tee -a "${LOG}"
test_status=${PIPESTATUS[0]}
set -e

echo | tee -a "${LOG}"
echo "=== Failure summary ===" | tee -a "${LOG}"
grep -E "FAILED|^failures:|test result" "${LOG}" || echo "No failure or test-result lines found"
exit "${test_status}"
EOF
