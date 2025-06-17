numfmt-about = Convert numbers from/to human-readable strings
numfmt-usage = numfmt [OPTION]... [NUMBER]...
numfmt-after-help = UNIT options:

  - none: no auto-scaling is done; suffixes will trigger an error
  - auto: accept optional single/two letter suffix:

      1K = 1000, 1Ki = 1024, 1M = 1000000, 1Mi = 1048576,

  - si: accept optional single letter suffix:

      1K = 1000, 1M = 1000000, ...

  - iec: accept optional single letter suffix:

      1K = 1024, 1M = 1048576, ...

  - iec-i: accept optional two-letter suffix:

      1Ki = 1024, 1Mi = 1048576, ...

  - FIELDS supports cut(1) style field ranges:

      N N'th field, counted from 1
      N- from N'th field, to end of line
      N-M from N'th to M'th field (inclusive)
      -M from first to M'th field (inclusive)
      - all fields

  Multiple fields/ranges can be separated with commas

  FORMAT must be suitable for printing one floating-point argument %f.
  Optional quote (%'f) will enable --grouping (if supported by current locale).
  Optional width value (%10f) will pad output. Optional zero (%010f) width
  will zero pad the number. Optional negative values (%-10f) will left align.
  Optional precision (%.1f) will override the input determined precision.

# Help messages
numfmt-help-delimiter = use X instead of whitespace for field delimiter
numfmt-help-field = replace the numbers in these input fields; see FIELDS below
numfmt-help-format = use printf style floating-point FORMAT; see FORMAT below for details
numfmt-help-from = auto-scale input numbers to UNITs; see UNIT below
numfmt-help-from-unit = specify the input unit size
numfmt-help-to = auto-scale output numbers to UNITs; see UNIT below
numfmt-help-to-unit = the output unit size
numfmt-help-padding = pad the output to N characters; positive N will right-align; negative N will left-align; padding is ignored if the output is wider than N; the default is to automatically pad if a whitespace is found
numfmt-help-header = print (without converting) the first N header lines; N defaults to 1 if not specified
numfmt-help-round = use METHOD for rounding when scaling
numfmt-help-suffix = print SUFFIX after each formatted number, and accept inputs optionally ending with SUFFIX
numfmt-help-invalid = set the failure mode for invalid input
numfmt-help-zero-terminated = line delimiter is NUL, not newline

# Error messages
numfmt-error-unsupported-unit = Unsupported unit is specified
numfmt-error-invalid-unit-size = invalid unit size: { $size }
numfmt-error-invalid-padding = invalid padding value { $value }
numfmt-error-invalid-header = invalid header value { $value }
numfmt-error-grouping-cannot-be-combined-with-to = grouping cannot be combined with --to
numfmt-error-delimiter-must-be-single-character = the delimiter must be a single character
numfmt-error-invalid-number-empty = invalid number: ''
numfmt-error-invalid-suffix = invalid suffix in input: { $input }
numfmt-error-invalid-number = invalid number: { $input }
numfmt-error-missing-i-suffix = missing 'i' suffix in input: '{ $number }{ $suffix }' (e.g Ki/Mi/Gi)
numfmt-error-rejecting-suffix = rejecting suffix in input: '{ $number }{ $suffix }' (consider using --from)
numfmt-error-suffix-unsupported-for-unit = This suffix is unsupported for specified unit
numfmt-error-unit-auto-not-supported-with-to = Unit 'auto' isn't supported with --to options
numfmt-error-number-too-big = Number is too big and unsupported
numfmt-error-format-no-percent = format '{ $format }' has no % directive
numfmt-error-format-ends-in-percent = format '{ $format }' ends in %
numfmt-error-invalid-format-directive = invalid format '{ $format }', directive must be %[0]['][-][N][.][N]f
numfmt-error-invalid-format-width-overflow = invalid format '{ $format }' (width overflow)
numfmt-error-invalid-precision = invalid precision in format '{ $format }'
numfmt-error-format-too-many-percent = format '{ $format }' has too many % directives
numfmt-error-unknown-invalid-mode = Unknown invalid mode: { $mode }
