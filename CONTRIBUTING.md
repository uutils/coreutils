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

## Setting up your development environment

To setup your local development environment for this project please follow [DEVELOPMENT.md guide](DEVELOPMENT.md)

It covers [installation of necessary tools and prerequisites](DEVELOPMENT.md#tools) as well as using those tools to [test your code changes locally](DEVELOPMENT.md#testing)

## Improving the GNU compatibility

Please make sure you have installed [GNU utils and prerequisites](DEVELOPMENT.md#gnu-utils-and-prerequisites) and can execute commands described in [Comparing with GNU](DEVELOPMENT.md#comparing-with-gnu) section of [DEVELOPMENT.md](DEVELOPMENT.md)

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

To generate code coverage report locally please follow [Code coverage report](DEVELOPMENT.md#code-coverage-report) section of [DEVELOPMENT.md](DEVELOPMENT.md)

## Other implementations

The Coreutils have different implementations, with different levels of completions:

* [GNU's](https://git.savannah.gnu.org/gitweb/?p=coreutils.git)
* [OpenBSD](https://github.com/openbsd/src/tree/master/bin)
* [Busybox](https://github.com/mirror/busybox/tree/master/coreutils)
* [Toybox (Android)](https://github.com/landley/toybox/tree/master/toys/posix)
* [V lang](https://github.com/vlang/coreutils)
* [SerenityOS](https://github.com/SerenityOS/serenity/tree/master/Userland/Utilities)
* [Initial Unix](https://github.com/dspinellis/unix-history-repo)
* [Perl Power Tools](https://metacpan.org/pod/PerlPowerTools)

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
