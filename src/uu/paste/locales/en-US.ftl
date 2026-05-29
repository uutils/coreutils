paste-about = Write lines consisting of the sequentially corresponding lines from each
  FILE, separated by TABs, to standard output.
paste-usage = paste [OPTIONS] [FILE]...

# Help messages
paste-help-serial = paste one file at a time instead of in parallel
paste-help-delimiter = reuse characters from LIST instead of TABs
paste-help-zero-terminated = line delimiter is NUL, not newline

# Error messages
paste-error-delimiter-unescaped-backslash = delimiter list ends with an unescaped backslash: { $delimiters }
paste-error-stdin-borrow = failed to access standard input: { $error }
