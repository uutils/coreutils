unexpand-about = Convert blanks in each FILE to tabs, writing to standard output.
  With no FILE, or when FILE is -, read standard input.
unexpand-usage = unexpand [OPTION]... [FILE]...

# Help messages
unexpand-help-all = convert all blanks, instead of just initial blanks
unexpand-help-first-only = convert only leading sequences of blanks (overrides -a)
unexpand-help-tabs = use comma separated LIST of tab positions or have tabs N characters apart instead of 8 (enables -a)
unexpand-help-no-utf8 = interpret input file as 8-bit ASCII rather than UTF-8

# Error messages
unexpand-error-invalid-character = tab size contains invalid character(s): { $char }
unexpand-error-tab-size-cannot-be-zero = tab size cannot be 0
unexpand-error-tab-size-too-large = tab stop value is too large
unexpand-error-tab-sizes-must-be-ascending = tab sizes must be ascending
unexpand-error-is-directory = { $path }: Is a directory
