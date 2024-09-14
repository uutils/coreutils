# pr

```
pr [OPTION]... [FILE]...
```

Write content of given file or standard input to standard output with pagination filter

## After help

`+PAGE`           Begin output at page number page of the formatted input.
`-COLUMN`         Produce multi-column output. See `--column`

The pr utility is a printing and pagination filter for text files.
When multiple input files are specified, each is read, formatted, and written to standard output.
By default, the input is separated into 66-line pages, each with

* A 5-line header with the page number, date, time, and the pathname of the file.
* A 5-line trailer consisting of blank lines.

If standard output is associated with a terminal, diagnostic messages are suppressed until the pr
utility has completed processing.

When multiple column output is specified, text columns are of equal width.
By default, text columns are separated by at least one `<blank>`.
Input lines that do not fit into a text column are truncated.
Lines are not truncated under single column output.
