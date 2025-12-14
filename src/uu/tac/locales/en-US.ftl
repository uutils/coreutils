tac-about = Write each file to standard output, last line first.
tac-usage = tac [OPTION]... [FILE]...
tac-help-before = attach the separator before instead of after
tac-help-regex = interpret the sequence as a regular expression
tac-help-separator = use STRING as the separator instead of newline

# Error messages
tac-error-invalid-regex = invalid regular expression: { $error }
tac-error-invalid-argument = { $argument }: read error: Invalid argument
tac-error-file-not-found = failed to open { $filename } for reading: No such file or directory
tac-error-read-error = failed to read from { $filename }: { $error }
tac-error-write-error = failed to write to stdout: { $error }
