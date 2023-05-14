<!-- spell-checker:ignore MbdfhnRrV -->

# sort

```
sort [OPTION]... [FILE]...
```

Display sorted concatenation of all FILE(s). With no FILE, or when FILE is -, read standard input.

## After help

The key format is `FIELD[.CHAR][OPTIONS][,FIELD[.CHAR]][OPTIONS]`.

Fields by default are separated by the first whitespace after a non-whitespace character. Use `-t` to specify a custom separator.
In the default case, whitespace is appended at the beginning of each field. Custom separators however are not included in fields.

`FIELD` and `CHAR` both start at 1 (i.e. they are 1-indexed). If there is no end specified after a comma, the end will be the end of the line.
If `CHAR` is set 0, it means the end of the field. `CHAR` defaults to 1 for the start position and to 0 for the end position.

Valid options are: `MbdfhnRrV`. They override the global options for this key.
