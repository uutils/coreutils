od-about = Dump files in octal and other formats
od-usage = od [OPTION]... [--] [FILENAME]...
  od [-abcdDefFhHiIlLoOsxX] [FILENAME] [[+][0x]OFFSET[.][b]]
  od --traditional [OPTION]... [FILENAME] [[+][0x]OFFSET[.][b] [[+][0x]LABEL[.][b]]]
od-after-help = Displays data in various human-readable formats. If multiple formats are
  specified, the output will contain all formats in the order they appear on the
  command line. Each format will be printed on a new line. Only the line
  containing the first format will be prefixed with the offset.

  If no filename is specified, or it is "-", stdin will be used. After a "--", no
  more options will be recognized. This allows for filenames starting with a "-".

  If a filename is a valid number which can be used as an offset in the second
  form, you can force it to be recognized as a filename if you include an option
  like "-j0", which is only valid in the first form.

  RADIX is one of o,d,x,n for octal, decimal, hexadecimal or none.

  BYTES is decimal by default, octal if prefixed with a "0", or hexadecimal if
  prefixed with "0x". The suffixes b, KB, K, MB, M, GB, G, will multiply the
  number with 512, 1000, 1024, 1000^2, 1024^2, 1000^3, 1024^3, 1000^2, 1024^2.

  OFFSET and LABEL are octal by default, hexadecimal if prefixed with "0x" or
  decimal if a "." suffix is added. The "b" suffix will multiply with 512.

  TYPE contains one or more format specifications consisting of:
      a for printable 7-bits ASCII
      c for utf-8 characters or octal for undefined characters
      d[SIZE] for signed decimal
      f[SIZE] for floating point
      o[SIZE] for octal
      u[SIZE] for unsigned decimal
      x[SIZE] for hexadecimal
  SIZE is the number of bytes which can be the number 1, 2, 4, 8 or 16,
      or C, I, S, L for 1, 2, 4, 8 bytes for integer types,
      or F, D, L for 4, 8, 16 bytes for floating point.
  Any type specification can have a "z" suffix, which will add a ASCII dump at
      the end of the line.

  If an error occurred, a diagnostic message will be printed to stderr, and the
  exit code will be non-zero.

# Error messages
od-error-invalid-endian = Invalid argument --endian={$endian}
od-error-invalid-inputs = Invalid inputs: {$msg}
od-error-too-large = value is too large
od-error-radix-invalid = Radix must be one of [o, d, x, n], got: {$radix}
od-error-radix-empty = Radix cannot be empty, and must be one of [o, d, x, n]
od-error-invalid-width = invalid width {$width}; using {$min} instead
od-error-missing-format-spec = missing format specification after '--format' / '-t'
od-error-unexpected-char = unexpected char '{$char}' in format specification {$spec}
od-error-invalid-number = invalid number {$number} in format specification {$spec}
od-error-invalid-size = invalid size '{$size}' in format specification {$spec}
od-error-invalid-offset = invalid offset: {$offset}
od-error-invalid-label = invalid label: {$label}
od-error-too-many-inputs = too many inputs after --traditional: {$input}
od-error-parse-failed = parse failed
od-error-invalid-suffix = invalid suffix in --{$option} argument {$value}
od-error-invalid-argument = invalid --{$option} argument {$value}
od-error-argument-too-large = --{$option} argument {$value} too large
od-error-skip-past-end = tried to skip past end of input

# Help messages
od-help-help = Print help information.
od-help-address-radix = Select the base in which file offsets are printed.
od-help-skip-bytes = Skip bytes input bytes before formatting and writing.
od-help-read-bytes = limit dump to BYTES input bytes
od-help-endian = byte order to use for multi-byte formats
od-help-a = named characters, ignoring high-order bit
od-help-b = octal bytes
od-help-c = ASCII characters or backslash escapes
od-help-d = unsigned decimal 2-byte units
od-help-d4 = unsigned decimal 4-byte units
od-help-format = select output format or formats
od-help-output-duplicates = do not use * to mark line suppression
od-help-width = output BYTES bytes per output line. 32 is implied when BYTES is not
                specified.
od-help-traditional = compatibility mode with one input, offset and label.
