# Benchmarking sort

<!-- spell-checker:ignore (words) kbytes -->

Most of the time when sorting is spent comparing lines. The comparison functions however differ based
on which arguments are passed to `sort`, therefore it is important to always benchmark multiple scenarios.
This is an overview over what was benchmarked, and if you make changes to `sort`, you are encouraged to check
how performance was affected for the workloads listed below. Feel free to add other workloads to the
list that we should improve / make sure not to regress.

Run `cargo build --release` before benchmarking after you make a change!

## Sorting a wordlist

-   Get a wordlist, for example with [words](<https://en.wikipedia.org/wiki/Words_(Unix)>) on Linux. The exact wordlist
    doesn't matter for performance comparisons. In this example I'm using `/usr/share/dict/american-english` as the wordlist.
-   Shuffle the wordlist by running `sort -R /usr/share/dict/american-english > shuffled_wordlist.txt`.
-   Benchmark sorting the wordlist with hyperfine: `hyperfine "target/release/coreutils sort shuffled_wordlist.txt -o output.txt"`.

## Sorting a wordlist with ignore_case

-   Same wordlist as above
-   Benchmark sorting the wordlist ignoring the case with hyperfine: `hyperfine "target/release/coreutils sort shuffled_wordlist.txt -f -o output.txt"`.

## Sorting numbers

-   Generate a list of numbers: `seq 0 100000 | sort -R > shuffled_numbers.txt`.
-   Benchmark numeric sorting with hyperfine: `hyperfine "target/release/coreutils sort shuffled_numbers.txt -n -o output.txt"`.

## Sorting numbers with -g

-   Same list of numbers as above.
-   Benchmark numeric sorting with hyperfine: `hyperfine "target/release/coreutils sort shuffled_numbers.txt -g -o output.txt"`.

## Sorting numbers with SI prefixes

-   Generate a list of numbers:
    <details>
      <summary>Rust script</summary>

    ## Cargo.toml

    ```toml
    [dependencies]
    rand = "0.8.3"
    ```

    ## main.rs

    ```rust
    use rand::prelude::*;
    fn main() {
      let suffixes = ['k', 'K', 'M', 'G', 'T', 'P', 'E', 'Z', 'Y'];
      let mut rng = thread_rng();
      for _ in 0..100000 {
          println!(
              "{}{}",
              rng.gen_range(0..1000000),
              suffixes.choose(&mut rng).unwrap()
          )
      }
    }

    ```

    ## running

    `cargo run > shuffled_numbers_si.txt`

    </details>

-   Benchmark numeric sorting with hyperfine: `hyperfine "target/release/coreutils sort shuffled_numbers_si.txt -h -o output.txt"`.

## External sorting

Try running commands with the `-S` option set to an amount of memory to be used, such as `1M`. Additionally, you could try sorting
huge files (ideally multiple Gigabytes) with `-S` (or without `-S` to benchmark with our default value).
Creating such a large file can be achieved by running `cat shuffled_wordlist.txt | sort -R >> shuffled_wordlist.txt`
multiple times (this will add the contents of `shuffled_wordlist.txt` to itself).
Example: Run `hyperfine './target/release/coreutils sort shuffled_wordlist.txt -S 1M' 'sort shuffled_wordlist.txt -S 1M'`

## Merging

"Merge" sort merges already sorted files. It is a sub-step of external sorting, so benchmarking it separately may be helpful.

-   Splitting `shuffled_wordlist.txt` can be achieved by running `split shuffled_wordlist.txt shuffled_wordlist_slice_ --additional-suffix=.txt`
-   Sort each part by running `for f in shuffled_wordlist_slice_*; do sort $f -o $f; done`
-   Benchmark merging by running `hyperfine "target/release/coreutils sort -m shuffled_wordlist_slice_*"`

## Check

When invoked with -c, we simply check if the input is already ordered. The input for benchmarking should be an already sorted file.

-   Benchmark checking by running `hyperfine "target/release/coreutils sort -c sorted_wordlist.txt"`

## Stdout and stdin performance

Try to run the above benchmarks by piping the input through stdin (standard input) and redirect the
output through stdout (standard output):

-   Remove the input file from the arguments and add `cat [input_file] | ` at the beginning.
-   Remove `-o output.txt` and add `> output.txt` at the end.

Example: `hyperfine "target/release/coreutils sort shuffled_numbers.txt -n -o output.txt"` becomes
`hyperfine "cat shuffled_numbers.txt | target/release/coreutils sort -n > output.txt`

-   Check that performance is similar to the original benchmark.

## Comparing with GNU sort

Hyperfine accepts multiple commands to run and will compare them. To compare performance with GNU sort
duplicate the string you passed to hyperfine but remove the `target/release/coreutils` bit from it.

Example: `hyperfine "target/release/coreutils sort shuffled_numbers_si.txt -h -o output.txt"` becomes
`hyperfine "target/release/coreutils sort shuffled_numbers_si.txt -h -o output.txt" "sort shuffled_numbers_si.txt -h -o output.txt"`
(This assumes GNU sort is installed as `sort`)

## Memory and CPU usage

The above benchmarks use hyperfine to measure the speed of sorting. There are however other useful metrics to determine overall
resource usage. One way to measure them is the `time` command. This is not to be confused with the `time` that is built in to the bash shell.
You may have to install `time` first, then you have to run it with `/bin/time -v` to give it precedence over the built in `time`.

<details>
      <summary>Example output</summary>

      Command being timed: "target/release/coreutils sort shuffled_numbers.txt"
      User time (seconds): 0.10
      System time (seconds): 0.00
      Percent of CPU this job got: 365%
      Elapsed (wall clock) time (h:mm:ss or m:ss): 0:00.02
      Average shared text size (kbytes): 0
      Average unshared data size (kbytes): 0
      Average stack size (kbytes): 0
      Average total size (kbytes): 0
      Maximum resident set size (kbytes): 25360
      Average resident set size (kbytes): 0
      Major (requiring I/O) page faults: 0
      Minor (reclaiming a frame) page faults: 5802
      Voluntary context switches: 462
      Involuntary context switches: 73
      Swaps: 0
      File system inputs: 1184
      File system outputs: 0
      Socket messages sent: 0
      Socket messages received: 0
      Signals delivered: 0
      Page size (bytes): 4096
      Exit status: 0

</details>

Useful metrics to look at could be:

-   User time
-   Percent of CPU this job got
-   Maximum resident set size
