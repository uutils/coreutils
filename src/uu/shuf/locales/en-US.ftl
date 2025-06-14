shuf-about = Shuffle the input by outputting a random permutation of input lines.
  Each output permutation is equally likely.
  With no FILE, or when FILE is -, read standard input.
shuf-usage = shuf [OPTION]... [FILE]
  shuf -e [OPTION]... [ARG]...
  shuf -i LO-HI [OPTION]...

# Help messages
shuf-help-echo = treat each ARG as an input line
shuf-help-input-range = treat each number LO through HI as an input line
shuf-help-head-count = output at most COUNT lines
shuf-help-output = write result to FILE instead of standard output
shuf-help-random-source = get random bytes from FILE
shuf-help-repeat = output lines can be repeated
shuf-help-zero-terminated = line delimiter is NUL, not newline

# Error messages
shuf-error-unexpected-argument = unexpected argument { $arg } found
shuf-error-failed-to-open-for-writing = failed to open { $file } for writing
shuf-error-failed-to-open-random-source = failed to open random source { $file }
shuf-error-read-error = read error
shuf-error-no-lines-to-repeat = no lines to repeat
shuf-error-start-exceeds-end = start exceeds end
shuf-error-missing-dash = missing '-'
shuf-error-write-failed = write failed
