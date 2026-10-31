#!/usr/bin/env bash
# spell-checker:ignore (jargon) profdata profraw sysroot rustlib nullglob aeiou nocheck CGU Cprofile cygpath atexit
#
# Build uutils coreutils with Profile-Guided Optimization.
#
#   1. build an instrumented multicall binary (-Cprofile-generate)
#   2. run representative workloads to collect raw profiles
#   3. merge them with llvm-profdata
#   4. build the optimized binary (-Cprofile-use), unless --train-only
#
# Runs on Linux, macOS and Windows (git-bash).
#
# Usage:
#   util/build-pgo.sh [--target TRIPLE] [--target-dir DIR] [--features LIST]
#                     [--train-only] [--llvm-profdata PATH]

set -euo pipefail

# On Windows the instrumented binary, rustc and the profiler runtime are all
# native programs that do not understand git-bash's `/d/a/...` paths, while bash
# itself does not understand `D:\a\...`. `cygpath -m` yields `D:/a/...`, which
# both sides accept, so every path the script builds goes through it.
norm_path() {
    if command -v cygpath >/dev/null 2>&1; then cygpath -m "$1"; else printf '%s\n' "$1"; fi
}

REPO_ROOT="$(norm_path "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)")"
TARGET_DIR="${REPO_ROOT}/target/coreutils-pgo"
FEATURES="unix"
TARGET=""
TRAIN_ONLY=0
LLVM_PROFDATA=""

while [ $# -gt 0 ]; do
    case "$1" in
        --target) TARGET="$2"; shift 2 ;;
        --target-dir) TARGET_DIR="$2"; shift 2 ;;
        --features) FEATURES="$2"; shift 2 ;;
        --llvm-profdata) LLVM_PROFDATA="$2"; shift 2 ;;
        --train-only) TRAIN_ONLY=1; shift ;;
        -h|--help) sed -n '3,16p' "${BASH_SOURCE[0]}"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

mkdir -p "$TARGET_DIR"
TARGET_DIR="$(norm_path "$(cd "$TARGET_DIR" && pwd)")"

HOST="$(rustc --print host-tuple)"
[ -n "$TARGET" ] || TARGET="$HOST"
# Cargo puts the artifacts of an explicit --target under a per-target directory,
# and Windows binaries carry a suffix.
case "$TARGET" in *windows*) EXE=".exe" ;; *) EXE="" ;; esac
echo "target: ${TARGET} (host: ${HOST})"

SCRIPT_START=$SECONDS

fmt_duration() { printf '%dm%02ds' $(($1 / 60)) $(($1 % 60)); }
begin_step() {
    STEP_NAME="$1"
    STEP_START=$SECONDS
    echo
    echo "=== ${STEP_NAME} ==="
}
end_step() { echo "--- ${STEP_NAME}: $(fmt_duration $((SECONDS - STEP_START))) ---"; }

PROFILE_DIR="${TARGET_DIR}/profiles"
CORPUS_DIR="${TARGET_DIR}/corpus"
MERGED="${TARGET_DIR}/coreutils.profdata"

# llvm-profdata must come from the *active* toolchain: its version has to match
# the rustc that instrumented the binary.
if [ -z "$LLVM_PROFDATA" ]; then
    SYSROOT="$(norm_path "$(rustc --print sysroot)")"
    # llvm-profdata ships for the host, whatever we are cross-building for.
    case "$HOST" in *windows*) PROFDATA_EXE=".exe" ;; *) PROFDATA_EXE="" ;; esac
    LLVM_PROFDATA="${SYSROOT}/lib/rustlib/${HOST}/bin/llvm-profdata${PROFDATA_EXE}"
fi
if [ ! -x "$LLVM_PROFDATA" ]; then
    echo "llvm-profdata not found at ${LLVM_PROFDATA}" >&2
    echo "Run: rustup component add llvm-tools" >&2
    exit 1
fi
echo "llvm-profdata: ${LLVM_PROFDATA}"

cargo_build() {
    # $1: target dir, $2: extra rustflags
    local feature_args=()
    if [ -n "$FEATURES" ]; then feature_args=(--features="$FEATURES"); fi
    (
        cd "$REPO_ROOT"
        export CARGO_TARGET_DIR="$1"
        export CARGO_INCREMENTAL=0
        export RUSTFLAGS="${RUSTFLAGS:+${RUSTFLAGS} }$2"
        # bash 3.2 (macOS) errors on an empty array expansion under `set -u`,
        # hence the `[@]+` guard on both expansions below.
        echo "Running: cargo build --release --target=${TARGET} ${feature_args[*]+${feature_args[*]}}"
        echo "  RUSTFLAGS=${RUSTFLAGS}"
        cargo build --release --target="$TARGET" ${feature_args[@]+"${feature_args[@]}"}
    )
}

