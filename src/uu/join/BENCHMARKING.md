# Benchmarking join

<!-- spell-checker:ignore (words) CSVs nocheck hotpaths -->

## Performance profile

The amount of time spent in which part of the code can vary depending on the files being joined and the flags used.
A benchmark with `-j` and `-i` shows the following time:

| Function/Method  | Fraction of Samples | Why? |
| ---------------- | ------------------- | ---- |
| `Line::new`      | 27% | Linear search for field separators, plus some vector operations. |
| `read_until`     | 22% | Mostly libc reading file contents, with a few vector operations to represent them. |
| `Input::compare` | 20% | ~2/3 making the keys lowercase, ~1/3 comparing them. |
| `print_fields`   | 11% | Writing to and flushing the buffer. |
| Other            | 20% | |
| libc             | 25% | I/O and memory allocation. |

More detailed profiles can be obtained via [flame graphs](https://github.com/flamegraph-rs/flamegraph):
```
cargo flamegraph --bin join --package uu_join -- file1 file2 > /dev/null
```
You may need to add the following lines to the top-level `Cargo.toml` to get full stack traces:
```
[profile.release]
debug = true
```

## How to benchmark

Benchmarking typically requires files large enough to ensure that the benchmark is not overwhelmed by background system noise; say, on the order of tens of MB.
While `join` operates on line-oriented data, and not properly formatted CSVs (e.g., `join` is not designed to accommodate escaped or quoted delimiters),
in practice many CSV datasets will function well after being sorted.

Like most of the utils, the recommended tool for benchmarking is [hyperfine](https://github.com/sharkdp/hyperfine).
To benchmark your changes:
 - checkout the main branch (without your changes), do a `--release` build, and back up the executable produced at `target/release/join`
 - checkout your working branch (with your changes), do a `--release` build
 - run
   ```
   hyperfine -w 5 "/path/to/main/branch/build/join file1 file2" "/path/to/working/branch/build/join file1 file2"
   ```
   - you'll likely need to add additional options to both commands, such as a field separator, or if you're benchmarking some particular behavior
   - you can also optionally benchmark against GNU's join

## What to benchmark

The following options can have a non-trivial impact on performance:
 - `-a`/`-v` if one of the two files has significantly more lines than the other
 - `-j`/`-1`/`-2` cause work to be done to grab the appropriate field
 - `-i` adds a call to `to_ascii_lowercase()` that adds some time for allocating and dropping memory for the lowercase key
 - `--nocheck-order` causes some calls of `Input::compare` to be skipped

The content of the files being joined has a very significant impact on the performance.
Things like how long each line is, how many fields there are, how long the key fields are, how many lines there are, how many lines can be joined, and how many lines each line can be joined with all change the behavior of the hotpaths.
