#!/usr/bin/env bash
# requires stage1 rustc built from https://github.com/victor-prokhorov/rust/commit/a9d5767288d132bc3688799ad45c4647b7043f7d
# which is a clone of https://github.com/rust-lang/rust/pull/78515
set -euo pipefail

COREUTILS_DIR=/home/victorprokhorov/coreutils
RUST_DIR=/home/victorprokhorov/rust-lang/rust
GNU_DIR=/home/victorprokhorov/gnu
STAGE1_SYSROOT_LIB="$RUST_DIR/build/aarch64-unknown-linux-gnu/stage1/lib/rustlib/aarch64-unknown-linux-gnu/lib"
STAGE1_RUSTC="$RUST_DIR/build/aarch64-unknown-linux-gnu/stage1/bin/rustc"

ok()   { echo "  OK  $*"; }
fail() { echo "FAIL $*"; exit 1; }
step() { echo; echo "$*"; }

step "build stdlib"
cd "$RUST_DIR"
python3 x.py build library
ok "stdlib built"

step "toolchain check"
rustup run stage1 rustc --version | grep -q 'rustc' \
  || fail "stage1 toolchain not found"
ok "$(rustup run stage1 rustc --version)"
rustup run stage1 rustc --edition 2021 -C prefer-dynamic \
  -o /tmp/poc_api_check - <<'RUST' \
  || fail "stdout_switchable_buffering or set_buffer_mode not found in stage1 stdlib"
#![feature(stdout_switchable_buffering)]
use std::io::{self, BufferMode};
fn main() { io::stdout().lock().set_buffer_mode(BufferMode::Line); }
RUST
ok "stage1 stdlib has stdout_switchable_buffering and set_buffer_mode"

step "cargo +stage1 build"
cd "$COREUTILS_DIR"
RUSTC="$STAGE1_RUSTC" cargo build -p uu_stdbuf_libstdbuf -p uu_stdbuf -p uu_uniq

step "block buffering test all lines must appear together at the end"
rustup run stage1 rustc --edition 2021 -C prefer-dynamic \
  -o /tmp/poc_block_test - <<'RUST' || fail "compile poc_block_test"
#![feature(stdout_switchable_buffering)]
use std::io::{self, BufferMode};
fn main() {
    io::stdout().lock().set_buffer_mode(BufferMode::Block);
    for line in ["1", "2", "3", "soleil"] {
        println!("{line}");
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
RUST
LD_LIBRARY_PATH="$STAGE1_SYSROOT_LIB" /tmp/poc_block_test

step "GNU compliance tests/misc/stdbuf.sh"
cd "$COREUTILS_DIR"
path_UUTILS="$COREUTILS_DIR" path_GNU="$GNU_DIR" PROFILE=debug \
  util/run-gnu-test.sh tests/misc/stdbuf.sh
