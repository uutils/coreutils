<!-- spell-checker:ignore reimplementing toybox -->

# Contributing to coreutils

Contributions are very welcome, and should target Rust's main branch until the
standard libraries are stabilized. You may *claim* an item on the to-do list by
following these steps:

1. Open an issue named "Implement [the utility of your choice]", e.g. "Implement
   ls".
1. State that you are working on this utility.
1. Develop the utility.
1. Add integration tests.
1. Add the reference to your utility into Cargo.toml and Makefile.
1. Remove utility from the to-do list in the README.
1. Submit a pull request and close the issue.

The steps above imply that, before starting to work on a utility, you should
search the issues to make sure no one else is working on it.

## Best practices

1. Follow what GNU is doing in terms of options and behavior. It is recommended
   to look at the GNU Coreutils manual ([on the
   web](https://www.gnu.org/software/coreutils/manual/html_node/index.html), or
   locally using `info <utility>`). It is more in depth than the man pages and
   provides a good description of available features and their implementation
   details.
1. If possible, look at the GNU test suite execution in the CI and make the test
   work if failing.
1. Use clap for argument management.
1. Make sure that the code coverage is covering all of the cases, including
   errors.
1. The code must be clippy-warning-free and rustfmt-compliant.
1. Don't hesitate to move common functions into uucore if they can be reused by
   other binaries.
1. Unsafe code should be documented with Safety comments.
1. uutils is original code. It cannot contain code from existing GNU or Unix-like
   utilities, nor should it link to or reference GNU libraries.

## Platforms

We take pride in supporting many operating systems and architectures.

**Tip:**
For Windows, Microsoft provides some images (VMWare, Hyper-V, VirtualBox and Parallels)
for development:
<https://developer.microsoft.com/windows/downloads/virtual-machines/>

## Testing

Testing can be done using either Cargo or `make`.

### Testing with Cargo

Just like with building, we follow the standard procedure for testing using
Cargo:

```shell
cargo test
```

By default, `cargo test` only runs the common programs. To run also platform
specific tests, run:

```shell
cargo test --features unix
```

If you would prefer to test a select few utilities:

```shell
cargo test --features "chmod mv tail" --no-default-features
```

If you also want to test the core utilities:

```shell
cargo test  -p uucore -p coreutils
```

To debug:

```shell
gdb --args target/debug/coreutils ls
(gdb) b ls.rs:79
(gdb) run
```

### Testing with GNU Make

To simply test all available utilities:

```shell
make test
```

To test all but a few of the available utilities:

```shell
make SKIP_UTILS='UTILITY_1 UTILITY_2' test
```

To test only a few of the available utilities:

```shell
make UTILS='UTILITY_1 UTILITY_2' test
```

To include tests for unimplemented behavior:

```shell
make UTILS='UTILITY_1 UTILITY_2' SPEC=y test
```

### Run Busybox Tests

This testing functionality is only available on *nix operating systems and
requires `make`.

To run busybox tests for all utilities for which busybox has tests

```shell
make busytest
```

To run busybox tests for a few of the available utilities

```shell
make UTILS='UTILITY_1 UTILITY_2' busytest
```

To pass an argument like "-v" to the busybox test runtime

```shell
make UTILS='UTILITY_1 UTILITY_2' RUNTEST_ARGS='-v' busytest
```

### Comparing with GNU

To run uutils against the GNU test suite locally, run the following commands:

```shell
bash util/build-gnu.sh
bash util/run-gnu-test.sh
# To run a single test:
bash util/run-gnu-test.sh tests/touch/not-owner.sh # for example
# To run several tests:
bash util/run-gnu-test.sh tests/touch/not-owner.sh tests/rm/no-give-up.sh # for example
# If this is a perl (.pl) test, to run in debug:
DEBUG=1 bash util/run-gnu-test.sh tests/misc/sm3sum.pl
```

Note that it relies on individual utilities (not the multicall binary).

### Improving the GNU compatibility

The Python script `./util/remaining-gnu-error.py` shows the list of failing
tests in the CI.

To improve the GNU compatibility, the following process is recommended:

1. Identify a test (the smaller, the better) on a program that you understand or
   is easy to understand. You can use the `./util/remaining-gnu-error.py` script
   to help with this decision.
1. Build both the GNU and Rust coreutils using: `bash util/build-gnu.sh`
1. Run the test with `bash util/run-gnu-test.sh <your test>`
1. Start to modify `<your test>` to understand what is wrong. Examples:
   1. Add `set -v` to have the bash verbose mode
   1. Add `echo $?` where needed
   1. When the variable `fail` is used in the test, `echo $fail` to see when the
      test started to fail
   1. Bump the content of the output (ex: `cat err`)
   1. ...
1. Or, if the test is simple, extract the relevant information to create a new
   test case running both GNU & Rust implementation
1. Start to modify the Rust implementation to match the expected behavior
1. Add a test to make sure that we don't regress (our test suite is super quick)

## Commit messages

To help the project maintainers review pull requests from contributors across
numerous utilities, the team has settled on conventions for commit messages.

From <http://git-scm.com/book/ch5-2.html>:

```
Short (50 chars or less) summary of changes

More detailed explanatory text, if necessary.  Wrap it to about 72
characters or so.  In some contexts, the first line is treated as the
subject of an email and the rest of the text as the body.  The blank
line separating the summary from the body is critical (unless you omit
the body entirely); tools like rebase can get confused if you run the
two together.

Further paragraphs come after blank lines.

  - Bullet points are okay, too

  - Typically a hyphen or asterisk is used for the bullet, preceded by a
    single space, with blank lines in between, but conventions vary here
```

Furthermore, here are a few examples for a summary line:

* commit for a single utility

```
nohup: cleanup and refactor
```

* commit for a utility's tests

```
tests/rm: test new feature
```

Beyond changes to an individual utility or its tests, other summary
lines for non-utility modules include:

```
README: add help
```

```
uucore: add new modules
```

```
uutils: add new utility
```

```
gitignore: add temporary files
```

## cargo-deny

This project uses [cargo-deny](https://github.com/EmbarkStudios/cargo-deny/) to
detect duplicate dependencies, checks licenses, etc. To run it locally, first
install it and then run with:

```
cargo deny --all-features check all
```

## Other implementations

The Coreutils have different implementations, with different levels of completions:

* [GNU's](https://git.savannah.gnu.org/gitweb/?p=coreutils.git)
* [OpenBSD](https://github.com/openbsd/src/tree/master/bin)
* [Busybox](https://github.com/mirror/busybox/tree/master/coreutils)
* [Toybox (Android)](https://github.com/landley/toybox/tree/master/toys/posix)
* [V lang](https://github.com/vlang/coreutils)
* [SerenityOS](https://github.com/SerenityOS/serenity/tree/master/Userland/Utilities)
* [Initial Unix](https://github.com/dspinellis/unix-history-repo)

However, when reimplementing the tools/options in Rust, don't read their source codes 
when they are using reciprocal licenses (ex: GNU GPL, GNU LGPL, etc).


## Licensing

uutils is distributed under the terms of the MIT License; see the `LICENSE` file
for details. This is a permissive license, which allows the software to be used
with few restrictions.

Copyrights in the uutils project are retained by their contributors, and no
copyright assignment is required to contribute.

If you wish to add or change dependencies as part of a contribution to the
project, a tool like `cargo-license` can be used to show their license details.
The following types of license are acceptable:

* MIT License
* Dual- or tri-license with an MIT License option ("Apache-2.0 or MIT" is a
  popular combination)
* "MIT equivalent" license (2-clause BSD, 3-clause BSD, ISC)
* License less restrictive than the MIT License (CC0 1.0 Universal)
* Apache License version 2.0

Licenses we will not use:

* An ambiguous license, or no license
* Strongly reciprocal licenses (GNU GPL, GNU LGPL)

If you wish to add a reference but it doesn't meet these requirements, please
raise an issue to describe the dependency.
