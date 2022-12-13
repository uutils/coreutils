# Extensions over GNU

Though the main goal of the project is compatibility, uutils supports a few
features that are not supported by GNU coreutils. We take care not to introduce
features that are incompatible with the GNU coreutils. Below is a list of uutils
extensions.

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
