# uniq

```
uniq [OPTION]... [INPUT [OUTPUT]]
```

Report or omit repeated lines.

## After help

Filter adjacent matching lines from `INPUT` (or standard input),
writing to `OUTPUT` (or standard output).

Note: `uniq` does not detect repeated lines unless they are adjacent.
You may want to sort the input first, or use `sort -u` without `uniq`.
