# Extensions over GNU

Though the main goal of the project is compatibility, uutils supports a few
features that are not supported by GNU coreutils. We take care not to introduce
features that are incompatible with the GNU coreutils. Below is a list of uutils
extensions.

## General

GNU coreutils provides two ways to define short options taking an argument:

```
$ ls -w 80
$ ls -w80
```

We support a third way:

```
$ ls -w=80
```

## `env`

`env` has an additional `-f`/`--file` flag that can parse `.env` files and set
variables accordingly. This feature is adopted from `dotenv` style packages.

## `cp`

`cp` can display a progress bar when the `-g`/`--progress` flag is set.

## `mv`

`mv` can display a progress bar when the `-g`/`--progress` flag is set.

## `hashsum`

This utility does not exist in GNU coreutils. `hashsum` is a utility that
supports computing the checksums with several algorithms. The flags and options
are identical to the `*sum` family of utils (`sha1sum`, `sha256sum`, `b2sum`,
etc.).

## `b3sum`

This utility does not exist in GNU coreutils. The behavior is modeled after both
the `b2sum` utility of GNU and the
[`b3sum`](https://github.com/BLAKE3-team/BLAKE3) utility by the BLAKE3 team and
supports the `--no-names` option that does not appear in the GNU util.

## `more`

We provide a simple implementation of `more`, which is not part of GNU
coreutils. We do not aim for full compatibility with the `more` utility from
`util-linux`. Features from more modern pagers (like `less` and `bat`) are
therefore welcomed.

## `cut`

`cut` can separate fields by whitespace (Space and Tab) with `-w` flag. This
feature is adopted from [FreeBSD](https://www.freebsd.org/cgi/man.cgi?cut).

## `fmt`

`fmt` has additional flags for prefixes: `-P`/`--skip-prefix`, `-x`/`--exact-prefix`, and
`-X`/`--exact-skip-prefix`. With `-m`/`--preserve-headers`, an attempt is made to detect and preserve
mail headers in the input. `-q`/`--quick` breaks lines more quickly. And `-T`/`--tab-width` defines the
number of spaces representing a tab when determining the line length.

## `seq`

`seq` provides `-t`/`--terminator` to set the terminator character.

## `ls`

GNU `ls` provides two ways to use a long listing format: `-l` and `--format=long`. We support a
third way: `--long`.

GNU `ls --sort=VALUE` only supports special non-default sort orders.
We support `--sort=name`, which makes it possible to override an earlier value.

## `du`

`du` allows `birth` and `creation` as values for the `--time` argument to show the creation time. It
also provides a `-v`/`--verbose` flag.
