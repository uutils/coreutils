## Benchmarking hashsum

### To bench blake2

Taken from: https://github.com/uutils/coreutils/pull/2296

With a large file:
$ hyperfine "./target/release/coreutils hashsum --b2sum large-file" "b2sum large-file"

