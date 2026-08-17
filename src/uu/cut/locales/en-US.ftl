cut-about = Extract sections from each line of files.
cut-usage = cut OPTION... [FILE]...
cut-after-help = If no FILE is specified, reads from stdin. Use '-' as a FILE argument to include stdin.

  LIST is a comma-separated list of numbers or ranges:

    N           only column N
    N-          columns N through the end of the line
    N-M         columns N through M
    -M          columns 1 through M

  Examples:

    cut -f 2,5-7 file.txt             extract fields 2, 5, 6, and 7
    cut -f 3- file.txt                extract field 3 to the end of the line
    cut --complement -f 4-6 file.txt  extract all fields except 4, 5, and 6
    cut -d',' -f1 file.csv            extract first field from a CSV

# Help messages
cut-help-bytes = extract bytes listed in LIST
cut-help-characters = extract characters listed in LIST
cut-help-delimiter = use DELIM as field separator (default: Tab)
cut-help-whitespace-delimited = use any whitespace (Space/Tab) as delimiter; ignore leading and trailing blanks with 'trimmed'
cut-help-fields = extract fields listed in LIST
cut-help-fields-merged = like -f, but merge adjacent delimiters; the delimiter defaults to whitespace and the output delimiter to a space
cut-help-complement = invert selection: print all columns except the specified ones
cut-help-only-delimited = suppress lines that do not contain the delimiter
cut-help-zero-terminated = use NULL (\0) instead of newline as line terminator
cut-help-output-delimiter = replace input delimiter with NEW_DELIM in output
cut-help-no-partial = with -b, do not output partial multi-byte characters

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
