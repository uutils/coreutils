env-about = Set each NAME to VALUE in the environment and run COMMAND
env-usage = env [OPTION]... [-] [NAME=VALUE]... [COMMAND [ARG]...]
env-after-help = A mere - implies -i. If no COMMAND, print the resulting environment.

# Help messages
env-help-ignore-environment = start with an empty environment
env-help-chdir = change working directory to DIR
env-help-null = end each output line with a 0 byte rather than a newline (only valid when printing the environment)
env-help-file = read and set variables from a ".env"-style configuration file (prior to any unset and/or set)
env-help-unset = remove variable from the environment
env-help-debug = print verbose information for each processing step
env-help-split-string = process and split S into separate arguments; used to pass multiple arguments on shebang lines
env-help-argv0 = Override the zeroth argument passed to the command being executed. Without this option a default value of `command` is used.
env-help-ignore-signal = set handling of SIG signal(s) to do nothing
env-help-default-signal = reset handling of SIG signal(s) to the default action
env-help-block-signal = block delivery of SIG signal(s) while running COMMAND
env-help-list-signal-handling = list signal handling changes requested by preceding options

# Error messages
env-error-missing-closing-quote = no terminating quote in -S string at position { $position } for quote '{ $quote }'
env-error-invalid-backslash-at-end = invalid backslash at end of string in -S at position { $position } in context { $context }
env-error-backslash-c-not-allowed = '\c' must not appear in double-quoted -S string at position { $position }
env-error-invalid-sequence = invalid sequence '\{ $char }' in -S at position { $position }
env-error-missing-closing-brace = Missing closing brace at position { $position }
env-error-missing-variable = Missing variable name at position { $position }
env-error-missing-closing-brace-after-value = Missing closing brace after default value at position { $position }
env-error-unexpected-number = Unexpected character: '{ $char }', expected variable name must not start with 0..9 at position { $position }
env-error-expected-brace-or-colon = Unexpected character: '{ $char }', expected a closing brace ('{"}"}') or colon (':') at position { $position }
env-error-cannot-specify-null-with-command = cannot specify --null (-0) with command
env-error-invalid-signal = { $signal }: invalid signal
env-error-config-file = { $file }: { $error }
env-error-variable-name-issue = variable name issue (at { $position }): { $error }
env-error-generic = Error: { $error }
env-error-no-such-file = { $program }: No such file or directory
env-error-use-s-shebang = use -[v]S to pass options in shebang lines
env-error-cannot-unset = cannot unset '{ $name }': Invalid argument
env-error-cannot-unset-invalid = cannot unset { $name }: Invalid argument
env-error-must-specify-command-with-chdir = must specify command with --chdir (-C)
env-error-cannot-change-directory = cannot change directory to { $directory }: { $error }
env-error-argv0-not-supported = --argv0 is currently not supported on this platform
env-error-permission-denied = { $program }: Permission denied
env-error-unknown = unknown error: { $error }
env-error-failed-set-signal-action = failed to set signal action for signal { $signal }: { $error }

# Warning messages
env-warning-no-name-specified = no name specified for value { $value }
