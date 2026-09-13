# Benchmarking to measure performance

To compare the performance of the `uutils` version of `seq` with the
GNU version of `seq`, you can use a benchmarking tool like
[hyperfine][0]. On Ubuntu 18.04 or later, you can install `hyperfine` by
running

```shell
sudo apt-get install hyperfine
```

Next, build the `seq` binary under the release profile:

```shell
cargo build --release -p uu_seq
```

Finally, you can compare the performance of the two versions of `seq`
by running, for example,

```shell
hyperfine -L seq seq,target/release/seq "{seq} 1000000"
```

## Interesting test cases

Performance characteristics may vary a lot depending on the parameters,
and if custom formatting is required. In particular, it does appear
that the GNU implementation is heavily optimized for positive integer
outputs (which is probably the most common use case for `seq`).

Specifying a format or fixed width will slow down the
execution a lot (~15-20 times on GNU `seq`):
```shell
hyperfine -L seq seq,target/release/seq "{seq} -f%g 1000000"
hyperfine -L seq seq,target/release/seq "{seq} -w 1000000"
```

Floating point increments, or any negative bound, also degrades the
performance (~10-15 times on GNU `seq`):
```shell
hyperfine -L seq seq,./target/release/seq "{seq} 0 0.000001 1"
hyperfine -L seq seq,./target/release/seq "{seq} -100 1 1000000"
```

It is also interesting to compare performance with large precision
format. But in this case, the output itself should also be compared,
as GNU `seq` may not provide the same precision (`uutils` version of
`seq` provides arbitrary precision, while GNU `seq` appears to be
limited to `long double` on the given platform, i.e. 64/80/128-bit
float):
```shell
hyperfine -L seq seq,target/release/seq "{seq} -f%.30f 0 0.000001 1"
```

## Optimizations

### Buffering stdout

The original `uutils` implementation of `seq` did unbuffered writes
to stdout, causing a large number of system calls (and therefore a large amount
of system time). Simply wrapping `stdout` in a `BufWriter` increased performance
by about 2 times for a floating point increment test case, leading to similar
performance compared with GNU `seq`.

### Directly print strings

As expected, directly printing a string:
```rust
stdout.write_all(separator.as_bytes())?
```
is quite a bit faster than using format to do the same operation:
```rust
write!(stdout, "{separator}")?
```

The change above resulted in a ~10% speedup.

### Fast increment path

When dealing with positive integer values (first/increment/last), and
the default format is used, we use a custom fast path that does arithmetic
on u8 arrays (i.e. strings), instead of repeatedly calling into
formatting format.

This provides _massive_ performance gains, in the order of 10-20x compared
with the default implementation, at the expense of some added code complexity.

Just from performance numbers, it is clear that GNU `seq` uses similar
tricks, but we are more liberal on when we use our fast path (e.g. large
increments are supported, equal width is supported). Our fast path
implementation gets within ~10% of `seq` performance when its fast
path is activated.

[0]: https://github.com/sharkdp/hyperfine
