echo-about = Display a line of text
echo-usage = echo [OPTIONS]... [STRING]...
echo-after-help = Echo the STRING(s) to standard output.

  If -e is in effect, the following sequences are recognized:

  - \ backslash
  - \a alert (BEL)
  - \b backspace
  - \c produce no further output
  - \e escape
  - \f form feed
  - \n new line
  - \r carriage return
  - \t horizontal tab
  - \v vertical tab
  - \0NNN byte with octal value NNN (1 to 3 digits)
  - \xHH byte with hexadecimal value HH (1 to 2 digits)

echo-help-no-newline = do not output the trailing newline
echo-help-enable-escapes = enable interpretation of backslash escapes
echo-help-disable-escapes = disable interpretation of backslash escapes (default)

echo-error-non-utf8 = Non-UTF-8 arguments provided, but this platform does not support them
