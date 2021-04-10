# Benchmarking sort

Most of the time when sorting is spent comparing lines. The comparison functions however differ based
on which arguments are passed to `sort`, therefore it is important to always benchmark multiple scenarios.
This is an overwiew over what was benchmarked, and if you make changes to `sort`, you are encouraged to check
how performance was affected for the workloads listed below. Feel free to add other workloads to the
list that we should improve / make sure not to regress.

Run `cargo build --release` before benchmarking after you make a change!

## Sorting a wordlist
- Get a wordlist, for example with [words](https://en.wikipedia.org/wiki/Words_(Unix)) on Linux. The exact wordlist
  doesn't matter for performance comparisons. In this example I'm using `/usr/share/dict/american-english` as the wordlist.
- Shuffle the wordlist by running `sort -R /usr/share/dict/american-english > shuffled_wordlist.txt`.
- Benchmark sorting the wordlist with hyperfine: `hyperfine "target/release/coreutils sort shuffled_wordlist.txt -o output.txt"`.

## Sorting a wordlist with ignore_case
- Same wordlist as above
- Benchmark sorting the wordlist ignoring the case with hyperfine: `hyperfine "target/release/coreutils sort shuffled_wordlist.txt -f -o output.txt"`.

## Sorting numbers
- Generate a list of numbers: `seq 0 100000 | sort -R > shuffled_numbers.txt`.
- Benchmark numeric sorting with hyperfine: `hyperfine "target/release/coreutils sort shuffled_numbers.txt -n -o output.txt"`.

## Stdout and stdin performance
Try to run the above benchmarks by piping the input through stdin (standard input) and redirect the
output through stdout (standard output):
- Remove the input file from the arguments and add `cat [inputfile] | ` at the beginning.
- Remove `-o output.txt` and add `> output.txt` at the end.

Example: `hyperfine "target/release/coreutils sort shuffled_numbers.txt -n -o output.txt"` becomes
`hyperfine "cat shuffled_numbers.txt | target/release/coreutils sort -n > output.txt`
- Check that performance is similar to the original benchmark.