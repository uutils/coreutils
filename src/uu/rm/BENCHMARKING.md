<!-- spell-checker:ignore samply flamegraph flamegraphs -->
# Benchmarking rm

Run `cargo build --release` before benchmarking after you make a change!

## Simple recursive rm

- Get a large tree, for example linux kernel source tree.
- We'll need to pass a `--prepare` argument, since `rm` deletes the dir each time.
- Benchmark simple recursive rm with hyperfine: `hyperfine --prepare "cp -r tree tree-tmp" "target/release/coreutils rm -r tree-tmp"`.

## Comparing with GNU rm

Hyperfine accepts multiple commands to run and will compare them. To compare performance with GNU rm
duplicate the string you passed to hyperfine but remove the `target/release/coreutils` bit from it.

Example: `hyperfine --prepare "cp -r tree tree-tmp" "target/release/coreutils rm -rf tree-tmp"` becomes
`hyperfine --prepare "cp -r tree tree-tmp" "target/release/coreutils rm -rf tree-tmp" "rm -rf tree-tmp"`
(This assumes GNU rm is installed as `rm`)

This can also be used to compare with version of rm built before your changes to ensure your change does not regress this.

Here is a `bash` script for doing this comparison:

```shell
#!/bin/bash
cargo build --no-default-features --features rm --release
test_dir="$1"
hyperfine --prepare "cp -r $test_dir tmp_d"  "rm -rf tmp_d" "target/release/coreutils rm -rf tmp_d"
```

## Checking system call count

- Another thing to look at would be system calls count using strace (on linux) or equivalent on other operating systems.
- Example: `strace -c target/release/coreutils rm -rf tree`

## Flamegraph

### samply

[samply](https://github.com/mstange/samply) is one option for simply creating flamegraphs. It uses the Firefox profiler as a UI.

To install:
```bash
cargo install samply
```

To run:

```bash
samply record target/release/coreutils rm -rf ../linux
```

### Cargo Flamegraph

With Cargo Flamegraph you can easily make a flamegraph of `rm`:

```shell
cargo flamegraph --cmd coreutils -- rm [additional parameters]
```

