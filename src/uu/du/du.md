# du

```
du [OPTION]... [FILE]...
du [OPTION]... --files0-from=F
```

Estimate file space usage

## After Help

Display values are in units of the first available SIZE from --block-size,
and the DU_BLOCK_SIZE, BLOCK_SIZE and BLOCKSIZE environment variables.
Otherwise, units default to 1024 bytes (or 512 if POSIXLY_CORRECT is set).

SIZE is an integer and optional unit (example: 10M is 10*1024*1024).
Units are K, M, G, T, P, E, Z, Y (powers of 1024) or KB, MB,... (powers
of 1000).

PATTERN allows some advanced exclusions. For example, the following syntaxes
are supported:
`?` will match only one character
`*` will match zero or more characters
`{a,b}` will match a or b
