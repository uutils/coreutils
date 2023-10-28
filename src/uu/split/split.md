<!-- spell-checker:ignore PREFI -->

# split

```
split [OPTION]... [INPUT [PREFIX]]
```

Create output files containing consecutive or interleaved sections of input

## After Help

Output fixed-size pieces of INPUT to PREFIXaa, PREFIXab, ...; default size is 1000, and default PREFIX is 'x'. With no INPUT, or when INPUT is -, read standard input.

The SIZE argument is an integer and optional unit (example: 10K is 10*1024).
Units are K,M,G,T,P,E,Z,Y,R,Q (powers of 1024) or KB,MB,... (powers of 1000).
Binary prefixes can be used, too: KiB=K, MiB=M, and so on.

CHUNKS may be:

- N       split into N files based on size of input
- K/N     output Kth of N to stdout
- l/N     split into N files without splitting lines/records
- l/K/N   output Kth of N to stdout without splitting lines/records
- r/N     like 'l' but use round robin distribution
- r/K/N   likewise but only output Kth of N to stdout
