tee-about = Copy standard input to each FILE, and also to standard output.
tee-usage = tee [OPTION]... [FILE]...
tee-after-help = If a FILE is -, it refers to a file named - .

# Help messages
tee-help-help = Print help
tee-help-append = append to the given FILEs, do not overwrite
tee-help-ignore-interrupts = ignore interrupt signals (ignored on non-Unix platforms)
tee-help-ignore-pipe-errors = set write error behavior (ignored on non-Unix platforms)
tee-help-output-error = set write error behavior
tee-help-output-error-warn = produce warnings for errors writing to any output
tee-help-output-error-warn-nopipe = produce warnings for errors that are not pipe errors (ignored on non-unix platforms)
tee-help-output-error-exit = exit on write errors to any output
tee-help-output-error-exit-nopipe = exit on write errors to any output that are not pipe errors (equivalent to exit on non-unix platforms)

# Error messages
tee-error-stdin = stdin: { $error }

# Other messages
tee-standard-output = 'standard output'