begin_step "Step 1: instrumented build"
INSTR_DIR="${TARGET_DIR}/instrumented"
rm -rf "$PROFILE_DIR"
mkdir -p "$PROFILE_DIR"
# The instrumented build must use the same LTO and codegen-unit settings as the
# final one. Inlining happens before instrumentation, so building it with LTO
# off yields counters for functions that no longer exist once the final build
# runs whole-program codegen, and the profile is then largely wasted on it.
#
# `--cfg pgo_training` makes the binary flush its own counters before exiting:
# it always leaves through `std::process::exit`, which on Windows is
# `ExitProcess` and skips the `atexit` handler the profiling runtime would
# otherwise write the profile from.
cargo_build "$INSTR_DIR" "-Cprofile-generate=${PROFILE_DIR} --cfg pgo_training"

BIN="${INSTR_DIR}/${TARGET}/release/coreutils${EXE}"
[ -x "$BIN" ] || { echo "instrumented binary not found: ${BIN}" >&2; exit 1; }
# Training runs the binary we just built, so a foreign target only works where
# the host can execute it (x86_64 on arm64 macOS needs Rosetta, for instance).
if ! "$BIN" true >/dev/null 2>&1; then
    echo "cannot run the instrumented ${TARGET} binary on this ${HOST} host" >&2
    exit 1
fi
end_step

begin_step "Step 2: corpus"
# Everything is generated from scratch: reading whatever /usr/share/dict/words
# or /etc/passwd happens to hold would make the profile machine-dependent.
export LLVM_PROFILE_FILE="${PROFILE_DIR}/coreutils-%p.profraw"
rm -rf "$CORPUS_DIR"
mkdir -p "$CORPUS_DIR"

WORDS="${CORPUS_DIR}/words.txt"
NUMBERS="${CORPUS_DIR}/numbers.txt"
REPEATED="${CORPUS_DIR}/repeated.txt"
PAIRS="${CORPUS_DIR}/pairs.txt"
BLOB="${CORPUS_DIR}/blob.bin"
COLUMNS="${CORPUS_DIR}/columns.txt"

# Built with bash's own printf rather than awk: git-bash has no awk, and the
# whole corpus is still generated in a couple of seconds.
# Shuffled by a stride so sort/uniq get unordered input rather than a no-op.
for ((i = 0; i < 200000; i++)); do printf 'w%06d\n' $(((i * 7919) % 200000)); done > "$WORDS"
for ((i = 0; i < 100000; i++)); do printf 'line%04d\n' $((i % 1000)); done > "$REPEATED"
for ((i = 0; i < 100000; i++)); do printf '%08d value%d\n' "$i" "$i"; done > "$PAIRS"
for ((i = 0; i < 2000; i++)); do printf 'user%d:x:%d:%d:User %d:/home/user%d:/bin/sh\n' "$i" $((1000 + i)) $((1000 + i)) "$i" "$i"; done > "$COLUMNS"
# seq/dd are part of what we want to profile, so use the instrumented binary.
"$BIN" seq 500000 > "$NUMBERS"
"$BIN" dd "if=${BIN}" "of=${BLOB}" bs=64K count=64 2>/dev/null
end_step

begin_step "Step 3: training workloads"
# Kept inside the target dir rather than under TMPDIR: git-bash may hand back a
# Windows-style TMPDIR that neither mktemp nor the workloads would agree on.
WORK="${TARGET_DIR}/work"
rm -rf "$WORK"
mkdir -p "$WORK"
trap 'rm -rf "$WORK"' EXIT

# Individual workloads are allowed to fail (a util may be absent from the
# selected feature set); a partial profile is still a usable profile.
run() { "$BIN" "$@" >/dev/null 2>&1 || true; }

# sort: the single hottest util, in its main modes
run sort "$WORDS" -o "${WORK}/sorted.txt"
run sort -n "$NUMBERS" -o "${WORK}/sorted-n.txt"
run sort -u "$REPEATED" -o "${WORK}/sorted-u.txt"
run sort -r "$WORDS" -o "${WORK}/sorted-r.txt"
run sort -k2,2 -t: "$COLUMNS" -o "${WORK}/sorted-k.txt"

# wc / cat / head / tail, to a regular file so the write path is not /dev/null
run wc -l "$WORDS"
run wc -w "$WORDS"
run wc -c "$BLOB"
run wc "$WORDS" "$NUMBERS"
"$BIN" cat "$WORDS" > "${WORK}/cat.txt" 2>/dev/null || true
"$BIN" cat -n "$WORDS" > "${WORK}/cat-n.txt" 2>/dev/null || true
"$BIN" head -n 50000 "$WORDS" > "${WORK}/head.txt" 2>/dev/null || true
"$BIN" tail -n 50000 "$WORDS" > "${WORK}/tail.txt" 2>/dev/null || true
run head -c 1M "$BLOB"

"$BIN" cat "$WORDS" | "$BIN" sort | "$BIN" uniq -c > "${WORK}/pipe1.txt" 2>/dev/null || true
"$BIN" seq 200000 | "$BIN" wc -l > /dev/null 2>&1 || true
"$BIN" cat "$BLOB" | "$BIN" sha256sum > /dev/null 2>&1 || true
"$BIN" sort -n "$NUMBERS" | "$BIN" tail -n 1000 | "$BIN" cut -c1-4 > "${WORK}/pipe2.txt" 2>/dev/null || true

