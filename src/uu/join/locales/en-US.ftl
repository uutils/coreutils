join-about = For each pair of input lines with identical join fields, write a line to
  standard output. The default join field is the first, delimited by blanks.

  When FILE1 or FILE2 (not both) is -, read standard input.
join-usage = join [OPTION]... FILE1 FILE2

# Join help messages
join-help-a = also print unpairable lines from file FILENUM, where
  FILENUM is 1 or 2, corresponding to FILE1 or FILE2
join-help-v = like -a FILENUM, but suppress joined output lines
join-help-e = replace missing input fields with EMPTY
join-help-i = ignore differences in case when comparing fields
join-help-j = equivalent to '-1 FIELD -2 FIELD'
join-help-o = obey FORMAT while constructing output line
join-help-t = use CHAR as input and output field separator
join-help-1 = join on this FIELD of file 1
join-help-2 = join on this FIELD of file 2
join-help-check-order = check that the input is correctly sorted, even if all input lines are pairable
join-help-nocheck-order = do not check that the input is correctly sorted
join-help-header = treat the first line in each file as field headers, print them without trying to pair them
join-help-z = line delimiter is NUL, not newline

# Join error messages
join-error-io = io error: { $error }
join-error-non-utf8-tab = non-UTF-8 multi-byte tab
join-error-unprintable-separators = unprintable field separators are only supported on unix-like platforms
join-error-multi-character-tab = multi-character tab { $value }
join-error-both-files-stdin = both files cannot be standard input
join-error-invalid-field-specifier = invalid field specifier: { $spec }
join-error-invalid-file-number = invalid file number in field spec: { $spec }
join-error-invalid-file-number-simple = invalid file number: { $value }
join-error-invalid-field-number = invalid field number: { $value }
join-error-incompatible-fields = incompatible join fields { $field1 }, { $field2 }
join-error-not-sorted = { $file }:{ $line_num }: is not sorted: { $content }
join-error-input-not-sorted = input is not in sorted order
