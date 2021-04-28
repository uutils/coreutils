## Benchmarking cut

### Performance profile

In normal use cases a significant amount of the total execution time of `cut`
is spent performing I/O. When invoked with the `-f` option (cut fields) some
CPU time is spent on detecting fields (in `Searcher::next`). Other than that
some small amount of CPU time is spent on breaking the input stream into lines.


### How to

When fixing bugs or adding features you might want to compare
performance before and after your code changes.

-   `hyperfine` can be used to accurately measure and compare the total
    execution time of one or more commands.

    ```
    $ cargo build --release --package uu_cut

    $ hyperfine -w3 "./target/release/cut -f2-4,8 -d' ' input.txt" "cut -f2-4,8 -d' ' input.txt"
    ```
    You can put those two commands in a shell script to be sure that you don't
    forget to build after making any changes.

When optimizing or fixing performance regressions seeing the number of times a
function is called, and the amount of time it takes can be useful.

-   `cargo flamegraph` generates flame graphs from function level metrics it records using `perf` or `dtrace`

    ```
    $ cargo flamegraph --bin cut --package uu_cut -- -f1,3-4 input.txt > /dev/null
    ```


### What to benchmark

There are four different performance paths in `cut` to benchmark.

-   Byte ranges `-c`/`--characters` or `-b`/`--bytes` e.g. `cut -c 2,4,6-`
-   Byte ranges with output delimiters e.g. `cut -c 4- --output-delimiter=/`
-   Fields e.g. `cut -f -4`
-   Fields with output delimiters e.g. `cut -f 7-10 --output-delimiter=:`

Choose a test input file with large number of lines so that program startup time does not significantly affect the benchmark.
