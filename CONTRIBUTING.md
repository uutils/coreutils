<!-- spell-checker:ignore reimplementing toybox RUNTEST CARGOFLAGS nextest embeddable Rustonomicon rustdoc's -->

# Contributing to coreutils

Hi! Welcome to uutils/coreutils!

Thanks for wanting to contribute to this project! This document explains
everything you need to know to contribute. Before you start make sure to also
check out these documents:

- Our community's [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md).
- [DEVELOPMENT.md](./DEVELOPMENT.md) for setting up your development
  environment.

Now follows a very important warning:

> [!WARNING]
> uutils is original code and cannot contain any code from GNU or
> other implementations. This means that **we cannot accept any changes based on
> the GNU source code**. To make sure that cannot happen, **you cannot link to
> the GNU source code** either. It is however possible to look at other implementations
> under a BSD or MIT license like [Apple's implementation](https://opensource.apple.com/source/file_cmds/)
> or [OpenBSD](https://github.com/openbsd/src/tree/master/bin).

Finally, feel free to join our [Discord](https://discord.gg/wQVJbvJ)!

## Getting Oriented

uutils is a big project consisting of many parts. Here are the most important
parts for getting started:

- [`src/uu`](./src/uu/): The code for all utilities
- [`src/uucore`](./src/uucore/): Crate containing all the shared code between
  the utilities. This crate is also used outside of the Coreutils.
- [`tests/by-util`](./tests/by-util/): The tests for all utilities.
- [`src/bin/coreutils.rs`](./src/bin/coreutils.rs): Code for the multicall
  binary.
- [`docs`](./docs/src): the documentation for the website
- [`tests/uutests/`](./tests/uutests/): Crate implementing
  the various functions to test uutils commands.

Each utility is defined as a separate crate. The structure of each of these
crates is as follows:

- `Cargo.toml`
- `src/main.rs`: contains only a single macro call
- `src/<util name>.rs`: the actual code for the utility
- `<util name>.md`: the documentation for the utility

We have separated repositories for crates that we maintain but also publish for
use by others:

- [uutils-term-grid](https://github.com/uutils/uutils-term-grid)
- [parse_datetime](https://github.com/uutils/parse_datetime)

## Design Goals

We have the following goals with our development:

- **Compatible**: The utilities should be a drop-in replacement for the GNU
  coreutils.
- **Cross-platform**: All utilities should run on as many of the supported
  platforms as possible.
- **Reliable**: The utilities should never unexpectedly fail.
- **Performant**: Our utilities should be written in fast idiomatic Rust. We aim
  to match or exceed the performance of the GNU utilities.
  [hyperfine](https://github.com/sharkdp/hyperfine) is the recommended tool for
  this task.
- **Well-tested**: We should have a lot of tests to be able to guarantee
  reliability and compatibility.

## How to Help

There are several ways to help and writing code is just one of them. Reporting
issues and writing documentation are just as important as writing code.

### Reporting Issues

We can't fix bugs we don't know about, so good issues are super helpful! Here
are some tips for writing good issues:

- If you find a bug, make sure it's still a problem on the `main` branch.
- Search through the existing issues to see whether it has already been
  reported.
- Make sure to include all relevant information, such as:
  - Which version of uutils did you check?
  - Which version of GNU coreutils are you comparing with?
  - What platform are you on?
- Provide a way to reliably reproduce the issue.
- Be as specific as possible!

### Writing Documentation

There's never enough documentation. If you come across any documentation that
could be improved, feel free to submit a PR for it!

### Writing Code

If you want to submit a PR, make sure that you've discussed the solution with
the maintainers beforehand. We want to avoid situations where you put a lot of
work into a fix that we can't merge! If there's no issue for what you're trying
to fix yet, make one _before_ you start working on the PR.

Generally, we try to follow what GNU is doing in terms of options and behavior.
It is recommended to look at the GNU coreutils manual
([on the web](https://www.gnu.org/software/coreutils/manual/html_node/index.html),
or locally using `info <utility>`). It is more in depth than the man pages and
provides a good description of available features and their implementation
details. But remember, you cannot look at the GNU source code!

Also remember that we can only merge PRs which pass our test suite, follow
rustfmt, and do not have any warnings from clippy. See
[DEVELOPMENT.md](./DEVELOPMENT.md) for more information. Be sure to also read
about our [Rust style](#our-rust-style).

## Our Rust Style

We want uutils to be written in idiomatic Rust, so here are some guidelines to
follow. Some of these are aspirational, meaning that we don't do them correctly
everywhere in the code. If you find violations of the advice below, feel free to
submit a patch!

### Don't `panic!`

The coreutils should be very reliable. This means that we should never `panic!`.
Therefore, you should avoid using `.unwrap()` and `panic!`. Sometimes the use of
`unreachable!` can be justified with a comment explaining why that code is
unreachable.

### Don't `exit`

We want uutils to be embeddable in other programs. This means that no function
in uutils should exit the program. Doing so would also lead to code with more
confusing control flow. Avoid therefore `std::process::exit` and similar
functions which exit the program early.

### `unsafe`

uutils cannot be entirely safe, because we have to call out to `libc` and do
syscalls. However, we still want to limit our use of `unsafe`. We generally only
accept `unsafe` for FFI, with very few exceptions. Note that performance is very
rarely a valid argument for using `unsafe`.

If you still need to write code with `unsafe`, make sure to read the
[Rustonomicon](https://doc.rust-lang.org/nomicon/intro.html) and annotate the
calls with `// SAFETY:` comments explaining why the use of `unsafe` is sound.

### Macros

Macros can be a great tool, but they are also usually hard to understand. They
should be used sparingly. Make sure to explore simpler options before you reach
for a solution involving macros.

### `str`, `OsStr` & `Path`

Rust has many string-like types, and sometimes it's hard to choose the right
one. It's tempting to use `str` (and `String`) for everything, but that is not
always the right choice for uutils, because we need to support invalid UTF-8,
just like the GNU coreutils. For example, paths on Linux might not be valid
UTF-8! Whenever we are dealing with paths, we should therefore stick with
`OsStr` and `Path`. Make sure that you only convert to `str`/`String` if you
know that something is always valid UTF-8. If you need more operations on
`OsStr`, you can use the [`bstr`](https://docs.rs/bstr/latest/bstr/) crate.

### Doc-comments

We use rustdoc for our documentation, so it's best to follow
[rustdoc's guidelines](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html#documenting-components).
Make sure that your documentation is not just repeating the name of the
function, but actually giving more useful information. Rustdoc recommends the
following structure:

```
[short sentence explaining what it is]

[more detailed explanation]

[at least one code example that users can copy/paste to try it]

[even more advanced explanations if necessary]
```

### Other comments

Comments should be written to _explain_ the code, not to _describe_ the code.
Try to focus on explaining _why_ the code is the way it is. If you feel like you
have to describe the code, that's usually a sign that you could improve the
naming of variables and functions.

If you edit a piece of code, make sure to update any comments that need to
change as a result. The only thing worse than having no comments is having
outdated comments!

## Git Etiquette

To ensure easy collaboration, we have guidelines for using Git and GitHub.

### Commits

- Make small and atomic commits.
- Keep a clean history of commits.
- Write informative commit messages.
- Annotate your commit message with the component you're editing. For example:
  `cp: do not overwrite on with -i` or `uucore: add support for FreeBSD`.
- Do not unnecessarily move items around in the code. This makes the changes
  much harder to review. If you do need to move things around, do that in a
  separate commit.

### Commit messages

You can read [this section in the Git book](https://git-scm.com/book/ms/v2/Distributed-Git-Contributing-to-a-Project) to learn how to write good commit
messages.

In addition, here are a few examples for a summary line when committing to
uutils:

- commit for a single utility

```
nohup: cleanup and refactor
```

- commit for a utility's tests

```
tests/rm: test new feature
```

Beyond changes to an individual utility or its tests, other summary lines for
non-utility modules include:

```
README: add help
uucore: add new modules
uutils: add new utility
gitignore: add temporary files
```

### PRs

- Make the titles of PRs descriptive.
  - This means describing the problem you solve. For example, do not write
    `Fix #1234`, but `ls: fix version sort order`.
  - You can prefix the title with the utility the PR concerns.
- Keep PRs small and self-contained. A set of small PRs is much more likely to
  get merged quickly than one large PR.
- Make sure the CI passes (up to intermittently failing tests).
- You know your code best, that's why it's best if you can solve merge conflicts
  on your branch yourself.
  - It's up to you whether you want to use `git merge main` or
    `git rebase main`.
  - Feel free to ask for help with merge conflicts.
- You do not need to ping maintainers to request a review, but it's fine to do
  so if you don't get a response within a few days.

## Platforms

We take pride in supporting many operating systems and architectures. Any code
you contribute must at least compile without warnings for all platforms in the
CI. However, you can use `#[cfg(...)]` attributes to create platform dependent
features.

**Tip:** For Windows, Microsoft provides some images (VMWare, Hyper-V,
VirtualBox and Parallels) for development [here](https://developer.microsoft.com/windows/downloads/virtual-machines/).

## Improving the GNU compatibility

Please make sure you have installed
[GNU utils and prerequisites](DEVELOPMENT.md#gnu-utils-and-prerequisites) and
can execute commands described in
[Comparing with GNU](DEVELOPMENT.md#comparing-with-gnu) section of
[DEVELOPMENT.md](DEVELOPMENT.md)

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

## Code coverage

To generate code coverage report locally please follow
[Code coverage report](DEVELOPMENT.md#code-coverage-report) section of
[DEVELOPMENT.md](DEVELOPMENT.md)

## Other implementations

The Coreutils have different implementations, with different levels of
completions:

- [GNU's](https://git.savannah.gnu.org/gitweb/?p=coreutils.git)
- [OpenBSD](https://github.com/openbsd/src/tree/master/bin)
- [Busybox](https://github.com/mirror/busybox/tree/master/coreutils)
- [Toybox (Android)](https://github.com/landley/toybox/tree/master/toys/posix)
- [Mac OS](https://opensource.apple.com/source/file_cmds/)
- [V lang](https://github.com/vlang/coreutils)
- [SerenityOS](https://github.com/SerenityOS/serenity/tree/master/Userland/Utilities)
- [Initial Unix](https://github.com/dspinellis/unix-history-repo)
- [Perl Power Tools](https://metacpan.org/pod/PerlPowerTools)

However, when reimplementing the tools/options in Rust, don't read their source
codes when they are using reciprocal licenses (ex: GNU GPL, GNU LGPL, etc).

## Licensing

uutils is distributed under the terms of the MIT License; see the `LICENSE` file
for details. This is a permissive license, which allows the software to be used
with few restrictions.

Copyrights in the uutils project are retained by their contributors, and no
copyright assignment is required to contribute.

If you wish to add or change dependencies as part of a contribution to the
project, a tool like `cargo-license` can be used to show their license details.
The following types of license are acceptable:

- MIT License
- Dual- or tri-license with an MIT License option ("Apache-2.0 or MIT" is a
  popular combination)
- "MIT equivalent" license (2-clause BSD, 3-clause BSD, ISC)
- License less restrictive than the MIT License (CC0 1.0 Universal)
- Apache License version 2.0

Licenses we will not use:

- An ambiguous license, or no license
- Strongly reciprocal licenses (GNU GPL, GNU LGPL)

If you wish to add a reference but it doesn't meet these requirements, please
raise an issue to describe the dependency.
