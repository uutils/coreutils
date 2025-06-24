split-about = Create output files containing consecutive or interleaved sections of input
split-usage = split [OPTION]... [INPUT [PREFIX]]
split-after-help = Output fixed-size pieces of INPUT to PREFIXaa, PREFIXab, ...; default size is 1000, and default PREFIX is 'x'. With no INPUT, or when INPUT is -, read standard input.

  The SIZE argument is an integer and optional unit (example: 10K is 10*1024).
  Units are K,M,G,T,P,E,Z,Y,R,Q (powers of 1024) or KB,MB,... (powers of 1000).
  Binary prefixes can be used, too: KiB=K, MiB=M, and so on.

  CHUNKS may be:

  - N split into N files based on size of input
  - K/N output Kth of N to stdout
  - l/N split into N files without splitting lines/records
  - l/K/N output Kth of N to stdout without splitting lines/records
  - r/N like 'l' but use round robin distribution
  - r/K/N likewise but only output Kth of N to stdout

# Error messages
split-error-suffix-not-parsable = invalid suffix length: { $value }
split-error-suffix-contains-separator = invalid suffix { $value }, contains directory separator
split-error-suffix-too-small = the suffix length needs to be at least { $length }
split-error-multi-character-separator = multi-character separator { $separator }
split-error-multiple-separator-characters = multiple separator characters specified
split-error-filter-with-kth-chunk = --filter does not process a chunk extracted to stdout
split-error-invalid-io-block-size = invalid IO block size: { $size }
split-error-not-supported = --filter is currently not supported in this platform
split-error-invalid-number-of-chunks = invalid number of chunks: { $chunks }
split-error-invalid-chunk-number = invalid chunk number: { $chunk }
split-error-invalid-number-of-lines = invalid number of lines: { $error }
split-error-invalid-number-of-bytes = invalid number of bytes: { $error }
split-error-cannot-split-more-than-one-way = cannot split in more than one way
split-error-overflow = Overflow
split-error-output-file-suffixes-exhausted = output file suffixes exhausted
split-error-numerical-suffix-start-too-large = numerical suffix start value is too large for the suffix length
split-error-cannot-open-for-reading = cannot open { $file } for reading
split-error-would-overwrite-input = { $file } would overwrite input; aborting
split-error-cannot-determine-input-size = { $input }: cannot determine input size
split-error-cannot-determine-file-size = { $input }: cannot determine file size
split-error-cannot-read-from-input = { $input }: cannot read from input : { $error }
split-error-input-output-error = input/output error
split-error-unable-to-open-file = unable to open { $file }; aborting
split-error-unable-to-reopen-file = unable to re-open { $file }; aborting
split-error-file-descriptor-limit = at file descriptor limit, but no file descriptor left to close. Closed { $count } writers before.
split-error-shell-process-returned = Shell process returned { $code }
split-error-shell-process-terminated = Shell process terminated by signal

# Help messages for command-line options
split-help-bytes = put SIZE bytes per output file
split-help-line-bytes = put at most SIZE bytes of lines per output file
split-help-lines = put NUMBER lines/records per output file
split-help-number = generate CHUNKS output files; see explanation below
split-help-additional-suffix = additional SUFFIX to append to output file names
split-help-filter = write to shell COMMAND; file name is $FILE (Currently not implemented for Windows)
split-help-elide-empty-files = do not generate empty output files with '-n'
split-help-numeric-suffixes-short = use numeric suffixes starting at 0, not alphabetic
split-help-numeric-suffixes = same as -d, but allow setting the start value
split-help-hex-suffixes-short = use hex suffixes starting at 0, not alphabetic
split-help-hex-suffixes = same as -x, but allow setting the start value
split-help-suffix-length = generate suffixes of length N (default 2)
split-help-verbose = print a diagnostic just before each output file is opened
split-help-separator = use SEP instead of newline as the record separator; '\\0' (zero) specifies the NUL character
