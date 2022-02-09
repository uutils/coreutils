# Benchmarking shuf

`shuf` is a simple utility, but there are at least two important cases
benchmark: with and without repetition.

When benchmarking changes, make sure to always build with the `--release` flag.
You can compare with another branch by compiling on that branch and than
renaming the executable from `shuf` to `shuf.old`.

## Without repetition

By default, `shuf` samples without repetition. To benchmark only the
randomization and not IO, we can pass the `-i` flag with a range of numbers to
randomly sample from. An example of a command that works well for testing:

```shell
hyperfine --warmup 10 "target/release/shuf -i 0-10000000"
```

## With repetition

When repetition is allowed, `shuf` works very differently under the hood, so it
should be benchmarked separately. In this case we have to pass the `-n` flag or
the command will run forever. An example of a hyperfine command is

```shell
hyperfine --warmup 10 "target/release/shuf -r -n 10000000 -i 0-1000"
```
