# Benchmarking `factor`

## Microbenchmarking deterministic functions

We currently use [`criterion`] to benchmark deterministic functions,
such as `gcd` and `table::factor`.

Those benchmarks can be simply executed with `cargo bench` as usual,
but may require a recent version of Rust, *i.e.* the project's minimum
supported version of Rust does not apply to the benchmarks.

[`criterion`]: https://bheisler.github.io/criterion.rs/book/index.html
