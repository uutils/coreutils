# Common strings shared across all uutils commands
# Mostly clap

# Generic words
common-error = error
common-tip = tip
common-usage = Usage
common-help = help
common-version = version

# Common clap error messages
clap-error-unexpected-argument = { $error_word }: unexpected argument '{ $arg }' found
clap-error-similar-argument = { $tip_word }: a similar argument exists: '{ $suggestion }'
clap-error-pass-as-value = { $tip_word }: to pass '{ $arg }' as a value, use '{ $tip_command }'
clap-error-invalid-value = { $error_word }: invalid value '{ $value }' for '{ $option }'
clap-error-value-required = { $error_word }: a value is required for '{ $option }' but none was supplied
clap-error-possible-values = possible values
clap-error-help-suggestion = For more information, try '{ $command } --help'.
common-help-suggestion = For more information, try '--help'.

# Common help text patterns
help-flag-help = Print help information
help-flag-version = Print version information

# Common error contexts
error-io = I/O error
error-permission-denied = Permission denied
error-file-not-found = No such file or directory
error-invalid-argument = Invalid argument

# Common actions
action-copying = copying
action-moving = moving
action-removing = removing
action-creating = creating
action-reading = reading
action-writing = writing
