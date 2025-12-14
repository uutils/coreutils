truncate-about = Shrink or extend the size of each file to the specified size.
truncate-usage = truncate [OPTION]... [FILE]...
truncate-after-help = SIZE is an integer with an optional prefix and optional unit.
  The available units (K, M, G, T, P, E, Z, and Y) use the following format:
      'KB' => 1000 (kilobytes)
      'K' => 1024 (kibibytes)
      'MB' => 1000*1000 (megabytes)
      'M' => 1024*1024 (mebibytes)
      'GB' => 1000*1000*1000 (gigabytes)
      'G' => 1024*1024*1024 (gibibytes)
  SIZE may also be prefixed by one of the following to adjust the size of each
  file based on its current size:
      '+' => extend by
      '-' => reduce by
      '<' => at most
      '>' => at least
      '/' => round down to multiple of
      '%' => round up to multiple of

# Help messages
truncate-help-io-blocks = treat SIZE as the number of I/O blocks of the file rather than bytes (NOT IMPLEMENTED)
truncate-help-no-create = do not create files that do not exist
truncate-help-reference = base the size of each file on the size of RFILE
truncate-help-size = set or adjust the size of each file according to SIZE, which is in bytes unless --io-blocks is specified

# Error messages
truncate-error-missing-file-operand = missing file operand
truncate-error-cannot-open-no-device = cannot open { $filename } for writing: No such device or address
truncate-error-cannot-open-for-writing = cannot open { $filename } for writing
truncate-error-invalid-number = Invalid number: { $error }
truncate-error-must-specify-relative-size = you must specify a relative '--size' with '--reference'
truncate-error-division-by-zero = division by zero
truncate-error-cannot-stat-no-such-file = cannot stat { $filename }: No such file or directory
