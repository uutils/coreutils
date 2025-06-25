pr-about = paginate or columnate FILE(s) for printing
pr-after-help =
  If no FILE(s) are given, or if FILE is -, read standard input.

  When creating multicolumn output, columns will be of equal width. When using
  the '-s' option to separate columns, the default separator is a single tab
  character. When using the '-S' option to separate columns, the default separator
  is a single space character.
pr-usage = pr [OPTION]... [FILE]...

# Help messages
pr-help-pages = Begin and stop printing with page FIRST_PAGE[:LAST_PAGE]
pr-help-header =
  Use the string header to replace the file name
                  in the header line.
pr-help-double-space =
  Produce output that is double spaced. An extra <newline>
                  character is output following every <newline> found in the input.
pr-help-number-lines =
  Provide width digit line numbering.  The default for width,
                  if not specified, is 5.  The number occupies the first width column
                  positions of each text column or each line of -m output.  If char
                  (any non-digit character) is given, it is appended to the line number
                  to separate it from whatever follows.  The default for char is a <tab>.
                  Line numbers longer than width columns are truncated.
pr-help-first-line-number = start counting with NUMBER at 1st line of first page printed
pr-help-omit-header =
  Write neither the five-line identifying header nor the five-line
                  trailer usually supplied for each page. Quit writing after the last line
                   of each file without spacing to the end of the page.
pr-help-page-length =
  Override the 66-line default (default number of lines of text 56,
                  and with -F 63) and reset the page length to lines.  If lines is not
                  greater than the sum  of  both the  header  and trailer depths (in lines),
                  the pr utility shall suppress both the header and trailer, as if the -t
                  option were in effect.
pr-help-no-file-warnings = omit warning when a file cannot be opened
pr-help-form-feed =
  Use a <form-feed> for new pages, instead of the default behavior that
                  uses a sequence of <newline>s.
pr-help-column-width =
  Set the width of the line to width column positions for multiple
                  text-column output only. If the -w option is not specified and the -s option
                  is not specified, the default width shall be 72. If the -w option is not specified
                  and the -s option is specified, the default width shall be 512.
pr-help-page-width =
  set page width to PAGE_WIDTH (72) characters always,
                  truncate lines, except -J option is set, no interference
                  with -S or -s
pr-help-across =
  Modify the effect of the - column option so that the columns are filled
                  across the page in a  round-robin  order (for example, when column is 2, the
                  first input line heads column 1, the second heads column 2, the third is the
                  second line in column 1, and so on).
pr-help-column =
  Produce multi-column output that is arranged in column columns
                  (the default shall be 1) and is written down each column  in  the order in which
                  the text is received from the input file. This option should not be used with -m.
                  The options -e and -i shall be assumed for multiple text-column output.  Whether
                  or not text columns are produced with identical vertical lengths is unspecified,
                  but a text column shall never exceed the length of the page (see the -l option).
                  When used with -t, use the minimum number of lines to write the output.
pr-help-column-char-separator =
  Separate text columns by the single character char instead of by the
                  appropriate number of <space>s (default for char is the <tab> character).
pr-help-column-string-separator =
  separate columns by STRING,
                  without -S: Default separator <TAB> with -J and <space>
                  otherwise (same as -S\" \"), no effect on column options
pr-help-merge =
  Merge files. Standard output shall be formatted so the pr utility
                  writes one line from each file specified by a file operand, side by side
                  into text columns of equal fixed widths, in terms of the number of column
                  positions. Implementations shall support merging of at least nine file operands.
pr-help-indent =
  Each line of output shall be preceded by offset <space>s. If the -o
                  option is not specified, the default offset shall be zero. The space taken is
                  in addition to the output line width (see the -w option below).
pr-help-join-lines =
  merge full lines, turns off -W line truncation, no column
                  alignment, --sep-string[=STRING] sets separators
pr-help-help = Print help information

# Page header text
pr-page = Page

# Error messages
pr-error-reading-input = pr: Reading from input {$file} gave error
pr-error-unknown-filetype = pr: {$file}: unknown filetype
pr-error-is-directory = pr: {$file}: Is a directory
pr-error-socket-not-supported = pr: cannot open {$file}, Operation not supported on socket
pr-error-no-such-file = pr: cannot open {$file}, No such file or directory
pr-error-column-merge-conflict = cannot specify number of columns when printing in parallel
pr-error-across-merge-conflict = cannot specify both printing across and printing in parallel
pr-error-invalid-pages-range = invalid --pages argument '{$start}:{$end}'
