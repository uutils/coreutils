cut-about = Prints specified byte or field columns from each line of stdin or the input files
cut-usage = cut OPTION... [FILE]...
cut-after-help = Each call must specify a mode (what to use for columns),
  a sequence (which columns to print), and provide a data source

  Specifying a mode:

  Use --bytes (-b) or --characters (-c) to specify byte mode

  Use --fields (-f) to specify field mode, where each line is broken into
  fields identified by a delimiter character. For example for a typical CSV
  you could use this in combination with setting comma as the delimiter

  Specifying a sequence:

  A sequence is a group of 1 or more numbers or inclusive ranges separated
  by a commas.

  cut -f 2,5-7 some_file.txt

  will display the 2nd, 5th, 6th, and 7th field for each source line

  Ranges can extend to the end of the row by excluding the second number

  cut -f 3- some_file.txt

  will display the 3rd field and all fields after for each source line

  The first number of a range can be excluded, and this is effectively the
  same as using 1 as the first number: it causes the range to begin at the
  first column. Ranges can also display a single column

  cut -f 1,3-5 some_file.txt

  will display the 1st, 3rd, 4th, and 5th field for each source line

  The --complement option, when used, inverts the effect of the sequence

  cut --complement -f 4-6 some_file.txt

  will display the every field but the 4th, 5th, and 6th

  Specifying a data source:

  If no sourcefile arguments are specified, stdin is used as the source of
  lines to print

  If sourcefile arguments are specified, stdin is ignored and all files are
  read in consecutively if a sourcefile is not successfully read, a warning
  will print to stderr, and the eventual status code will be 1, but cut
  will continue to read through proceeding sourcefiles

  To print columns from both STDIN and a file argument, use - (dash) as a
  sourcefile argument to represent stdin.

  Field mode options:

  The fields in each line are identified by a delimiter (separator)

  Set the delimiter:

  Set the delimiter which separates fields in the file using the
  --delimiter (-d) option. Setting the delimiter is optional.
  If not set, a default delimiter of Tab will be used.

  If the -w option is provided, fields will be separated by any number
  of whitespace characters (Space and Tab). The output delimiter will
  be a Tab unless explicitly specified. Only one of -d or -w option can be specified.
  This is an extension adopted from FreeBSD.

  Optionally filter based on delimiter:

  If the --only-delimited (-s) flag is provided, only lines which
  contain the delimiter will be printed

  Replace the delimiter:

  If the --output-delimiter option is provided, the argument used for
  it will replace the delimiter character in each line printed. This is
  useful for transforming tabular data - e.g. to convert a CSV to a
  TSV (tab-separated file)

  Line endings:

  When the --zero-terminated (-z) option is used, cut sees \\0 (null) as the
  'line ending' character (both for the purposes of reading lines and
  separating printed lines) instead of \\n (newline). This is useful for
  tabular data where some of the cells may contain newlines

  echo 'ab\\0cd' | cut -z -c 1

  will result in 'a\\0c\\0'

# Help messages
cut-help-bytes = filter byte columns from the input source
cut-help-characters = alias for character mode
cut-help-delimiter = specify the delimiter character that separates fields in the input source. Defaults to Tab.
cut-help-whitespace-delimited = Use any number of whitespace (Space, Tab) to separate fields in the input source (FreeBSD extension).
cut-help-fields = filter field columns from the input source
cut-help-fields-merged = like -f, but merge adjacent delimiters; the delimiter defaults to whitespace and the output delimiter to a space
cut-help-complement = invert the filter - instead of displaying only the filtered columns, display all but those columns
cut-help-only-delimited = in field mode, only print lines which contain the delimiter
cut-help-zero-terminated = instead of filtering columns based on line, filter columns based on \\0 (NULL character)
cut-help-output-delimiter = in field mode, replace the delimiter in output lines with this option's argument
cut-help-no-partial = with -b, don't output partial multi-byte characters

# Error messages
cut-error-is-directory = Is a directory
cut-error-write-error = write error
cut-error-delimiter-and-whitespace-conflict = -d and -w are mutually exclusive
cut-error-delimiter-must-be-single-character = the delimiter must be a single character
cut-error-multiple-mode-args = only one list may be specified
cut-error-missing-mode-arg = you must specify a list of bytes, characters, or fields
cut-error-delimiter-only-with-fields = an input delimiter makes sense{ "\u000A\u0009" }only when operating on fields
cut-error-only-delimited-only-with-fields = suppressing non-delimited lines makes sense{ "\u000A\u0009" }only when operating on fields
cut-error-field-numbered-from-1 = fields are numbered from 1
cut-error-position-numbered-from-1 = byte/character positions are numbered from 1
cut-error-invalid-field-range = invalid field range
cut-error-invalid-position-range = invalid byte or character range
cut-error-invalid-decreasing-range = invalid decreasing range
cut-error-invalid-range-no-endpoint = invalid range with no endpoint: { $range }
cut-error-invalid-field-value = invalid field value { $value }
cut-error-invalid-position-value = invalid byte/character position { $value }
cut-error-field-number-too-large = field number { $value } is too large
cut-error-position-too-large = byte/character offset { $value } is too large

# Diagnostic labels: what the caret points at in a list of ranges
cut-diag-label-zero-bound = counting starts at 1
cut-diag-help-list-syntax = a list is N, N-M, N- or -M, separated by commas, as in -f1,4-6,9-
