csplit-about = Split a file into sections determined by context lines
csplit-usage = csplit [OPTION]... FILE PATTERN...
csplit-after-help = Output pieces of FILE separated by PATTERN(s) to files 'xx00', 'xx01', ..., and output byte counts of each piece to standard output.

# Help messages
csplit-help-suffix-format = use sprintf FORMAT instead of %02d
csplit-help-prefix = use PREFIX instead of 'xx'
csplit-help-keep-files = do not remove output files on errors
csplit-help-suppress-matched = suppress the lines matching PATTERN
csplit-help-digits = use specified number of digits instead of 2
csplit-help-quiet = do not print counts of output file sizes
csplit-help-elide-empty-files = remove empty output files

# Error messages
csplit-error-line-out-of-range = { $pattern }: line number out of range
csplit-error-line-out-of-range-on-repetition = { $pattern }: line number out of range on repetition { $repetition }
csplit-error-match-not-found = { $pattern }: match not found
csplit-error-match-not-found-on-repetition = { $pattern }: match not found on repetition { $repetition }
csplit-error-line-number-is-zero = 0: line number must be greater than zero
csplit-error-line-number-smaller-than-previous = line number '{ $current }' is smaller than preceding line number, { $previous }
csplit-error-invalid-pattern = { $pattern }: invalid pattern
csplit-error-invalid-number = invalid number: { $number }
csplit-error-suffix-format-incorrect = incorrect conversion specification in suffix
csplit-error-suffix-format-too-many-percents = too many % conversion specifications in suffix
csplit-error-not-regular-file = { $file } is not a regular file
csplit-warning-line-number-same-as-previous = line number '{ $line_number }' is the same as preceding line number
csplit-stream-not-utf8 = stream did not contain valid UTF-8
csplit-read-error = read error
csplit-write-split-not-created = trying to write to a split that was not created
