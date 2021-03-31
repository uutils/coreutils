uutils coreutils
================

[![Crates.io](https://img.shields.io/crates/v/coreutils.svg)](https://crates.io/crates/coreutils)
[![Discord](https://img.shields.io/badge/discord-join-7289DA.svg?logo=discord&longCache=true&style=flat)](https://discord.gg/wQVJbvJ)
[![License](http://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/uutils/coreutils/blob/master/LICENSE)
[![LOC](https://tokei.rs/b1/github/uutils/coreutils?category=code)](https://github.com/Aaronepower/tokei)
[![dependency status](https://deps.rs/repo/github/uutils/coreutils/status.svg)](https://deps.rs/repo/github/uutils/coreutils)

[![Build Status](https://api.travis-ci.org/uutils/coreutils.svg?branch=master)](https://travis-ci.org/uutils/coreutils)
[![Build Status (FreeBSD)](https://api.cirrus-ci.com/github/uutils/coreutils.svg)](https://cirrus-ci.com/github/uutils/coreutils/master)
[![codecov](https://codecov.io/gh/uutils/coreutils/branch/master/graph/badge.svg)](https://codecov.io/gh/uutils/coreutils)

-----------------------------------------------

uutils is an attempt at writing universal (as in cross-platform) CLI
utilities in [Rust](http://www.rust-lang.org). This repository is intended to
aggregate GNU coreutils rewrites.

Why?
----

Many GNU, Linux and other utilities are useful, and obviously
[some](http://gnuwin32.sourceforge.net) [effort](http://unxutils.sourceforge.net)
has been spent in the past to port them to Windows. However, those projects
are written in platform-specific C, a language considered unsafe compared to Rust, and 
have other issues.

Rust provides a good, platform-agnostic way of writing systems utilities that are easy
to compile anywhere, and this is as good a way as any to try and learn it.

Requirements
------------

* Rust (`cargo`, `rustc`)
* GNU Make (required to build documentation)
* [Sphinx](http://www.sphinx-doc.org/) (for documentation)
* gzip (for installing documentation)

### Rust Version ###

uutils follows Rust's release channels and is tested against stable, beta and nightly.
The current oldest supported version of the Rust compiler is `1.40.0`.

On both Windows and Redox, only the nightly version is tested currently.

Build Instructions
------------------

There are currently two methods to build uutils: GNU Make and Cargo.  However,
while there may be two methods, both systems are required to build on Unix
(only Cargo is required on Windows).

First, for both methods, we need to fetch the repository:
```bash
$ git clone https://github.com/uutils/coreutils
$ cd coreutils
```

### Cargo ###

Building uutils using Cargo is easy because the process is the same as for
every other Rust program:
```bash
# to keep debug information, compile without --release
$ cargo build --release
```

Because the above command attempts to build utilities that only work on
Unix-like platforms at the moment, to build on Windows, you must do the
following:
```bash
# to keep debug information, compile without --release
$ cargo build --release --no-default-features --features windows
```

If you don't want to build every utility available on your platform into the
multicall binary (the Busybox-esque binary), you can also specify which ones
you want to build manually.  For example:
```bash
$ cargo build --features "base32 cat echo rm" --no-default-features
```

If you don't even want to build the multicall binary and would prefer to just
build the utilities as individual binaries, that is possible too.  For example:
```bash
$ cargo build -p uu_base32 -p uu_cat -p uu_echo -p uu_rm
```

### GNU Make ###

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

Installation Instructions
-------------------------

### Cargo ###

Likewise, installing can simply be done using:
```bash
$ cargo install --path .
```

This command will install uutils into Cargo's *bin* folder (*e.g.* `$HOME/.cargo/bin`).

### GNU Make ###

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

### NixOS ###

The [standard package set](https://nixos.org/nixpkgs/manual/) of [NixOS](https://nixos.org/)
provides this package out of the box since 18.03:

```
nix-env -iA nixos.uutils-coreutils
```

Uninstallation Instructions
---------------------------

Uninstallation differs depending on how you have installed uutils.  If you used
Cargo to install, use Cargo to uninstall.  If you used GNU Make to install, use
Make to uninstall.

### Cargo ###

To uninstall uutils:
```bash
$ cargo uninstall uutils
```

### GNU Make ###

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

Test Instructions
-----------------

Testing can be done using either Cargo or `make`.

### Cargo ###

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

To debug:
```bash
$ gdb --args target/debug/coreutils ls
(gdb) b ls.rs:79
(gdb) run
```

### GNU Make ###

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

Run Busybox Tests
-----------------

This testing functionality is only available on *nix operating systems and
requires `make`.

To run busybox's tests for all utilities for which busybox has tests
```bash
$ make busytest
```

To run busybox's tests for a few of the available utilities
```bash
$ make UTILS='UTILITY_1 UTILITY_2' busytest
```

To pass an argument like "-v" to the busybox test runtime
```bash
$ make UTILS='UTILITY_1 UTILITY_2' RUNTEST_ARGS='-v' busytest
```

Contribute
----------

To contribute to uutils, please see [CONTRIBUTING](CONTRIBUTING.md).

Utilities
---------

| Done      | Semi-Done | To Do  |
|-----------|-----------|--------|
| arch      | cp        | chcon  |
| base32    | expr      | dd     |
| base64    | install   | numfmt |
| basename  | ls        | pr     |
| cat       | more      | runcon |
| chgrp     | od (`--strings` and 128-bit data types missing) | stty |
| chmod     | printf    |        |
| chown     | sort      |        |
| chroot    | split     |        |
| cksum     | tail      |        |
| comm      | test      |        |
| csplit    | date      |        |
| cut       | join      |        |
| dircolors | df        |        |
| dirname   | tac       |        |
| du        |           |        |
| echo      |           |        |
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
| kill      |           |        |
| link      |           |        |
| ln        |           |        |
| logname   |           |        |
| ~~md5sum~~ (replaced by [hashsum](https://github.com/uutils/coreutils/blob/master/src/uu/hashsum/src/hashsum.rs)) | |
| ~~sha1sum~~ (replaced by [hashsum](https://github.com/uutils/coreutils/blob/master/src/uu/hashsum/src/hashsum.rs)) | |
| ~~sha224sum~~ (replaced by [hashsum](https://github.com/uutils/coreutils/blob/master/src/uu/hashsum/src/hashsum.rs)) | |
| ~~sha256sum~~ (replaced by [hashsum](https://github.com/uutils/coreutils/blob/master/src/uu/hashsum/src/hashsum.rs)) | |
| ~~sha384sum~~ (replaced by [hashsum](https://github.com/uutils/coreutils/blob/master/src/uu/hashsum/src/hashsum.rs)) | |
| ~~sha512sum~~ (replaced by [hashsum](https://github.com/uutils/coreutils/blob/master/src/uu/hashsum/src/hashsum.rs)) | |
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

Targets that compile
-------

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

License
-------

uutils is licensed under the MIT License - see the `LICENSE` file for details

GNU Coreutils is licensed under the GPL 3.0 or later.
