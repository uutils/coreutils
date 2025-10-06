<!-- spell-checker:ignore taskset -->

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
