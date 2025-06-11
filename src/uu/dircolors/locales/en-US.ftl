dircolors-about = Output commands to set the LS_COLORS environment variable.
dircolors-usage = dircolors [OPTION]... [FILE]
dircolors-after-help = If FILE is specified, read it to determine which colors to use for which
  file types and extensions. Otherwise, a precompiled database is used.
  For details on the format of these files, run 'dircolors --print-database'

# Help messages
dircolors-help-bourne-shell = output Bourne shell code to set LS_COLORS
dircolors-help-c-shell = output C shell code to set LS_COLORS
dircolors-help-print-database = print the byte counts
dircolors-help-print-ls-colors = output fully escaped colors for display

# Error messages
dircolors-error-shell-and-output-exclusive = the options to output non shell syntax,
  and to select a shell syntax are mutually exclusive
dircolors-error-print-database-and-ls-colors-exclusive = options --print-database and --print-ls-colors are mutually exclusive
dircolors-error-extra-operand-print-database = extra operand { $operand }
  file operands cannot be combined with --print-database (-p)
dircolors-error-no-shell-environment = no SHELL environment variable, and no shell type option given
dircolors-error-extra-operand = extra operand { $operand }
dircolors-error-expected-file-got-directory = expected file, got directory { $path }
dircolors-error-invalid-line-missing-token = { $file }:{ $line }: invalid line;  missing second token
dircolors-error-unrecognized-keyword = unrecognized keyword { $keyword }
