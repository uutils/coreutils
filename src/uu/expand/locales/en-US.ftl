expand-about = Convert tabs in each FILE to spaces, writing to standard output.
  With no FILE, or when FILE is -, read standard input.
expand-usage = expand [OPTION]... [FILE]...

# Help messages
expand-help-initial = do not convert tabs after non blanks
expand-help-tabs = have tabs N characters apart, not 8 or use comma separated list of explicit tab positions
expand-help-no-utf8 = interpret input file as 8-bit ASCII rather than UTF-8

# Error messages
expand-error-invalid-character = tab size contains invalid character(s): { $char }
expand-error-specifier-not-at-start = { $specifier } specifier not at start of number: { $number }
expand-error-specifier-only-allowed-with-last = { $specifier } specifier only allowed with the last value
expand-error-tab-size-cannot-be-zero = tab size cannot be 0
expand-error-tab-size-too-large = tab stop is too large { $size }
expand-error-tab-sizes-must-be-ascending = tab sizes must be ascending
expand-error-is-directory = { $file }: Is a directory
expand-error-failed-to-write-output = failed to write output
