comm-about = Compare two sorted files line by line.

  When FILE1 or FILE2 (not both) is -, read standard input.

  With no options, produce three-column output. Column one contains
  lines unique to FILE1, column two contains lines unique to FILE2,
  and column three contains lines common to both files.
comm-usage = comm [OPTION]... FILE1 FILE2

# Help messages
comm-help-column-1 = suppress column 1 (lines unique to FILE1)
comm-help-column-2 = suppress column 2 (lines unique to FILE2)
comm-help-column-3 = suppress column 3 (lines that appear in both files)
comm-help-delimiter = separate columns with STR
comm-help-zero-terminated = line delimiter is NUL, not newline
comm-help-total = output a summary
comm-help-check-order = check that the input is correctly sorted, even if all input lines are pairable
comm-help-no-check-order = do not check that the input is correctly sorted

# Error messages
comm-error-file-not-sorted = comm: file { $file_num } is not in sorted order
comm-error-input-not-sorted = comm: input is not in sorted order
comm-error-is-directory = Is a directory
comm-error-multiple-conflicting-delimiters = multiple conflicting output delimiters specified

# Other messages
comm-total = total
