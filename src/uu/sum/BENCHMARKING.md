# Benchmarking `sum`

<!-- spell-checker:ignore wikidatawiki -->

Large sample files can for example be found in the [Wikipedia database dumps](https://dumps.wikimedia.org/wikidatawiki/latest/), usually sized at multiple gigabytes and comprising more than 100M lines.

After you have obtained and uncompressed such a file, you need to build `sum` in release mode

```shell
cargo build --release --package uu_sum
```

and then you can time how it long it takes to checksum the file by running

```shell
time ./target/release/sum wikidatawiki-20211001-pages-logging.xml
```

For more systematic measurements that include warm-ups, repetitions and comparisons, [Hyperfine](https://github.com/sharkdp/hyperfine) can be helpful. For example, to compare this implementation to the one provided by your distribution run

```shell
hyperfine "./target/release/sum wikidatawiki-20211001-pages-logging.xml" "sum wikidatawiki-20211001-pages-logging.xml"
```
