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
hyperfine "seq 1000000" "target/release/seq 1000000"
```

[0]: https://github.com/sharkdp/hyperfine