# text utils
run uniq "$REPEATED"
run uniq -c "$REPEATED"
run uniq -u "$REPEATED"
run cut -d: -f1,3 "$COLUMNS"
run cut -c1-10 "$WORDS"
run nl "$WORDS"
run fold -w 40 "$WORDS"
run expand "$WORDS"
run unexpand "$WORDS"
run paste "$WORDS" "$WORDS"
run join "$PAIRS" "$PAIRS"
run join --nocheck-order "$PAIRS" "$PAIRS"
run split -l 20000 "$WORDS" "${WORK}/split-"
"$BIN" tr a-z A-Z < "$WORDS" > "${WORK}/tr.txt" 2>/dev/null || true
"$BIN" tr -d aeiou < "$WORDS" > "${WORK}/tr-d.txt" 2>/dev/null || true
"$BIN" tee "${WORK}/tee.txt" < "$NUMBERS" > /dev/null 2>&1 || true

# numbers / encoding / hashing
run seq 1000000
run seq 0 0.1 10000
"$BIN" base64 "$BLOB" > "${WORK}/encoded.b64" 2>/dev/null || true
run base64 -d "${WORK}/encoded.b64"
run cksum "$BLOB"
run md5sum "$BLOB"
run sha1sum "$BLOB"
run sha256sum "$BLOB"

# file management: cp/mv/rm/ls are as hot in practice as anything above
run cp "$WORDS" "${WORK}/copy.txt"
run cp -r "${REPO_ROOT}/src/uu/sort" "${WORK}/tree"
run mv "${WORK}/copy.txt" "${WORK}/moved.txt"
run mv "${WORK}/tree" "${WORK}/tree2"
run ls -la "${REPO_ROOT}/src/uu"
run ls -lR "${REPO_ROOT}/src/uu"
run ls --color=always -la "${REPO_ROOT}/src"
run du -sh "${REPO_ROOT}/src"
run df -h
run rm -rf "${WORK}/tree2"

echo "Workloads complete."
end_step

begin_step "Step 4: merging profiles"
shopt -s nullglob
RAW=("${PROFILE_DIR}"/coreutils-*.profraw)
shopt -u nullglob
if [ ${#RAW[@]} -eq 0 ]; then
    echo "no .profraw files found in ${PROFILE_DIR}" >&2
    exit 1
fi
echo "Merging ${#RAW[@]} profile(s)..."
"$LLVM_PROFDATA" merge -sparse "${RAW[@]}" -o "$MERGED"
echo "Merged profile: ${MERGED}"

# A profile that covers almost nothing still builds fine and silently produces a
# barely-optimized binary, so fail loudly instead: every workload in step 3 is
# allowed to fail individually, and without this a broken corpus would ship.
"$LLVM_PROFDATA" show "$MERGED" | head -6
COVERED="$("$LLVM_PROFDATA" show "$MERGED" | sed -n 's/^Total functions: *//p')"
MIN_FUNCTIONS=500
if [ -z "$COVERED" ] || [ "$COVERED" -lt "$MIN_FUNCTIONS" ]; then
    echo "profile covers only ${COVERED:-0} functions (expected >= ${MIN_FUNCTIONS})" >&2
    echo "the training workloads probably did not run; refusing to ship this profile" >&2
    # Distinguish "the workloads never ran" from "they ran but the counters were
    # never written": the first leaves no/short raw files, the second leaves
    # plenty of same-sized ones that merge down to zero functions.
    echo "--- raw profiles in ${PROFILE_DIR}:" >&2
    ls -l "$PROFILE_DIR" | head -12 >&2
    echo "--- first raw profile (${RAW[0]}):" >&2
    "$LLVM_PROFDATA" show "${RAW[0]}" >&2 || true
    echo "--- instrumented binary: ${BIN}" >&2
    ls -l "$BIN" >&2
    exit 1
fi
echo "Profile covers ${COVERED} functions."
end_step

# CI needs the absolute, natively-spelled path to put in RUSTFLAGS; it cannot
# reconstruct it portably because of the Windows path rewriting above.
printf '%s\n' "$MERGED" > "${TARGET_DIR}/profdata-path.txt"

if [ "$TRAIN_ONLY" -eq 1 ]; then
    echo
    echo "To use the profile in a release build, add to RUSTFLAGS:"
    echo "  -Cprofile-use=${MERGED}"
    echo "Total: $(fmt_duration $((SECONDS - SCRIPT_START)))"
    exit 0
fi

begin_step "Step 5: optimized build"
cargo_build "$TARGET_DIR" "-Cprofile-use=${MERGED}"
end_step
echo
echo "Optimized binary: ${TARGET_DIR}/${TARGET}/release/coreutils${EXE}"
echo "Total: $(fmt_duration $((SECONDS - SCRIPT_START)))"
