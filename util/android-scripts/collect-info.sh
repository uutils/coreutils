#!/bin/bash

# spell-checker:ignore nextest watchplus PIPESTATUS

echo "system resources - RAM:"
free -hm
echo "system resources - CPU:"
lscpu
echo "system resources - file systems:"
mount

echo "$HOME"
PATH=$HOME/.cargo/bin:$PATH
export PATH
echo "$PATH"
pwd
command -v rustc && rustc -Vv
ls -la ~/.cargo/bin
cargo --list
cargo nextest --version
