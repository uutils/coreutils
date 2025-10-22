# Benchmarking `tr`

<!-- spell-checker:ignore aeiou -->

`tr` performance is critical for large data processing pipelines. The implementation uses lookup tables for O(1) character operations.

## Building

```shell
cargo build --release --package uu_tr
```

## Basic Benchmarks

### Character Translation

```shell
# Create test data
dd if=/dev/zero bs=1M count=10 | tr '\0' 'x' > 10mb_input

# Benchmark translation
hyperfine --warmup 3 \
    'tr a b < 10mb_input > /dev/null' \
    './target/release/tr a b < 10mb_input > /dev/null'
```

### Character Deletion

```shell
# Create mixed input
yes "abcdefghijklmnopqrstuvwxyz" | head -n 1000000 > mixed_input

# Benchmark deletion
hyperfine --warmup 3 \
    'tr -d aeiou < mixed_input > /dev/null' \
    './target/release/tr -d aeiou < mixed_input > /dev/null'
```

### Character Classes

```shell
# Benchmark character classes
hyperfine --warmup 3 \
    'tr "[:lower:]" "[:upper:]" < mixed_input > /dev/null' \
    './target/release/tr "[:lower:]" "[:upper:]" < mixed_input > /dev/null'
```

## Quick Regression Test

```shell
#!/bin/bash
cargo build --release --package uu_tr

echo "=== Translation ==="
hyperfine 'tr a b < 10mb_input > /dev/null' './target/release/tr a b < 10mb_input > /dev/null'

echo "=== Deletion ==="
hyperfine 'tr -d aeiou < mixed_input > /dev/null' './target/release/tr -d aeiou < mixed_input > /dev/null'
```

## Performance Notes

- Uses lookup tables instead of hash maps for O(1) operations
- 32KB I/O buffers for improved throughput
- Should be competitive with GNU `tr` for most operations

