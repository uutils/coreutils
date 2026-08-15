# Strings only an error path ever asks for: the messages of the shared
# parsers and the labels a caret carries. They live apart from the common
# uucore strings because every utility parses those at startup, and almost
# no run ever needs one of these.

checksum-error-failed-to-read-input = failed to read input
checksum-error-algo-bad-format = { $file }: { $line }: improperly formatted { $algo } checksum line
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

# Diagnostic labels: what the caret points at in a SIZE
size-diag-label-invalid-suffix = not a known unit
size-diag-label-too-big = this number is too large to use
size-diag-help-syntax = a size is a number and an optional unit: K, M, G and so on for 1024, KB, MB, GB for 1000

# Diagnostic labels: what the caret points at in a list of ranges. What a zero
# bound got wrong depends on what the range counts, so each utility says that
# in its own words.
range-diag-label-too-large = this number is too large
range-diag-label-inverted = this range ends before it starts
