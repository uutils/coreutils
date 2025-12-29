<!-- spell-checker:ignore (flags) Ccodegen Coverflow Cpanic Cinstrument Zpanic reimplementing toybox RUNTEST CARGOFLAGS nextest prereq autopoint gettext texinfo automake findutils shellenv libexec gnubin toolchains gsed -->

# Setting up your local development environment

For contributing rules and best practices please refer to [CONTRIBUTING.md](CONTRIBUTING.md)

## Before you start

For this guide we assume that you already have a GitHub account and have `git` and your favorite code editor or IDE installed and configured.
Before you start working on coreutils, please follow these steps:

1. Fork the [coreutils repository](https://github.com/uutils/coreutils) to your GitHub account.
***Tip:*** See [this GitHub guide](https://docs.github.com/en/get-started/quickstart/fork-a-repo) for more information on this step.
2. Clone that fork to your local development environment:

```shell
git clone https://github.com/YOUR-GITHUB-ACCOUNT/coreutils
cd coreutils
```

## Tools

You will need the tools mentioned in this section to build and test your code changes locally.
This section will explain how to install and configure these tools.
We also have an extensive CI that uses these tools and will check your code before it can be merged.
The next section [Testing](#testing) will explain how to run those checks locally to avoid waiting for the CI.

As an alternative to host installation of the tools, you can open the project with the provided development container configuration.
For more information about development containers, see the [Visual Studio Code Dev Containers documentation](https://code.visualstudio.com/docs/devcontainers/containers).

### Rust toolchain

[Install Rust](https://www.rust-lang.org/tools/install)

If you're using rustup to install and manage your Rust toolchains, `clippy` and `rustfmt` are usually already installed. If you are using one of the alternative methods, please make sure to install them manually. See following sub-sections for their usage: [clippy](#clippy) [rustfmt](#rustfmt).

***Tip*** You might also need to add 'llvm-tools' component if you are going to [generate code coverage reports locally](#code-coverage-report):

```shell
rustup component add llvm-tools-preview
```

### GNU utils and prerequisites

If you are developing on Linux, most likely you already have all/most GNU utilities and prerequisites installed.

To make sure, please check GNU coreutils [README-prereq](https://github.com/coreutils/coreutils/blob/master/README-prereq).

You will need these to [run uutils against the GNU test suite locally](#comparing-with-gnu).

For MacOS and Windows platform specific setup please check [MacOS GNU utils](#macos-gnu-utils) and [Windows GNU utils](#windows-gnu-utils) sections respectfully.

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

**NOTE: On MacOS** the pre-commit hooks are currently broken. There are workarounds involving switching to unstable nightly Rust and components.

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

```shell
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

This section explains how to run our CI checks locally.
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
# or
cargo test --all-features -p uucore
```

Running the complete test suite might take a while. We use [nextest](https://nexte.st/index.html) in
the CI and you might want to try it out locally. It can speed up the execution time of the whole
test run significantly if the cpu has multiple cores.

```shell
cargo nextest run --features unix --no-fail-fast
```

To debug:

```shell
rust-gdb --args target/debug/coreutils ls
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
# Build uutils with release optimizations
env PROFILE=release bash util/build-gnu.sh
# Build uutils with SELinux
env SELINUX_ENABLED=1 bash util/build-gnu.sh
bash util/run-gnu-test.sh
# To run a single test:
bash util/run-gnu-test.sh tests/touch/not-owner.sh # for example
# To run several tests:
bash util/run-gnu-test.sh tests/touch/not-owner.sh tests/rm/no-give-up.sh # for example
# If this is a perl (.pl) test, to run in debug:
DEBUG=1 bash util/run-gnu-test.sh tests/misc/sm3sum.pl
```

***Tip:*** First time you run `bash util/build-gnu.sh` command, it will provide instructions on how to checkout GNU coreutils repository at the correct release tag. Please follow those instructions and when done, run `bash util/build-gnu.sh` command again.

You also need to install [quilt](https://savannah.nongnu.org/projects/quilt), a tool used to manage a stack of patches for modifying GNU tests.

On FreeBSD, you need to install packages for GNU coreutils and sed (used in shell scripts instead of system commands):

```shell
pkg install coreutils gsed
```

## Code coverage report

Code coverage report can be generated using [grcov](https://github.com/mozilla/grcov).

To generate [gcov-based](https://github.com/mozilla/grcov#example-how-to-generate-gcda-files-for-a-rust-project) coverage report

```shell
export CARGO_INCREMENTAL=0
export RUSTFLAGS="-Cinstrument-coverage -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort"
export RUSTDOCFLAGS="-Cpanic=abort"
cargo build <options...> # e.g., --features feat_os_unix
cargo test <options...> # e.g., --features feat_os_unix test_pathchk
grcov . -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing --ignore build.rs --excl-br-line "^\s*((debug_)?assert(_eq|_ne)?\#\[derive\()" -o ./target/debug/coverage/
# open target/debug/coverage/index.html in browser
```

if changes are not reflected in the report then run `cargo clean` and run the above commands.

## Tips for setting up on Mac

### C Compiler and linker

On MacOS you'll need to install C compiler & linker:

```shell
xcode-select --install
```

### MacOS GNU utils

On MacOS you will need to install [Homebrew](https://docs.brew.sh/Installation) and use it to install the following Homebrew formulas:

```shell
brew install \
  coreutils \
  autoconf \
  gettext \
  texinfo \
  xz \
  automake \
  gnu-sed \
  m4 \
  bison \
  pre-commit \
  findutils
```

After installing these Homebrew formulas, please make sure to add the following lines to your `zsh` or `bash` rc file, i.e. `~/.profile` or `~/.zshrc` or `~/.bashrc` ...
(assuming Homebrew is installed at default location `/opt/homebrew`):

```shell
eval "$(/opt/homebrew/bin/brew shellenv)"
export PATH="/opt/homebrew/opt/coreutils/libexec/gnubin:$PATH"
export PATH="/opt/homebrew/opt/bison/bin:$PATH"
export PATH="/opt/homebrew/opt/findutils/libexec/gnubin:$PATH"
```

Last step is to link Homebrew coreutils version of `timeout` to `/usr/local/bin` (as admin user):

```shell
sudo ln -s /opt/homebrew/bin/timeout /usr/local/bin/timeout
```

Do not forget to either source updated rc file or restart you terminal session to update environment variables.

## Tips for setting up on Windows

### MSVC build tools

On Windows you'll need the MSVC build tools for Visual Studio 2013 or later.

If you are using `rustup-init.exe` to install Rust toolchain, it will guide you through the process of downloading and installing these prerequisites.

Otherwise please follow [this guide](https://learn.microsoft.com/en-us/windows/dev-environment/rust/setup).

### Windows GNU utils

If you have used [Git for Windows](https://gitforwindows.org) to install `git` on you Windows system you might already have some GNU core utilities installed as part of "GNU Bash" included in Git for Windows package, but it is not a complete package. [This article](https://gist.github.com/evanwill/0207876c3243bbb6863e65ec5dc3f058) provides instruction on how to add more to it.

Alternatively you can install [Cygwin](https://www.cygwin.com) and/or use [WSL2](https://learn.microsoft.com/en-us/windows/wsl/compare-versions#whats-new-in-wsl-2) to get access to all GNU core utilities on Windows.

## Preparing a new release

1. Modify `util/update-version.sh` (FROM & TO) and run it
1. Submit a new PR with these changes and wait for it to be merged
1. Tag the new release `git tag -a X.Y.Z` and `git push --tags`
1. Once the CI is green, a new release will be automatically created in draft mode.
   Reuse this release and make sure that assets have been added.
1. Write the release notes (it takes time) following previous examples
1. Run `util/publish.sh --do-it` to publish the new release to crates.io
