# Benchmarking ls

ls majorly involves fetching a lot of details (depending upon what details are requested, eg. time/date, inode details, etc) for each path using system calls. Ideally, any system call should be done only once for each of the paths - not adhering to this principle leads to a lot of system call overhead multiplying and bubbling up, especially for recursive ls, therefore it is important to always benchmark multiple scenarios.
This is an overview over what was benchmarked, and if you make changes to `ls`, you are encouraged to check
how performance was affected for the workloads listed below. Feel free to add other workloads to the
list that we should improve / make sure not to regress.

Run `cargo build --release` before benchmarking after you make a change!

## Simple recursive ls

-   Get a large tree, for example linux kernel source tree.
-   Benchmark simple recursive ls with hyperfine: `hyperfine --warmup 2 "target/release/coreutils ls -R tree > /dev/null"`.

## Recursive ls with all and long options

-   Same tree as above
-   Benchmark recursive ls with -al -R options with hyperfine: `hyperfine --warmup 2 "target/release/coreutils ls -al -R tree > /dev/null"`.

## Comparing with GNU ls

Hyperfine accepts multiple commands to run and will compare them. To compare performance with GNU ls
duplicate the string you passed to hyperfine but remove the `target/release/coreutils` bit from it.

Example: `hyperfine --warmup 2 "target/release/coreutils ls -al -R tree > /dev/null"` becomes
`hyperfine --warmup 2 "target/release/coreutils ls -al -R tree > /dev/null" "ls -al -R tree > /dev/null"`
(This assumes GNU ls is installed as `ls`)

This can also be used to compare with version of ls built before your changes to ensure your change does not regress this.

Here is a `bash` script for doing this comparison:
```bash
#!/bin/bash
cargo build --no-default-features --features ls --release
args="$@"
hyperfine "ls $args" "target/release/coreutils ls $args"
```

**Note**: No localization is currently implemented. This means that the comparison above is not really fair. We can fix this by setting `LC_ALL=C`, so GNU `ls` can ignore localization.

## Checking system call count

- Another thing to look at would be system calls count using strace (on linux) or equivalent on other operating systems.
- Example: `strace -c target/release/coreutils ls -al -R tree`

## Cargo Flamegraph

With Cargo Flamegraph you can easily make a flamegraph of `ls`:
```bash
cargo flamegraph --cmd coreutils -- ls [additional parameters]
```

However, if the `-R` option is given, the output becomes pretty much useless due to recursion. We can fix this by merging all the direct recursive calls with `uniq`, below is a `bash` script that does this.
```bash
#!/bin/bash
cargo build --release --no-default-features --features ls
perf record target/release/coreutils ls "$@"
perf script | uniq | inferno-collapse-perf | inferno-flamegraph > flamegraph.svg
```
