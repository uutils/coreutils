# Benchmarking `factor`

<!-- spell-checker:ignore (names) Daniel Lemire * Lemire's ; (misc) nohz -->

The benchmarks for `factor` are in
[`benches/factor_bench.rs`](benches/factor_bench.rs), inside the `uu_factor`
crate. They use [Divan], through the workspace's `divan` dependency on
`codspeed-divan-compat`, for local measurements and [CodSpeed] integration.

## Running benchmarks

Run these commands from the repository root:

```bash
# List the available benchmarks.
cargo bench -p uu_factor --bench factor_bench -- --list

# Run one benchmark by name.
cargo bench -p uu_factor --bench factor_bench factor_large_u64_prime
```

Omit the benchmark name to run the whole suite. Pass `-- --help` to see
Divan's options for filtering and sampling.

The current benchmarks call `uu_factor::uumain` through
`uucore::benchmark::run_util_function`. They measure argument handling and
output as well as factorization, rather than calling internal helpers in
isolation.

## Interpreting local measurements

Benchmarks are by nature unstable: not only are they specific to
the hardware, operating system version, etc., but they are noisy and affected
by other tasks on the system (browser, compile jobs, etc.). These effects can
look like performance improvements or regressions.

This can be mitigated by getting as close to [idealized conditions][lemire]
as possible:

- minimize the amount of computation and I/O running concurrently to the
  benchmark, *i.e.* close your browser and IM clients, don't compile at the
  same time, etc. ;
- keep the CPU's frequency constant during the benchmark ;
- [isolate a **physical** core], set it to `nohz_full`, and pin the benchmark
  to it, so it won't be preempted in the middle of a measurement ;
- where supported, disable ASLR with `setarch -R` when comparing multiple
  executions of the benchmark command.

[Divan]: https://docs.rs/divan/latest/divan/
[CodSpeed]: https://codspeed.io/
[lemire]: https://lemire.me/blog/2018/01/16/microbenchmarking-calls-for-idealized-conditions/
[isolate a **physical** core]: https://pyperf.readthedocs.io/en/latest/system.html#isolate-cpus-on-linux

## Guidance for designing benchmarks

Follow the existing `#[divan::bench]` functions, using `Bencher::bench` to
measure the operation and `divan::black_box` where needed. The shared
benchmark helpers are described in the project's
[performance guide](../../../docs/src/performance.md#writing-benchmarks).

1. Keep setup outside the measured operation.

   Prepare input data before `bencher.bench` unless its construction is part
   of what the benchmark is intended to measure.

1. Benchmarks are immutable (once merged in `uutils`)

   Modifying a benchmark means previously-collected values cannot meaningfully
   be compared, silently giving nonsensical results.  If you must modify an
   existing benchmark, rename it.

1. Test common cases

   Use fixed or reproducibly generated inputs so revisions run the same work.
   The suite includes primes, a semiprime, prime powers, and products of
   several primes to exercise different factorization paths.

## CI performance testing

The [benchmark workflow](../../../.github/workflows/benchmarks.yml) runs the
integrated benchmarks with CodSpeed's simulation and memory modes. See the
[performance guide](../../../docs/src/performance.md#running-benchmarks)
for local CodSpeed commands.

## Comparing randomized implementations across multiple inputs

`factor` is a challenging target for system benchmarks as it combines two
characteristics:

1. integer factoring algorithms are randomized, with large variance in
   execution time ;

1. various inputs also have large differences in factoring time, that
   corresponds to no natural, linear ordering of the inputs.

If (1) was untrue (i.e. if execution time wasn't random), we could faithfully
compare 2 implementations (2 successive versions, or `uutils` and GNU) using
a scatter plot, where each axis corresponds to the perf. of one implementation.

Similarly, without (2) we could plot numbers on the X axis and their factoring
time on the Y axis, using multiple lines for various quantiles.  The large
differences in factoring times for successive numbers, mean that such a plot
would be unreadable.
