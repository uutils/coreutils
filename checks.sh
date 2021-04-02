#!/usr/bin/env bash

echo "[uutils/coreutils] Runnning formatters for you..."

cargo fmt
find tests -name "*.rs" -print0 | xargs -0 cargo fmt

echo "[uutils/coreutils] Formatters ran successfully, starting test suite in:"
echo "[uutils/coreutils] 5..."
sleep 1
echo "[uutils/coreutils] 4..."
sleep 1
echo "[uutils/coreutils] 3..."
sleep 1
echo "[uutils/coreutils] 2..."
sleep 1
echo "[uutils/coreutils] 1..."
sleep 1
echo "[uutils/coreutils] Runnning the test suite for you..."

if [ -z "$1" ]
  then
    echo "[uutils/coreutils] No argument supplied, running full test suite"
    cargo check  || { echo "[uutils/coreutils] 'cargo check' failed to compile your code" ; exit 1; }
    cargo test || { echo "[uutils/coreutils] 'cargo test' failed, some tests are failing" ; exit 1; }
    cargo clippy || { echo "[uutils/coreutils] 'cargo clippy' failed, there are lint errors" ; exit 1; }
  else
    echo -e "[uutils/coreutils] Supplied '\e[4m$1\e[0m', running partial test suite"
    cargo check --features "$1" --no-default-features  || { echo "[uutils/coreutils] 'cargo check' failed to compile your code" ; exit 1; }
    cargo test --features "$1" --no-default-features || { echo "[uutils/coreutils] 'cargo test' failed, some tests are failing" ; exit 1; }
    cargo clippy --features "$1" --no-default-features || { echo "[uutils/coreutils] 'cargo clippy' failed, there are lint errors" ; exit 1; }
fi

echo -e "[uutils/coreutils] 🎉 If you see this message, your code \e[3mshould\e[0m pass ci 🎉"
