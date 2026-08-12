# Common strings shared across all uutils commands
# Mostly clap

# Generic words
common-error = error
common-tip = tip
common-usage = Usage
common-help = help
common-version = version
common-write-error = write error

# Common clap error messages
clap-error-unexpected-argument = { $error_word }: unexpected argument '{ $arg }' found
clap-error-unexpected-argument-simple = unexpected argument
clap-error-similar-argument = { $tip_word }: a similar argument exists: '{ $suggestion }'
clap-error-pass-as-value = { $tip_word }: to pass '{ $arg }' as a value, use '{ $tip_command }'
clap-error-invalid-value = { $error_word }: invalid value '{ $value }' for '{ $option }'
clap-error-value-required = { $error_word }: a value is required for '{ $option }' but none was supplied
clap-error-missing-required-arguments = { $error_word }: the following required arguments were not provided:
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
error-no-such-process = No such process
error-invalid-argument = Invalid argument
error-is-a-directory = { $file }: Is a directory

# Common actions
action-copying = copying
action-moving = moving
action-removing = removing
action-creating = creating
action-reading = reading
action-writing = writing

# SELinux error messages
selinux-error-not-enabled = SELinux is not enabled on this system
selinux-error-file-open-failure = failed to open the file: { $error }
selinux-error-context-retrieval-failure = failed to retrieve the security context: { $error }
selinux-error-context-set-failure = failed to set default file creation context to '{ $context }': { $error }
selinux-error-context-conversion-failure = failed to set default file creation context to '{ $context }': { $error }
selinux-error-operation-not-supported = operation not supported

# SMACK error messages
smack-error-not-enabled = SMACK is not enabled on this system
smack-error-label-retrieval-failure = failed to get security context: { $error }
smack-error-label-set-failure = failed to set default file creation context to '{ $context }': { $error }
smack-error-no-label-set = no security context set

# Safe traversal error messages
safe-traversal-error-path-contains-null = path contains null byte
safe-traversal-error-open-failed = failed to open { $path }: { $source }
safe-traversal-error-stat-failed = failed to stat { $path }: { $source }
safe-traversal-error-read-dir-failed = failed to read directory { $path }: { $source }
safe-traversal-error-unlink-failed = failed to unlink { $path }: { $source }
safe-traversal-error-invalid-fd = invalid file descriptor
safe-traversal-current-directory = <current directory>
safe-traversal-directory = <directory>

# checksum-related messages
checksum-no-properly-formatted = { $checksum_file }: no properly formatted checksum lines found
checksum-no-file-verified = { $checksum_file }: no file was verified
checksum-error-failed-to-read-input = failed to read input
checksum-bad-format = { $count ->
    [1] { $count } line is improperly formatted
   *[other] { $count } lines are improperly formatted
}
checksum-failed-cksum = { $count ->
    [1] { $count } computed checksum did NOT match
   *[other] { $count } computed checksums did NOT match
}
checksum-failed-open-file = { $count ->
    [1] { $count } listed file could not be read
   *[other] { $count } listed files could not be read
}
checksum-error-algo-bad-format = { $file }: { $line }: improperly formatted { $algo } checksum line

# uudoc tldr examples messages
uudoc-tldr-attribution = The examples are provided by the [tldr-pages project](https://tldr.sh) under the [CC BY 4.0 License](https://github.com/tldr-pages/tldr/blob/main/LICENSE.md).
uudoc-tldr-disclaimer = Please note that, as uutils is a work in progress, some examples might fail.

# Symbolic mode parsing messages
mode-error-unexpected-end = unexpected end of mode
mode-error-invalid-operator = invalid operator (expected +, -, or =, but found { $operator })

# Diagnostic labels: what the caret points at in a mode
mode-diag-label-missing-operator = this clause says who, but not what to change
mode-diag-label-invalid-number = not an octal mode
mode-diag-help-syntax = a mode is either octal, as in 644, or clauses such as u+rwx,go-w
# Format string parsing messages (printf, seq, env, ...)
format-error-invalid-spec = %{ $spec }: invalid conversion specification
format-error-too-many-specs = format '{ $format }' has too many % directives
format-error-no-spec = format '{ $format }' has no % directive
format-error-ends-with-percent = format { $format } ends in %
format-error-invalid-precision = invalid precision: '{ $precision }'
format-error-wrong-spec-type = wrong % directive type was given
format-error-write = write error: { $error }
format-error-no-more-arguments = no more arguments
format-error-invalid-argument = invalid argument
format-error-missing-hex = missing hexadecimal number in escape
format-error-invalid-universal-character = invalid universal character name \{ $escape }{ $digits }

# The word ariadne heads the advice line of a caret report with
diagnostics-help-label = Help

# Diagnostic label shared by the utilities whose arguments are an expression
diagnostics-label-expression-complete = the expression was already complete here

# Checksum errors (cksum, md5sum, sha*sum, b2sum)
checksum-error-raw-multiple-files = the --raw option is not supported with multiple files
checksum-error-check-only-flag = the --{ $flag } option is meaningful only when verifying checksums
checksum-error-length-required = --length required for { $algorithm }
checksum-error-invalid-length = invalid length: { $length }
checksum-error-length-too-big-for-blake = maximum digest length for { $algorithm } is 512 bits
checksum-error-length-not-multiple-of-8 = length is not a multiple of 8
checksum-error-invalid-length-for-sha = digest length for { $algorithm } must be 224, 256, 384, or 512
checksum-error-length-required-for-sha = --algorithm={ $algorithm } requires specifying --length 224, 256, 384, or 512
checksum-error-length-only-for-blake2b-sha2-sha3 = --length is only supported with --algorithm blake2b, sha2, or sha3
checksum-error-binary-text-conflict = the --binary and --text options are meaningless when verifying checksums
checksum-error-text-without-untagged = --text mode is only supported with --untagged
checksum-error-tag-check = the --tag option is meaningless when verifying checksums
checksum-error-text-after-tag = --tag does not support --text mode
checksum-error-algorithm-not-supported-with-check = --check is not supported with --algorithm={"{"}bsd,sysv,crc,crc32b{"}"}
checksum-error-combine-multiple-algorithms = You cannot combine multiple hash algorithms!
checksum-error-need-algorithm-to-hash = Needs an algorithm to hash with.
    Use --help for more information.
checksum-error-unknown-algorithm = unknown algorithm: { $algorithm }: clap should have prevented this case

# Diagnostic labels: what the caret points at in a list of ranges. What a zero
# bound got wrong depends on what the range counts, so each utility says that
# in its own words.
range-diag-label-too-large = this number is too large
range-diag-label-inverted = this range ends before it starts
