wc-about = Print newline, word, and byte counts for each FILE, and a total line if more than one FILE is specified.
wc-usage = wc [OPTION]... [FILE]...

# Help messages
wc-help-bytes = print the byte counts
wc-help-chars = print the character counts
wc-help-files0-from = read input from the files specified by
  NUL-terminated names in file F;
  If F is - then read names from standard input
wc-help-lines = print the newline counts
wc-help-max-line-length = print the length of the longest line
wc-help-total = when to print a line with total counts;
  WHEN can be: auto, always, only, never
wc-help-words = print the word counts

# Error messages
wc-error-files-disabled = extra operand { $extra }
  file operands cannot be combined with --files0-from
wc-error-stdin-repr-not-allowed = when reading file names from standard input, no file name of '-' allowed
wc-error-zero-length-filename = invalid zero-length file name
wc-error-zero-length-filename-ctx = { $path }:{ $idx }: invalid zero-length file name
wc-error-cannot-open-for-reading = cannot open { $path } for reading
wc-error-read-error = { $path }: read error
wc-error-failed-to-print-result = failed to print result for { $title }
wc-error-failed-to-print-total = failed to print total

# Decoder error messages
decoder-error-invalid-byte-sequence = invalid byte sequence: { $bytes }
decoder-error-io = underlying bytestream error: { $error }

# Other messages
wc-standard-input = standard input
wc-total = total

# Debug messages
wc-debug-hw-unavailable = debug: hardware support unavailable on this CPU
wc-debug-hw-using = debug: using hardware support (features: { $features })
wc-debug-hw-disabled-env = debug: hardware support disabled by environment
wc-debug-hw-disabled-glibc = debug: hardware support disabled by GLIBC_TUNABLES ({ $features })
wc-debug-hw-limited-glibc = debug: hardware support limited by GLIBC_TUNABLES (disabled: { $disabled }; enabled: { $enabled })
