<!-- spell-checker:ignore taskset usize -->

# Performance Profiling Tutorial

## Effective Benchmarking with Hyperfine

[Hyperfine](https://github.com/sharkdp/hyperfine) is a powerful command-line benchmarking tool that allows you to measure and compare execution times of commands with statistical rigor.

### Benchmarking Best Practices

When evaluating performance improvements, always set up your benchmarks to compare:

1. The GNU implementation as reference
2. The implementation without the change
3. The implementation with your change

This three-way comparison provides clear insights into:
- How your implementation compares to the standard (GNU)
- The actual performance impact of your specific change

### Example Benchmark

First, you will need to build the binary in release mode. Debug builds are significantly slower:

```bash
cargo build --features unix --profile profiling
```

```bash
# Three-way comparison benchmark
hyperfine \
  --warmup 3 \
  "/usr/bin/ls -R ." \
  "./target/profiling/coreutils.prev ls -R ." \
  "./target/profiling/coreutils ls -R ."

# can be simplified with:
hyperfine \
  --warmup 3 \
  -L ls /usr/bin/ls,"./target/profiling/coreutils.prev ls","./target/profiling/coreutils ls" \
  "{ls} -R ."
```

For Ubuntu 25.10 and other distributions that use uutils by default, replace `bin/ls` with `bin/gnuls`. Also:

```
# to improve the reproducibility of the results:
taskset -c 0
```

### Interpreting Results

Hyperfine provides summary statistics including:
- Mean execution time
- Standard deviation
- Min/max times
- Relative performance comparison

Look for consistent patterns rather than focusing on individual runs, and be aware of system noise that might affect results.

## Integrated Benchmarking

Utilities include integrated benchmarks in `src/uu/*/benches/*` using [CodSpeed](https://codspeed.io/) and [Divan](https://github.com/nvzqz/divan).

**Important**: Before starting performance optimization work, you should add a benchmark for the utility. This provides a baseline for measuring improvements and ensures changes have measurable impact.

### Running Benchmarks

```bash
# Build and run benchmarks for a specific utility
cargo codspeed build -p uu_expand
cargo codspeed run -p uu_expand
```

### Writing Benchmarks

Use common functions from `src/uucore/src/lib/features/benchmark.rs`:

```rust
use divan::{Bencher, black_box};
use uu_expand::uumain;
use uucore::benchmark::{create_test_file, run_util_function, text_data};

#[divan::bench(args = [10_000, 100_000])]
fn bench_expand(bencher: Bencher, num_lines: usize) {
    let data = text_data::generate_ascii_data(num_lines);
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = create_test_file(&data, temp_dir.path());

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[file_path.to_str().unwrap()]));
    });
}

fn main() {
    divan::main();
}
```

Common helpers include `text_data::generate_*()` for test data and `fs_tree::create_*()` for directory structures.

## Using Samply for Profiling

[Samply](https://github.com/mstange/samply) is a sampling profiler that helps you identify performance bottlenecks in your code.

### Basic Profiling

```bash
# Generate a flame graph for your application
samply record ./target/debug/coreutils ls -R

# Profile with higher sampling frequency
samply record --rate 1000 ./target/debug/coreutils seq 1 1000
```

The output using the `debug` profile might be easier to understand, but the performance characteristics may be somewhat different from `release` profile that we _actually_ care about.

Consider using the `profiling` profile, that compiles in `release` mode but with debug symbols. For example:
```bash
cargo build --profile profiling -p uu_ls
samply record -r 10000 target/profiling/ls -lR /var .git .git .git > /dev/null
```

## Workflow: Measuring Performance Improvements

1. **Establish baselines**:
   ```bash
   hyperfine --warmup 3 \
     "/usr/bin/sort large_file.txt" \
     "our-sort-v1 large_file.txt"
   ```

2. **Identify bottlenecks**:
   ```bash
   samply record ./our-sort-v1 large_file.txt
   ```

3. **Make targeted improvements** based on profiling data

4. **Verify improvements**:
   ```bash
   hyperfine --warmup 3 \
     "/usr/bin/sort large_file.txt" \
     "our-sort-v1 large_file.txt" \
     "our-sort-v2 large_file.txt"
   ```

5. **Document performance changes** with concrete numbers
   ```bash
   hyperfine --export-markdown file.md [...]
   ```

## Profile-Guided Optimization

Our released binaries are built with PGO, driven by `util/build-pgo.sh` (Linux,
macOS and Windows alike). When comparing against a release build, keep in mind
that a plain `cargo build --release` is *not* what we ship. To reproduce it:

```bash
rustup component add llvm-tools
./util/build-pgo.sh --features unix
# -> target/coreutils-pgo/<target>/release/coreutils
```

See [packaging](packaging.md#profile-guided-optimization-pgo) for the details
and the options.

How much it buys varies a lot by utility. Measured on x86_64 with `hyperfine`
against a plain `cargo build --release`, the text-processing utilities gain the
most, while utilities dominated by syscalls or by a single hand-tuned loop
barely move:

| Workload | Change |
| -------- | ------ |
| `cat -n`, `uniq -c`, `nl`, `fold -w` | -22% to -29% |
| `sort`, `sort -n` | -17% |
| `wc`, `sort -k` | -11% to -15% |
| `ls -lR`, `head` | -4% to -8% |
| `cut`, `seq`, `sha256sum`, `base64`, process startup | no change |

Two things to keep in mind when benchmarking a PGO build:

- The workloads in `util/build-pgo.sh` are what the profile is trained on. A
  utility or a mode that is not exercised there gets no benefit, and measuring
  one tells you little about the utilities that are.
- Wall-clock and instruction counts can disagree, since PGO changes code layout
  as well as the code itself. `wc` gets 15% faster while executing *more*
  instructions. If you use `valgrind --tool=cachegrind` for a noise-free
  comparison, confirm the result with `hyperfine` before believing it.
