# mknod

```
mknod [OPTION]... NAME TYPE [MAJOR MINOR]
```

Create the special file NAME of the given TYPE.

## After Help

Mandatory arguments to long options are mandatory for short options too.
`-m`, `--mode=MODE`    set file permission bits to `MODE`, not `a=rw - umask`

Both `MAJOR` and `MINOR` must be specified when `TYPE` is `b`, `c`, or `u`, and they
must be omitted when `TYPE` is `p`.  If `MAJOR` or `MINOR` begins with `0x` or `0X`,
it is interpreted as hexadecimal; otherwise, if it begins with 0, as octal;
otherwise, as decimal.  `TYPE` may be:

* `b`      create a block (buffered) special file
* `c`, `u`   create a character (unbuffered) special file
* `p`      create a FIFO

NOTE: your shell may have its own version of mknod, which usually supersedes
the version described here.  Please refer to your shell's documentation
for details about the options it supports.
