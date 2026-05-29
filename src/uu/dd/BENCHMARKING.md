<!-- spell-checker:ignore maximises -->

# Benchmarking dd

`dd` is a utility used for copying and converting files. It is often used for
writing directly to devices, such as when writing an `.iso` file directly to a
drive.

## Understanding dd

At the core, `dd` has a simple loop of operation. It reads in `blocksize` bytes
from an input, optionally performs a conversion on the bytes, and then writes
`blocksize` bytes to an output file.

In typical usage, the performance of `dd` is dominated by the speed at which it
can read or write to the filesystem. For those scenarios it is best to optimize
the blocksize for the performance of the devices being read/written to. Devices
typically have an optimal block size that they work best at, so for maximum
performance `dd` should be using a block size, or multiple of the block size,
that the underlying devices prefer.

For benchmarking `dd` itself we will use fast special files provided by the
operating system that work out of RAM, `/dev/zero` and `/dev/null`. This reduces
the time taken reading/writing files to a minimum and maximises the percentage
time we spend in the `dd` tool itself, but care still needs to be taken to
understand where we are benchmarking the `dd` tool and where we are just
benchmarking memory performance.

The main parameter to vary for a `dd` benchmark is the blocksize, but benchmarks
testing the conversions that are supported by `dd` could also be interesting.

`dd` has a convenient `count` argument, that will copy `count` blocks of data
from the input to the output, which is useful for benchmarking.

## Blocksize Benchmarks

When measuring the impact of blocksize on the throughput, we want to avoid
testing the startup time of `dd`. `dd` itself will give a report on the
throughput speed once complete, but it's probably better to use an external
tool, such as `hyperfine` to measure the performance.

Benchmarks should be sized so that they run for a handful of seconds at a
minimum to avoid measuring the startup time unnecessarily. The total time will
be roughly equivalent to the total bytes copied (`blocksize` x `count`).

Some useful invocations for testing would be the following:

```shell
hyperfine "./target/release/dd bs=4k count=1000000 < /dev/zero > /dev/null"
hyperfine "./target/release/dd bs=1M count=20000 < /dev/zero > /dev/null"
hyperfine "./target/release/dd bs=1G count=10 < /dev/zero > /dev/null"
```

Choosing what to benchmark depends greatly on what you want to measure.
Typically you would choose a small blocksize for measuring the performance of
`dd`, as that would maximize the overhead introduced by the `dd` tool. `dd`
typically does some set amount of work per block which only depends on the size
of the block if conversions are used.

As an example, <https://github.com/uutils/coreutils/pull/3600> made a change to
reuse the same buffer between block copies, avoiding the need to reallocate a
new block of memory for each copy. The impact of that change mostly had an
impact on large block size copies because those are the circumstances where the
memory performance dominated the total performance.
