<!-- spell-checker:ignore reimplementing toybox RUNTEST CARGOFLAGS nextest -->

# Contributing to coreutils

Contributions are very welcome via Pull Requests. If you don't know where to
start, take a look at the
[`good-first-issues`](https://github.com/uutils/coreutils/issues?q=is%3Aopen+is%3Aissue+label%3A%22good+first+issue%22).
If you have any questions, feel free to ask them in the issues or on
[Discord](https://discord.gg/wQVJbvJ).

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

We take pride in supporting many operating systems and architectures. Any code
you contribute must at least compile without warnings for all platforms in the
CI. However, you can use `#[cfg(...)]` attributes to create platform dependent features.

**Tip:** For Windows, Microsoft provides some images (VMWare, Hyper-V,
VirtualBox and Parallels) for development:
<https://developer.microsoft.com/windows/downloads/virtual-machines/>

## Tools

We have an extensive CI that will check your code before it can be merged. This
section explains how to run those checks locally to avoid waiting for the CI.

### pre-commit hooks

A configuration for `pre-commit` is provided in the repository. It allows
automatically checking every git commit you make to ensure it compiles, and
passes `clippy` and `rustfmt` without warnings.

To use the provided hook:

1. [Install `pre-commit`](https://pre-commit.com/#install)
1. Run `pre-commit install` while in the repository directory

Your git commits will then automatically be checked. If a check fails, an error
message will explain why, and your commit will be canceled. You can then make
the suggested changes, and run `git commit ...` again.

### clippy

```shell
cargo clippy --all-targets --all-features
```

The `msrv` key in the clippy configuration file `clippy.toml` is used to disable
lints pertaining to newer features by specifying the minimum supported Rust
version (MSRV).

### rustfmt

```shell
cargo fmt --all
```

### cargo-deny

This project uses [cargo-deny](https://github.com/EmbarkStudios/cargo-deny/) to
detect duplicate dependencies, checks licenses, etc. To run it locally, first
install it and then run with:

```
cargo deny --all-features check all
```

### Markdown linter

We use [markdownlint](https://github.com/DavidAnson/markdownlint) to lint the
Markdown files in the repository.

### Spell checker

We use `cspell` as spell checker for all files in the project. If you are using
VS Code, you can install the
[code spell checker](https://marketplace.visualstudio.com/items?itemName=streetsidesoftware.code-spell-checker)
extension to enable spell checking within your editor. Otherwise, you can
install [cspell](https://cspell.org/) separately.

If you want to make the spell checker ignore a word, you can add

```rust
// spell-checker:ignore word_to_ignore
```

at the top of the file.

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

Running the complete test suite might take a while. We use [nextest](https://nexte.st/index.html) in
the CI and you might want to try it out locally. It can speed up the execution time of the whole
test run significantly if the cpu has multiple cores.

```shell
cargo nextest run --features unix --no-fail-fast
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

To run tests with `nextest` just use the nextest target. Note you'll need to
[install](https://nexte.st/book/installation.html) `nextest` first. The `nextest` target accepts the
same arguments like the default `test` target, so it's possible to pass arguments to `nextest run`
via `CARGOFLAGS`:

```shell
make CARGOFLAGS='--no-fail-fast' UTILS='UTILITY_1 UTILITY_2' nextest
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

From <https://git-scm.com/book/ch5-2.html>:

```
Capitalized, short (50 chars or less) summary

More detailed explanatory text, if necessary.  Wrap it to about 72
characters or so.  In some contexts, the first line is treated as the
subject of an email and the rest of the text as the body.  The blank
line separating the summary from the body is critical (unless you omit
the body entirely); tools like rebase will confuse you if you run the
two together.

Write your commit message in the imperative: "Fix bug" and not "Fixed bug"
or "Fixes bug."  This convention matches up with commit messages generated
by commands like git merge and git revert.

Further paragraphs come after blank lines.

  - Bullet points are okay, too

  - Typically a hyphen or asterisk is used for the bullet, followed by a
    single space, with blank lines in between, but conventions vary here

  - Use a hanging indent
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

## Code coverage

<!-- spell-checker:ignore (flags) Ccodegen Coverflow Cpanic Zinstrument Zpanic -->

Code coverage report can be generated using [grcov](https://github.com/mozilla/grcov).

### Using Nightly Rust

To generate [gcov-based](https://github.com/mozilla/grcov#example-how-to-generate-gcda-files-for-a-rust-project) coverage report

```shell
export CARGO_INCREMENTAL=0
export RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort"
export RUSTDOCFLAGS="-Cpanic=abort"
cargo build <options...> # e.g., --features feat_os_unix
cargo test <options...> # e.g., --features feat_os_unix test_pathchk
grcov . -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing --ignore build.rs --excl-br-line "^\s*((debug_)?assert(_eq|_ne)?\#\[derive\()" -o ./target/debug/coverage/
# open target/debug/coverage/index.html in browser
```

if changes are not reflected in the report then run `cargo clean` and run the above commands.

### Using Stable Rust

If you are using stable version of Rust that doesn't enable code coverage instrumentation by default
then add `-Z-Zinstrument-coverage` flag to `RUSTFLAGS` env variable specified above.

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
