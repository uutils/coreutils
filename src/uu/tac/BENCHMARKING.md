## Benchmarking `tac`

<!-- spell-checker:ignore wikidatawiki -->

`tac` is often used to process log files in reverse chronological order, i.e. from newer towards older entries. In this case, the performance target to yield results as fast as possible, i.e. without reading in the whole file that is to be reversed line-by-line. Therefore, a sensible benchmark is to read a large log file containing N lines and measure how long it takes to produce the last K lines from that file.

Large text files can for example be found in the [Wikipedia database dumps](https://dumps.wikimedia.org/wikidatawiki/latest/), usually sized at multiple gigabytes and comprising more than 100M lines.

After you have obtained and uncompressed such a file, you need to build `tac` in release mode

```shell
$ cargo build --release --package uu_tac
```

and then you can time how it long it takes to extract the last 10M lines by running

```shell
$ /usr/bin/time ./target/release/tac wikidatawiki-20211001-pages-logging.xml | head -n10000000 >/dev/null
```

For more systematic measurements that include warm-ups, repetitions and comparisons, [Hyperfine](https://github.com/sharkdp/hyperfine) can be helpful. For example, to compare this implementation to the one provided by your distribution run

```shell
$ hyperfine "./target/release/tac wikidatawiki-20211001-pages-logging.xml | head -n10000000 >/dev/null" "/usr/bin/tac wikidatawiki-20211001-pages-logging.xml | head -n10000000 >/dev/null"
```
