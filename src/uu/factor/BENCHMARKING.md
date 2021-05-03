# Benchmarking `factor`

## Microbenchmarking deterministic functions

We currently use [`criterion`] to benchmark deterministic functions,
such as `gcd` and `table::factor`.

Those benchmarks can be simply executed with `cargo bench` as usual,
but may require a recent version of Rust, *i.e.* the project's minimum
supported version of Rust does not apply to the benchmarks.


However, µbenchmarks are by nature unstable: not only are they specific to
the hardware, operating system version, etc., but they are noisy and affected
by other tasks on the system (browser, compile jobs, etc.), which can cause
`criterion` to report spurious performance improvements and regressions.

This can be mitigated by getting as close to [idealised conditions][lemire]
as possible:
- minimize the amount of computation and I/O running concurrently to the
  benchmark, *i.e.* close your browser and IM clients, don't compile at the
  same time, etc. ;
- ensure the CPU's [frequency stays constant] during the benchmark ;
- [isolate a **physical** core], set it to `nohz_full`, and pin the benchmark
  to it, so it won't be preempted in the middle of a measurement ;
- disable ASLR by running `setarch -R cargo bench`, so we can compare results
  across multiple executions.  
  **TODO**: check this propagates to the benchmark process


[`criterion`]: https://bheisler.github.io/criterion.rs/book/index.html
[lemire]: https://lemire.me/blog/2018/01/16/microbenchmarking-calls-for-idealized-conditions/
[isolate a **physical** core]: https://pyperf.readthedocs.io/en/latest/system.html#isolate-cpus-on-linux
[frequency stays constant]: XXXTODO
