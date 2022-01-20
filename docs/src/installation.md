# Installation

## Requirements

* Rust (`cargo`, `rustc`)
* GNU Make (optional)

### Rust Version

uutils follows Rust's release channels and is tested against stable, beta and nightly.
The current oldest supported version of the Rust compiler is `1.54`.

On both Windows and Redox, only the nightly version is tested currently.

## Build Instructions

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

## Installation Instructions

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

## Un-installation Instructions

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
