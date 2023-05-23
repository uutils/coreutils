# stdbuf

```
stdbuf [OPTION]... COMMAND
```

Run `COMMAND`, with modified buffering operations for its standard streams.

Mandatory arguments to long options are mandatory for short options too.

## After Help

If `MODE` is 'L' the corresponding stream will be line buffered.
This option is invalid with standard input.

If `MODE` is '0' the corresponding stream will be unbuffered.

Otherwise, `MODE` is a number which may be followed by one of the following:

KB 1000, K 1024, MB 1000*1000, M 1024*1024, and so on for G, T, P, E, Z, Y.
In this case the corresponding stream will be fully buffered with the buffer size set to `MODE` bytes.

NOTE: If `COMMAND` adjusts the buffering of its standard streams (`tee` does for e.g.) then that will override corresponding settings changed by `stdbuf`.
Also some filters (like `dd` and `cat` etc.) don't use streams for I/O, and are thus unaffected by `stdbuf` settings.
