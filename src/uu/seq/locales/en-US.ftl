seq-about = Display numbers from FIRST to LAST, in steps of INCREMENT.
seq-usage = seq [OPTION]... LAST
  seq [OPTION]... FIRST LAST
  seq [OPTION]... FIRST INCREMENT LAST

# Help messages
seq-help-separator = Separator character (defaults to \n)
seq-help-terminator = Terminator character (defaults to \n)
seq-help-equal-width = Equalize widths of all numbers by padding with zeros
seq-help-format = use printf style floating-point FORMAT

# Error messages
seq-error-parse = invalid { $type } argument: { $arg }
seq-error-zero-increment = invalid Zero increment value: { $arg }
seq-error-no-arguments = missing operand
seq-error-format-and-equal-width = format string may not be specified when printing equal width strings

# Parse error types
seq-parse-error-type-float = floating point
seq-parse-error-type-nan = 'not-a-number'
