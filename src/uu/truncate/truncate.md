# truncate

```
truncate [OPTION]... [FILE]...
```

Shrink or extend the size of each file to the specified size.

## After help

SIZE is an integer with an optional prefix and optional unit.
The available units (K, M, G, T, P, E, Z, and Y) use the following format:
    'KB' =>           1000 (kilobytes)
    'K'  =>           1024 (kibibytes)
    'MB' =>      1000*1000 (megabytes)
    'M'  =>      1024*1024 (mebibytes)
    'GB' => 1000*1000*1000 (gigabytes)
    'G'  => 1024*1024*1024 (gibibytes)
SIZE may also be prefixed by one of the following to adjust the size of each
file based on its current size:
    '+'  => extend by
    '-'  => reduce by
    '<'  => at most
    '>'  => at least
    '/'  => round down to multiple of
    '%'  => round up to multiple of
