<!-- spell-checker:ignore tmpfs -->

# Benchmarking shuf

`shuf` is a simple utility, but there are at least two important cases
to benchmark: with and without repetition.

When benchmarking changes, make sure to always build with the `--release` flag.
You can compare with another branch by compiling on that branch and then
renaming the executable from `shuf` to `shuf.old`.

## Generate sample data

Sample input can be generated using `/dev/random`:

```shell
wget -O input.txt https://www.gutenberg.org/files/100/100-0.txt
```

To avoid distortions from IO, it is recommended to store input data in tmpfs.

## Without repetition

By default, `shuf` samples without repetition.

To benchmark only the randomization and not IO, we can pass the `-i` flag with
a range of numbers to randomly sample from. An example of a command that works
well for testing:

```shell
hyperfine --warmup 10 "target/release/shuf -i 0-10000000 > /dev/null"
```

To measure the time taken by shuffling an input file, the following command can
be used:

```shell
hyperfine --warmup 10 "target/release/shuf input.txt > /dev/null"
```

It is important to discard the output by redirecting it to `/dev/null`, since
otherwise, a substantial amount of time is added to write the output to the
filesystem.

## With repetition

When repetition is allowed, `shuf` works very differently under the hood, so it
should be benchmarked separately. In this case, we have to pass the `-n` flag or
the command will run forever. An example of a hyperfine command is

```shell
hyperfine --warmup 10 "target/release/shuf -r -n 10000000 -i 0-1000 > /dev/null"
```

## With huge interval ranges

When `shuf` runs with huge interval ranges, special care must be taken, so it
should be benchmarked separately also. An example of a hyperfine command is

```shell
hyperfine --warmup 10 "target/release/shuf -n 100 -i 1000-2000000000 > /dev/null"
```
