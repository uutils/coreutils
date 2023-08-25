#!/bin/bash

# Exit on any failures
set +x

cargo build --no-default-features --features rm --release
test_dir="$1"
hyperfine --prepare "cp -r $test_dir tmp_d"  "rm -rf tmp_d" "target/release/coreutils rm -rf tmp_d"
