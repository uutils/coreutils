# od

```
od [OPTION]... [--] [FILENAME]...
od [-abcdDefFhHiIlLoOsxX] [FILENAME] [[+][0x]OFFSET[.][b]]
od --traditional [OPTION]... [FILENAME] [[+][0x]OFFSET[.][b] [[+][0x]LABEL[.][b]]]
```

Dump files in octal and other formats

## After Help

Displays data in various human-readable formats. If multiple formats are
specified, the output will contain all formats in the order they appear on the
command line. Each format will be printed on a new line. Only the line
containing the first format will be prefixed with the offset.

If no filename is specified, or it is "-", stdin will be used. After a "--", no
more options will be recognized. This allows for filenames starting with a "-".

If a filename is a valid number which can be used as an offset in the second
form, you can force it to be recognized as a filename if you include an option
like "-j0", which is only valid in the first form.

RADIX is one of o,d,x,n for octal, decimal, hexadecimal or none.

BYTES is decimal by default, octal if prefixed with a "0", or hexadecimal if
prefixed with "0x". The suffixes b, KB, K, MB, M, GB, G, will multiply the
number with 512, 1000, 1024, 1000^2, 1024^2, 1000^3, 1024^3, 1000^2, 1024^2.

OFFSET and LABEL are octal by default, hexadecimal if prefixed with "0x" or
decimal if a "." suffix is added. The "b" suffix will multiply with 512.

TYPE contains one or more format specifications consisting of:
    a       for printable 7-bits ASCII
    c       for utf-8 characters or octal for undefined characters
    d[SIZE] for signed decimal
    f[SIZE] for floating point
    o[SIZE] for octal
    u[SIZE] for unsigned decimal
    x[SIZE] for hexadecimal
SIZE is the number of bytes which can be the number 1, 2, 4, 8 or 16,
    or C, I, S, L for 1, 2, 4, 8 bytes for integer types,
    or F, D, L for 4, 8, 16 bytes for floating point.
Any type specification can have a "z" suffix, which will add a ASCII dump at
    the end of the line.

If an error occurred, a diagnostic message will be printed to stderr, and the
exit code will be non-zero.
