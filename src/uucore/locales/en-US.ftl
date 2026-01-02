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

# checksum argument help messages
checksum-help-algorithm = select the digest type to use. See DIGEST below
checksum-help-untagged = create a reversed style checksum, without digest type
checksum-help-tag-default = create a BSD style checksum (default)
checksum-help-tag = create a BSD style checksum
checksum-help-text = read in text mode (default)
checksum-help-length = digest length in bits; must not exceed the max size and must be a multiple of 8 for blake2b; must be 224, 256, 384, or 512 for sha2 or sha3
checksum-help-check = read checksums from the FILEs and check them
checksum-help-base64 = emit base64-encoded digests, not hexadecimal
checksum-help-raw = emit a raw binary digest, not hexadecimal
checksum-help-zero = end each output line with NUL, not newline, and disable file name escaping

checksum-help-strict = exit non-zero for improperly formatted checksum lines
checksum-help-warn = warn about improperly formatted checksum lines
checksum-help-status = don't output anything, status code shows success
checksum-help-quiet = don't print OK for each successfully verified file
checksum-help-ignore-missing = don't fail or report status for missing files

checksum-help-debug = print CPU hardware capability detection info used by cksum
