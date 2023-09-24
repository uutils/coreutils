# echo

```
echo [OPTIONS]... [STRING]...
```

Display a line of text

## After Help

Echo the STRING(s) to standard output.

If -e is in effect, the following sequences are recognized:

- `\`       backslash
- `\a`      alert (BEL)
- `\b`      backspace
- `\c`      produce no further output
- `\e`      escape
- `\f`      form feed
- `\n`      new line
- `\r`      carriage return
- `\t`      horizontal tab
- `\v`      vertical tab
- `\0NNN`   byte with octal value NNN (1 to 3 digits)
- `\xHH`    byte with hexadecimal value HH (1 to 2 digits)
