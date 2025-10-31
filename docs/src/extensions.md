<!-- spell-checker:ignore hhhhp armv7 cccccccccccccccccccccccccccccccdp ccccccccccccccd ccccccccccccccdp fffffffp -->

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

With GNU coreutils, `--help` usually prints the help message and `--version` prints the version.
We also commonly provide short options: `-h` for help and `-V` for version.

## `env`

GNU `env` allows the empty string to be used as an environment variable name.
This is unsupported by uutils, which will show a warning for any such
assignment.

 `env` has an additional `-f`/`--file` flag that can
parse `.env` files and set variables accordingly. This feature is adopted from `dotenv` style
packages.

## `cp`

`cp` can display a progress bar when the `-g`/`--progress` flag is set.

## `mv`

`mv` can display a progress bar when the `-g`/`--progress` flag is set.

## `rm`

`rm` can display a progress bar when the `-g`/`--progress` flag is set.

## `hashsum` (deprecated)

This utility does not exist in GNU coreutils. `hashsum` is a utility that
supports computing the checksums with several algorithms. The flags and options
are identical to the `*sum` family of utils (`sha1sum`, `sha256sum`, `b2sum`,
etc.). This utility will be removed in the future and it is advised to use `cksum --untagged` instead.

## `b3sum`

This utility does not exist in GNU coreutils. The behavior is modeled after both
the `b2sum` utility of GNU and the
[`b3sum`](https://github.com/BLAKE3-team/BLAKE3) utility by the BLAKE3 team. It also
supports the `--no-names` option, that does not appear in the GNU utility.

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

## `printf`

`printf` uses arbitrary precision decimal numbers to parse and format floating point
numbers. GNU coreutils uses `long double`, whose actual size may be [double precision
64-bit float](https://en.wikipedia.org/wiki/Double-precision_floating-point_format)
(e.g 32-bit arm), [extended precision 80-bit float](https://en.wikipedia.org/wiki/Extended_precision)
(x86(-64)), or
[quadruple precision 128-bit float](https://en.wikipedia.org/wiki/Quadruple-precision_floating-point_format) (e.g. arm64).

Practically, this means that printing a number with high precision will remain exact:
```
printf "%.48f\n" 0.1
0.100000000000000000000000000000000000000000000000 << uutils on all platforms
0.100000000000000000001355252715606880542509316001 << GNU coreutils on x86(-64)
0.100000000000000000000000000000000004814824860968 << GNU coreutils on arm64
0.100000000000000005551115123125782702118158340454 << GNU coreutils on armv7 (32-bit)
```

### Hexadecimal floats

For hexadecimal float format (`%a`), POSIX only states that one hexadecimal number
should be present left of the decimal point (`0xh.hhhhp±d` [1]), but does not say how
many _bits_ should be included (between 1 and 4). On x86(-64), the first digit always
includes 4 bits, so its value is always between `0x8` and `0xf`, while on other
architectures, only 1 bit is included, so the value is always `0x1`.

However, the first digit will of course be `0x0` if the number is zero. Also,
rounding numbers may cause the first digit to be `0x1` on x86(-64) (e.g.
`0xf.fffffffp-5` rounds to `0x1.00p-1`), or `0x2` on other architectures.

We chose to replicate x86-64 behavior on all platforms.

Additionally, the default precision of the hexadecimal float format (`%a` without
any specifier) is expected to be "sufficient for exact representation of the value" [1].
This is not possible in uutils as we store arbitrary precision numbers that may be
periodic in hexadecimal form (`0.1 = 0xc.ccc...p-7`), so we revert
to the number of digits that would be required to exactly print an
[extended precision 80-bit float](https://en.wikipedia.org/wiki/Extended_precision),
emulating GNU coreutils behavior on x86(-64). An 80-bit float has 64 bits in its
integer and fractional part, so 16 hexadecimal digits are printed in total (1 digit
before the decimal point, 15 after).

Practically, this means that the default hexadecimal floating point output is
identical to x86(-64) GNU coreutils:
```
printf "%a\n" 0.1
0xc.ccccccccccccccdp-7 << uutils on all platforms
0xc.ccccccccccccccdp-7 << GNU coreutils on x86-64
0x1.999999999999999999999999999ap-4 << GNU coreutils on arm64
0x1.999999999999ap-4   << GNU coreutils on armv7 (32-bit)
```

We _can_ print an arbitrary number of digits if a larger precision is requested,
and the leading digit will still be in the `0x8`-`0xf` range:
```
printf "%.32a\n" 0.1
0xc.cccccccccccccccccccccccccccccccdp-7 << uutils on all platforms
0xc.ccccccccccccccd00000000000000000p-7 << GNU coreutils on x86-64
0x1.999999999999999999999999999a0000p-4 << GNU coreutils on arm64
0x1.999999999999a0000000000000000000p-4 << GNU coreutils on armv7 (32-bit)
```

***Note: The architecture-specific behavior on non-x86(-64) platforms may change in
the future.***

## `seq`

Unlike GNU coreutils, `seq` always uses arbitrary precision decimal numbers, no
matter the parameters (integers, decimal numbers, positive or negative increments,
format specified, etc.), so its output will be more correct than GNU coreutils for
some inputs (e.g. small fractional increments where GNU coreutils uses `long double`).

The only limitation is that the position of the decimal point is stored in a `i64`,
so values smaller than 10**(-2**63) will underflow to 0, and some values larger
than 10**(2**63) may overflow to infinity.

See also comments under `printf` for formatting precision and differences.

`seq` provides `-t`/`--terminator` to set the terminator character.

## `sort`

When sorting with `-g`/`--general-numeric-sort`, arbitrary precision decimal numbers
are parsed and compared, unlike GNU coreutils that uses platform-specific long
double floating point numbers.

Extremely large or small values can still overflow or underflow to infinity or zero,
see note in `seq`.

## `ls`

GNU `ls` provides two ways to use a long listing format: `-l` and `--format=long`. We support a
third way: `--long`.

GNU `ls --sort=VALUE` only supports special non-default sort orders.
We support `--sort=name`, which makes it possible to override an earlier value.

## `du`

`du` allows `birth` and `creation` as values for the `--time` argument to show the creation time. It
also provides a `-v`/`--verbose` flag.

## `id`

`id` has three additional flags:
* `-P` displays the id as a password file entry
* `-p` makes the output human-readable
* `-A` displays the process audit user ID

## `uptime`

Similar to the proc-ps implementation and unlike GNU/Coreutils, `uptime` provides `-s`/`--since` to show since when the system is up.

## `base32/base64/basenc`

Just like on macOS, `base32/base64/basenc` provides `-D` to decode data.

## `shred`

The number of random passes is deterministic in both GNU and uutils. However, uutils `shred` computes the number of random passes in a simplified way, specifically `max(3, x / 10)`, which is very close but not identical to the number of random passes that GNU would do. This also satisfies an expectation that reasonable users might have, namely that the number of random passes increases monotonically with the number of passes overall; GNU `shred` violates this assumption.

## `unexpand`

GNU `unexpand` provides `--first-only` to convert only leading sequences of blanks. We support a
second way: `-f` like busybox.

With `-U`/`--no-utf8`, you can interpret input files as 8-bit ASCII rather than UTF-8.

## `expand`

`expand` also offers the `-U`/`--no-utf8` option to interpret input files as 8-bit ASCII instead of UTF-8.
