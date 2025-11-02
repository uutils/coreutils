<!-- markdownlint-disable MD033 MD041 MD002 -->
<!-- markdownlint-disable commands-show-output no-duplicate-heading -->
<!-- spell-checker:ignore markdownlint ; (options) DESTDIR UTILNAME manpages reimplementation oranda libclang -->
<div class="oranda-hide">
<div align="center">

![uutils logo](docs/src/logo.svg)

# uutils coreutils

[![Crates.io](https://img.shields.io/crates/v/coreutils.svg)](https://crates.io/crates/coreutils)
[![Discord](https://img.shields.io/badge/discord-join-7289DA.svg?logo=discord&longCache=true&style=flat)](https://discord.gg/wQVJbvJ)
[![License](http://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/uutils/coreutils/blob/main/LICENSE)
[![dependency status](https://deps.rs/repo/github/uutils/coreutils/status.svg)](https://deps.rs/repo/github/uutils/coreutils)

[![CodeCov](https://codecov.io/gh/uutils/coreutils/branch/main/graph/badge.svg)](https://codecov.io/gh/uutils/coreutils)
![MSRV](https://img.shields.io/badge/MSRV-1.85.0-brightgreen)
[![Weblate](https://hosted.weblate.org/widget/rust-coreutils/svg-badge.svg)](https://hosted.weblate.org/projects/rust-coreutils/)

</div>

---

</div>

uutils coreutils is a cross-platform reimplementation of the GNU coreutils in
[Rust](http://www.rust-lang.org). While all programs have been implemented, some
options might be missing or different behavior might be experienced.

<div class="oranda-hide">

To install it:

```shell
cargo install coreutils
~/.cargo/bin/coreutils
```

</div>

<!-- markdownlint-disable-next-line MD026 -->

## Goals

uutils coreutils aims to be a drop-in replacement for the GNU utils. Differences with GNU
are treated as bugs.

Our key objectives include:
- Matching GNU's output (stdout and error code) exactly
- Better error messages
- Providing comprehensive internationalization support (UTF-8)
- Improved performances
- [Extensions](docs/src/extensions.md) when relevant (example: --progress)

uutils aims to work on as many platforms as possible, to be able to use the same
utils on Linux, macOS, Windows and other platforms. This ensures, for example,
that scripts can be easily transferred between platforms.

<div class="oranda-hide">

## Documentation
uutils has both user and developer documentation available:

- [User Manual](https://uutils.github.io/coreutils/docs/)
- [Developer Documentation](https://docs.rs/crate/coreutils/)

Both can also be generated locally, the instructions for that can be found in
the [coreutils docs](https://github.com/uutils/uutils.github.io) repository.

Use [weblate/rust-coreutils](https://hosted.weblate.org/projects/rust-coreutils/) to translate the Rust coreutils into your language.

<!-- ANCHOR: build (this mark is needed for mdbook) -->

## Requirements

- Rust (`cargo`, `rustc`)
- GNU Make (optional)

### Rust Version

uutils follows Rust's release channels and is tested against stable, beta and
nightly. The current Minimum Supported Rust Version (MSRV) is `1.85.0`.

## Building

There are currently two methods to build the uutils binaries: either Cargo or
GNU Make.

> Building the full package, including all documentation, requires both Cargo
> and GNU Make on a Unix platform.

For either method, we first need to fetch the repository:

```shell
git clone https://github.com/uutils/coreutils
cd coreutils
```

### Cargo

Building uutils using Cargo is easy because the process is the same as for every
other Rust program:

```shell
cargo build --release
```

This command builds the most portable common core set of uutils into a multicall
(BusyBox-type) binary, named 'coreutils', on most Rust-supported platforms.

Additional platform-specific uutils are often available. Building these expanded
sets of uutils for a platform (on that platform) is as simple as specifying it
as a feature:

```shell
cargo build --release --features macos
# or ...
cargo build --release --features windows
# or ...
cargo build --release --features unix
```

To build SELinux-specific features, including `chcon` and `runcon`, ensure that `libselinux` 
and `libclang` are installed on your system. Then, run the following command:
```
cargo build --release --features unix,feat_selinux
```

If you don't want to build every utility available on your platform into the
final binary, you can also specify which ones you want to build manually. For
example:

```shell
cargo build --features "base32 cat echo rm" --no-default-features
```

If you want to build the utilities as individual binaries, that is also possible:

```shell
cargo build --release --bins --workspace --exclude coreutils --exclude uu_runcon --exclude uu_chcon
```
Each utility is contained in its own package within the main repository, named "uu_UTILNAME". To
build selected individual utilities, use the `--package` [aka `-p`] option. For example:

```shell
cargo build -p uu_base32 -p uu_cat -p uu_echo -p uu_rm
```

### GNU Make

Building using `make` is a simple process as well.

To simply build all available utilities:

```shell
make
```

In release mode:

```shell
make PROFILE=release
```

To build all but a few of the available utilities:

```shell
make SKIP_UTILS='UTILITY_1 UTILITY_2'
```

To build only a few of the available utilities:

```shell
make UTILS='UTILITY_1 UTILITY_2'
```

## Installation

### Install with Cargo

Likewise, installing can simply be done using:

```shell
cargo install --path . --locked
```

This command will install uutils into Cargo's _bin_ folder (_e.g._
`$HOME/.cargo/bin`).

This does not install files necessary for shell completion or manpages. For
manpages or shell completion to work, use `GNU Make` or see
`Manually install shell completions`/`Manually install manpages`.

### Install with GNU Make

To install all available utilities:

```shell
make install
```

To install using `sudo` switch `-E` must be used:

```shell
sudo -E make install
```

To install all but a few of the available utilities:

```shell
make SKIP_UTILS='UTILITY_1 UTILITY_2' install
```

To install only a few of the available utilities:

```shell
make UTILS='UTILITY_1 UTILITY_2' install
```

To install every program with a prefix (e.g. uu-echo uu-cat):

```shell
make PROG_PREFIX=PREFIX_GOES_HERE install
```

To install the multicall binary:

```shell
make MULTICALL=y install
```

Set install parent directory (default value is /usr/local):

```shell
# DESTDIR is also supported
make PREFIX=/my/path install
```

Installing with `make` installs shell completions for all installed utilities
for `bash`, `fish` and `zsh`. Completions for `elvish` and `powershell` can also
be generated; See `Manually install shell completions`.

To skip installation of completions and manpages:

```shell
make COMPLETIONS=n MANPAGES=n install
```

### Manually install shell completions

The `uudoc` binary generates completions for the `bash`, `elvish`,
`fish`, `powershell` and `zsh` shells to stdout.

Install `uudoc` by
```shell
cargo install --bin uudoc --features uudoc --path .
```

Then use the installed binary:
```shell
uudoc completion <utility> <shell>
```

So, to install completions for `ls` on `bash` to
`/usr/local/share/bash-completion/completions/ls`, run:

```shell
uudoc completion ls bash > /usr/local/share/bash-completion/completions/ls.bash
```

Completion for prefixed `cp` with `uu-` on `zsh` is generated by
```shell
env PROG_PREFIX=uu- uudoc completion cp zsh
```

### Manually install manpages

To generate manpages, the syntax is:

```bash
uudoc manpage <utility>
```

So, to install the manpage for `ls` to `/usr/local/share/man/man1/ls.1` run:

```bash
uudoc manpage ls > /usr/local/share/man/man1/ls.1
```

## Un-installation

Un-installation differs depending on how you have installed uutils. If you used
Cargo to install, use Cargo to uninstall. If you used GNU Make to install, use
Make to uninstall.

### Uninstall with Cargo

To uninstall uutils:

```shell
cargo uninstall coreutils
```

### Uninstall with GNU Make

To uninstall all utilities:

```shell
make uninstall
```

To uninstall every program with a set prefix:

```shell
make PROG_PREFIX=PREFIX_GOES_HERE uninstall
```

To uninstall the multicall binary:

```shell
make MULTICALL=y uninstall
```

To uninstall from a custom parent directory:

```shell
# DESTDIR is also supported
make PREFIX=/my/path uninstall
```

<!-- ANCHOR_END: build (this mark is needed for mdbook) -->

## GNU test suite compatibility

Below is the evolution of how many GNU tests uutils passes. A more detailed
breakdown of the GNU test results of the main branch can be found
[in the user manual](https://uutils.github.io/coreutils/docs/test_coverage.html).

See <https://github.com/orgs/uutils/projects/1> for the main meta bugs
(many are missing).

![Evolution over time](https://github.com/uutils/coreutils-tracking/blob/main/gnu-results.svg?raw=true)

</div> <!-- close oranda-hide div -->

## Contributing

To contribute to uutils, please see [CONTRIBUTING](CONTRIBUTING.md).

## License

uutils is licensed under the MIT License - see the `LICENSE` file for details

GNU Coreutils is licensed under the GPL 3.0 or later.
