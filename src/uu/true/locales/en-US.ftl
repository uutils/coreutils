true-about = Returns true, a successful exit status.

  Immediately returns with the exit status 0, except when invoked with one of the recognized
  options. In those cases it will try to write the help or version text. Any IO error during this
  operation causes the program to return 1 instead.

true-help-text = Print help information
true-version-text = Print version information
