# uutils coreutils

[![Crates.io](https://img.shields.io/crates/v/coreutils.svg)](https://crates.io/crates/coreutils)
[![Discord](https://img.shields.io/badge/discord-join-7289DA.svg?logo=discord&longCache=true&style=flat)](https://discord.gg/wQVJbvJ)
[![License](http://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/uutils/coreutils/blob/main/LICENSE)
[![LOC](https://tokei.rs/b1/github/uutils/coreutils?category=code)](https://github.com/Aaronepower/tokei)
[![dependency status](https://deps.rs/repo/github/uutils/coreutils/status.svg)](https://deps.rs/repo/github/uutils/coreutils)

[![Build Status (FreeBSD)](https://api.cirrus-ci.com/github/uutils/coreutils.svg)](https://cirrus-ci.com/github/uutils/coreutils/master)
[![CodeCov](https://codecov.io/gh/uutils/coreutils/branch/master/graph/badge.svg)](https://codecov.io/gh/uutils/coreutils)

-----------------------------------------------

<!-- markdownlint-disable commands-show-output no-duplicate-heading -->
<!-- spell-checker:ignore markdownlint ; (options) DESTDIR RUNTEST UTILNAME -->

uutils is an attempt at writing universal (as in cross-platform) CLI
utilities in [Rust](http://www.rust-lang.org).

To install it:

```
$ cargo install coreutils
$ ~/.cargo/bin/coreutils
```

## Why?

uutils aims to work on as many platforms as possible, to be able to use the
same utils on Linux, Mac, Windows and other platforms. This ensures, for
example, that scripts can be easily transferred between platforms. Rust was
chosen not only because it is fast and safe, but is also excellent for
writing cross-platform code.

## Documentation
uutils has both user and developer documentation available:

- [User Manual](https://uutils.github.io/coreutils-docs/user/)
- [Developer Documentation](https://uutils.github.io/coreutils-docs/dev/coreutils/)

Both can also be generated locally, the instructions for that can be found in the
[coreutils docs](https://github.com/uutils/coreutils-docs) repository.

<!-- ANCHOR: installation (this mark is needed for mdbook) -->
## Requirements

* Rust (`cargo`, `rustc`)
* GNU Make (optional)

### Rust Version

uutils follows Rust's release channels and is tested against stable, beta and nightly.
The current oldest supported version of the Rust compiler is `1.56`.

## Building

There are currently two methods to build the uutils binaries: either Cargo
or GNU Make.

> Building the full package, including all documentation, requires both Cargo
> and Gnu Make on a Unix platform.

For either method, we first need to fetch the repository:

```bash
$ git clone https://github.com/uutils/coreutils
$ cd coreutils
```

### Cargo

Building uutils using Cargo is easy because the process is the same as for
every other Rust program:

```bash
$ cargo build --release
```

This command builds the most portable common core set of uutils into a multicall
(BusyBox-type) binary, named 'coreutils', on most Rust-supported platforms.

Additional platform-specific uutils are often available. Building these
expanded sets of uutils for a platform (on that platform) is as simple as
specifying it as a feature:

```bash
$ cargo build --release --features macos
# or ...
$ cargo build --release --features windows
# or ...
$ cargo build --release --features unix
```

If you don't want to build every utility available on your platform into the
final binary, you can also specify which ones you want to build manually.
For example:

```bash
$ cargo build --features "base32 cat echo rm" --no-default-features
```

If you don't want to build the multicall binary and would prefer to build
the utilities as individual binaries, that is also possible. Each utility
is contained in its own package within the main repository, named
"uu_UTILNAME". To build individual utilities, use cargo to build just the
specific packages (using the `--package` [aka `-p`] option). For example:

```bash
$ cargo build -p uu_base32 -p uu_cat -p uu_echo -p uu_rm
```

### GNU Make

Building using `make` is a simple process as well.

To simply build all available utilities:

```bash
$ make
```

To build all but a few of the available utilities:

```bash
$ make SKIP_UTILS='UTILITY_1 UTILITY_2'
```

To build only a few of the available utilities:

```bash
$ make UTILS='UTILITY_1 UTILITY_2'
```

## Installation

### Cargo

Likewise, installing can simply be done using:

```bash
$ cargo install --path .
```

This command will install uutils into Cargo's *bin* folder (*e.g.* `$HOME/.cargo/bin`).

This does not install files necessary for shell completion. For shell completion to work,
use `GNU Make` or see `Manually install shell completions`.

### GNU Make

To install all available utilities:

```bash
$ make install
```

To install using `sudo` switch `-E` must be used:

```bash
$ sudo -E make install
```

To install all but a few of the available utilities:

```bash
$ make SKIP_UTILS='UTILITY_1 UTILITY_2' install
```

To install only a few of the available utilities:

```bash
$ make UTILS='UTILITY_1 UTILITY_2' install
```

To install every program with a prefix (e.g. uu-echo uu-cat):

```bash
$ make PROG_PREFIX=PREFIX_GOES_HERE install
```

To install the multicall binary:

```bash
$ make MULTICALL=y install
```

Set install parent directory (default value is /usr/local):

```bash
# DESTDIR is also supported
$ make PREFIX=/my/path install
```

Installing with `make` installs shell completions for all installed utilities
for `bash`, `fish` and `zsh`. Completions for `elvish` and `powershell` can also
be generated; See `Manually install shell completions`.

### NixOS

The [standard package set](https://nixos.org/nixpkgs/manual/) of [NixOS](https://nixos.org/)
provides this package out of the box since 18.03:

```shell
$ nix-env -iA nixos.uutils-coreutils
```

### Manually install shell completions

The `coreutils` binary can generate completions for the `bash`, `elvish`, `fish`, `powershell`
and `zsh` shells. It prints the result to stdout.

The syntax is:
```bash
cargo run completion <utility> <shell>
```

So, to install completions for `ls` on `bash` to `/usr/local/share/bash-completion/completions/ls`,
run:

```bash
cargo run completion ls bash > /usr/local/share/bash-completion/completions/ls
```

## Un-installation

Un-installation differs depending on how you have installed uutils.  If you used
Cargo to install, use Cargo to uninstall.  If you used GNU Make to install, use
Make to uninstall.

### Cargo

To uninstall uutils:

```bash
$ cargo uninstall uutils
```

### GNU Make

To uninstall all utilities:

```bash
$ make uninstall
```

To uninstall every program with a set prefix:

```bash
$ make PROG_PREFIX=PREFIX_GOES_HERE uninstall
```

To uninstall the multicall binary:

```bash
$ make MULTICALL=y uninstall
```

To uninstall from a custom parent directory:

```bash
# DESTDIR is also supported
$ make PREFIX=/my/path uninstall
```
<!-- ANCHOR_END: installation (this mark is needed for mdbook) -->

## Testing

Testing can be done using either Cargo or `make`.

### Cargo

Just like with building, we follow the standard procedure for testing using
Cargo:

```bash
$ cargo test
```

By default, `cargo test` only runs the common programs. To run also platform
specific tests, run:

```bash
$ cargo test --features unix
```

If you would prefer to test a select few utilities:

```bash
$ cargo test --features "chmod mv tail" --no-default-features
```

If you also want to test the core utilities:

```bash
$ cargo test  -p uucore -p coreutils
```

To debug:

```bash
$ gdb --args target/debug/coreutils ls
(gdb) b ls.rs:79
(gdb) run
```

### GNU Make

To simply test all available utilities:

```bash
$ make test
```

To test all but a few of the available utilities:

```bash
$ make SKIP_UTILS='UTILITY_1 UTILITY_2' test
```

To test only a few of the available utilities:

```bash
$ make UTILS='UTILITY_1 UTILITY_2' test
```

To include tests for unimplemented behavior:

```bash
$ make UTILS='UTILITY_1 UTILITY_2' SPEC=y test
```

### Run Busybox Tests

This testing functionality is only available on *nix operating systems and
requires `make`.

To run busybox tests for all utilities for which busybox has tests

```bash
$ make busytest
```

To run busybox tests for a few of the available utilities

```bash
$ make UTILS='UTILITY_1 UTILITY_2' busytest
```

To pass an argument like "-v" to the busybox test runtime

```bash
$ make UTILS='UTILITY_1 UTILITY_2' RUNTEST_ARGS='-v' busytest
```

### Comparing with GNU

Below is the evolution of how many GNU tests uutils passes. A more detailed
breakdown of the GNU test results of the main branch can be found
[in the user manual](https://uutils.github.io/coreutils-docs/user/test_coverage.html).

![Evolution over time](https://github.com/uutils/coreutils-tracking/blob/main/gnu-results.png?raw=true)

To run locally:

```bash
$ bash util/build-gnu.sh
$ bash util/run-gnu-test.sh
# To run a single test:
$ bash util/run-gnu-test.sh tests/touch/not-owner.sh # for example
```

Note that it relies on individual utilities (not the multicall binary).

### Improving the GNU compatibility

The Python script `./util/remaining-gnu-error.py` shows the list of failing tests in the CI.

To improve the GNU compatibility, the following process is recommended:

1. Identify a test (the smaller, the better) on a program that you understand or is easy to understand. You can use the `./util/remaining-gnu-error.py` script to help with this decision.
1. Build both the GNU and Rust coreutils using: `bash util/build-gnu.sh`
1. Run the test with `bash util/run-gnu-test.sh <your test>`
1. Start to modify `<your test>` to understand what is wrong. Examples:
    1. Add `set -v` to have the bash verbose mode
    1. Add `echo $?` where needed
    1. When the variable `fail` is used in the test, `echo $fail` to see when the test started to fail
    1. Bump the content of the output (ex: `cat err`)
    1. ...
1. Or, if the test is simple, extract the relevant information to create a new test case running both GNU & Rust implementation
1. Start to modify the Rust implementation to match the expected behavior
1. Add a test to make sure that we don't regress (our test suite is super quick)


## Contributing

To contribute to uutils, please see [CONTRIBUTING](CONTRIBUTING.md).

## Utilities

Please note that this is not fully accurate:
* Some new options can be added / removed in the GNU implementation;
* Some error management might be missing;
* Some behaviors might be different.

See https://github.com/uutils/coreutils/issues/3336 for the main meta bugs
(many are missing).

| Done      | WIP       | To Do  |
|-----------|-----------|--------|
| arch      | cp        | stty   |
| base32    | date      |        |
| base64    | dd        |        |
| basename  | df        |        |
| basenc    | expr      |        |
| cat       | install   |        |
| chcon     | ls        |        |
| chgrp     | more      |        |
| chmod     | numfmt    |        |
| chown     | od (`--strings` and 128-bit data types missing) | |
| chroot    | pr        |        |
| cksum     | printf    |        |
| comm      | sort      |        |
| csplit    | split     |        |
| cut       | tac       |        |
| dircolors | tail      |        |
| dirname   | test      |        |
| du        | dir       |        |
| echo      | vdir      |        |
| env       |           |        |
| expand    |           |        |
| factor    |           |        |
| false     |           |        |
| fmt       |           |        |
| fold      |           |        |
| groups    |           |        |
| hashsum   |           |        |
| head      |           |        |
| hostid    |           |        |
| hostname  |           |        |
| id        |           |        |
| join      |           |        |
| kill      |           |        |
| link      |           |        |
| ln        |           |        |
| logname   |           |        |
| ~~md5sum~~ (replaced by [hashsum](https://github.com/uutils/coreutils/blob/main/src/uu/hashsum/src/hashsum.rs)) | | |
| ~~sha1sum~~ (replaced by [hashsum](https://github.com/uutils/coreutils/blob/main/src/uu/hashsum/src/hashsum.rs)) | | |
| ~~sha224sum~~ (replaced by [hashsum](https://github.com/uutils/coreutils/blob/main/src/uu/hashsum/src/hashsum.rs)) | | |
| ~~sha256sum~~ (replaced by [hashsum](https://github.com/uutils/coreutils/blob/main/src/uu/hashsum/src/hashsum.rs)) | | |
| ~~sha384sum~~ (replaced by [hashsum](https://github.com/uutils/coreutils/blob/main/src/uu/hashsum/src/hashsum.rs)) | | |
| ~~sha512sum~~ (replaced by [hashsum](https://github.com/uutils/coreutils/blob/main/src/uu/hashsum/src/hashsum.rs)) | | |
| mkdir     |           |        |
| mkfifo    |           |        |
| mknod     |           |        |
| mktemp    |           |        |
| mv        |           |        |
| nice      |           |        |
| nl        |           |        |
| nohup     |           |        |
| nproc     |           |        |
| paste     |           |        |
| pathchk   |           |        |
| pinky     |           |        |
| printenv  |           |        |
| ptx       |           |        |
| pwd       |           |        |
| readlink  |           |        |
| realpath  |           |        |
| relpath   |           |        |
| rm        |           |        |
| rmdir     |           |        |
| runcon    |           |        |
| seq       |           |        |
| shred     |           |        |
| shuf      |           |        |
| sleep     |           |        |
| stat      |           |        |
| stdbuf    |           |        |
| sum       |           |        |
| sync      |           |        |
| tee       |           |        |
| timeout   |           |        |
| touch     |           |        |
| tr        |           |        |
| true      |           |        |
| truncate  |           |        |
| tsort     |           |        |
| tty       |           |        |
| uname     |           |        |
| unexpand  |           |        |
| uniq      |           |        |
| unlink    |           |        |
| uptime    |           |        |
| users     |           |        |
| wc        |           |        |
| who       |           |        |
| whoami    |           |        |
| yes       |           |        |

## Targets that compile

This is an auto-generated table showing which binaries compile for each target-triple. Note that this **does not** indicate that they are fully implemented, or that the tests pass.

|######OS######|###ARCH####|arch|base32|base64|basename|cat|chgrp|chmod|chown|chroot|cksum|comm|cp|csplit|cut|date|df|dircolors|dirname|du|echo|env|expand|expr|factor|false|fmt|fold|groups|hashsum|head|hostid|hostname|id|install|join|kill|link|ln|logname|ls|mkdir|mkfifo|mknod|mktemp|more|mv|nice|nl|nohup|nproc|numfmt|od|paste|pathchk|pinky|printenv|printf|ptx|pwd|readlink|realpath|relpath|rm|rmdir|seq|shred|shuf|sleep|sort|split|stat|stdbuf|sum|sync|tac|tail|tee|test|timeout|touch|tr|true|truncate|tsort|tty|uname|unexpand|uniq|unlink|uptime|users|wc|who|whoami|yes|
|--------------|-----------|----|------|------|--------|---|-----|-----|-----|------|-----|----|--|------|---|----|--|---------|-------|--|----|---|------|----|------|-----|---|----|------|-------|----|------|--------|--|-------|----|----|----|--|-------|--|-----|------|-----|------|----|--|----|--|-----|-----|------|--|-----|-------|-----|--------|------|---|---|--------|--------|-------|--|-----|---|-----|----|-----|----|-----|----|------|---|----|---|----|---|----|-------|-----|--|----|--------|-----|---|-----|--------|----|------|------|-----|--|---|------|---|
|linux-gnu|aarch64|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y| |y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|
|linux-gnu|i686|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|
|linux-gnu|powerpc64|y|y|y|y|y|y|y|y|y|y|y| |y|y|y|y|y|y|y|y|y|y| |y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|
|linux-gnu|riscv64gc| | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | |
|linux-gnu|x86_64|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|
|windows-msvc|aarch64|y|y|y|y|y| | | | |y|y|y|y|y|y|y|y|y| |y|y|y| |y|y|y|y| |y|y|y|y| | |y| |y|y|y| |y| | |y|y|y| |y| |y|y|y|y| | |y|y|y|y|y|y|y|y|y|y|y|y|y|y|y| | |y|y|y|y|y|y| |y|y|y|y|y| |y|y|y| |y| |y| | |y|
|windows-gnu|i686|y|y|y|y|y| | | | |y|y|y|y|y|y|y|y|y| |y|y|y| |y|y|y|y| |y|y|y|y| | |y| |y|y|y|y|y| | |y|y|y| |y| |y|y|y|y| | |y|y|y|y|y|y|y|y|y|y|y|y|y|y|y| | |y|y|y|y|y|y| |y|y|y|y|y|y|y|y|y| |y| |y| |y|y|
|windows-msvc|i686|y|y|y|y|y| | | | |y|y|y|y|y|y|y|y|y| |y|y|y| |y|y|y|y| |y|y|y|y| | |y| |y|y|y|y|y| | |y|y|y| |y| |y|y|y|y| | |y|y|y|y|y|y|y|y|y|y|y|y|y|y|y| | |y|y|y|y|y|y| |y|y|y|y|y| |y|y|y| |y| |y| |y|y|
|windows-gnu|x86_64|y|y|y|y|y| | | | |y|y|y|y|y|y|y|y|y| |y|y|y| |y|y|y|y| |y|y|y|y| | |y| |y|y|y|y|y| | |y|y|y| |y| |y|y|y|y| | |y|y|y|y|y|y|y|y|y|y|y|y|y|y|y| | |y|y|y|y|y|y| |y|y|y|y|y|y|y|y|y| |y| |y| |y|y|
|windows-msvc|x86_64|y|y|y|y|y| | | | |y|y|y|y|y|y|y|y|y| |y|y|y| |y|y|y|y| |y|y|y|y| | |y| |y|y|y|y|y| | |y|y|y| |y| |y|y|y|y| | |y|y|y|y|y|y|y|y|y|y|y|y|y|y|y| | |y|y|y|y|y|y| |y|y|y|y|y| |y|y|y| |y| |y| |y|y|
|apple MacOS|aarch64|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y| |y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|
|apple MacOS|x86_64|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y| |y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|
|freebsd|x86_64|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|
|netbsd|x86_64|y|y|y|y|y|y|y|y| |y|y|y|y|y|y| |y|y|y|y|y|y| |y|y|y|y|y|y|y|y|y| |y|y| |y|y|y|y|y|y|y|y|y|y|y|y| |y|y|y|y|y| |y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y|y| |y|y|y|y|y|y| |y|y|y| | |y| |y|y|
|android|aarch64|y|y|y|y|y|y|y|y| |y|y|y|y|y|y| |y|y|y|y|y|y| |y|y|y|y|y|y|y|y|y| |y|y| |y|y|y|y|y|y|y|y|y|y|y|y| |y|y|y|y|y| |y|y|y|y|y|y|y|y|y|y|y|y|y|y|y| |y|y| |y|y|y|y| |y|y|y|y|y|y| |y|y|y| | |y| |y|y|
|android|x86_64|y|y|y|y|y|y|y|y| |y|y|y|y|y|y| |y|y|y|y|y|y| |y|y|y|y|y|y|y|y|y| |y|y| |y|y|y|y|y|y|y|y|y|y|y|y| |y|y|y|y|y| |y|y|y|y|y|y|y|y|y|y|y|y|y|y|y| |y|y| |y|y|y|y| |y|y|y|y|y|y| |y|y|y| | |y| |y|y|
|solaris|x86_64| | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | |
|wasi|wasm32| | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | |
|redox|x86_64| | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | |
|fuchsia|aarch64| | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | |
|fuchsia|x86_64| | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | |

## License

uutils is licensed under the MIT License - see the `LICENSE` file for details

GNU Coreutils is licensed under the GPL 3.0 or later.
