du-about = Estimate file space usage
du-usage = du [OPTION]... [FILE]...
  du [OPTION]... --files0-from=F
du-after-help = Display values are in units of the first available SIZE from --block-size,
  and the DU_BLOCK_SIZE, BLOCK_SIZE and BLOCKSIZE environment variables.
  Otherwise, units default to 1024 bytes (or 512 if POSIXLY_CORRECT is set).

  SIZE is an integer and optional unit (example: 10M is 10*1024*1024).
  Units are K, M, G, T, P, E, Z, Y (powers of 1024) or KB, MB,... (powers
  of 1000). Units can be decimal, hexadecimal, octal, binary.

  PATTERN allows some advanced exclusions. For example, the following syntaxes
  are supported:
  ? will match only one character
  { "*" } will match zero or more characters
  {"{"}a,b{"}"} will match a or b

# Help messages
du-help-print-help = Print help information.
du-help-all = write counts for all files, not just directories
du-help-apparent-size = print apparent sizes, rather than disk usage although the apparent size is usually smaller, it may be larger due to holes in ('sparse') files, internal fragmentation, indirect blocks, and the like
du-help-block-size = scale sizes by SIZE before printing them. E.g., '-BM' prints sizes in units of 1,048,576 bytes. See SIZE format below.
du-help-bytes = equivalent to '--apparent-size --block-size=1'
du-help-total = produce a grand total
du-help-max-depth = print the total for a directory (or file, with --all) only if it is N or fewer levels below the command line argument;  --max-depth=0 is the same as --summarize
du-help-human-readable = print sizes in human readable format (e.g., 1K 234M 2G)
du-help-inodes = list inode usage information instead of block usage like --block-size=1K
du-help-block-size-1k = like --block-size=1K
du-help-count-links = count sizes many times if hard linked
du-help-dereference = follow all symbolic links
du-help-dereference-args = follow only symlinks that are listed on the command line
du-help-no-dereference = don't follow any symbolic links (this is the default)
du-help-block-size-1m = like --block-size=1M
du-help-null = end each output line with 0 byte rather than newline
du-help-separate-dirs = do not include size of subdirectories
du-help-summarize = display only a total for each argument
du-help-si = like -h, but use powers of 1000 not 1024
du-help-one-file-system = skip directories on different file systems
du-help-threshold = exclude entries smaller than SIZE if positive, or entries greater than SIZE if negative
du-help-verbose = verbose mode (option not present in GNU/Coreutils)
du-help-exclude = exclude files that match PATTERN
du-help-exclude-from = exclude files that match any pattern in FILE
du-help-files0-from = summarize device usage of the NUL-terminated file names specified in file F; if F is -, then read names from standard input
du-help-time = show time of the last modification of any file in the directory, or any of its subdirectories. If WORD is given, show time as WORD instead of modification time: atime, access, use, ctime, status, birth or creation
du-help-time-style = show times using style STYLE: full-iso, long-iso, iso, +FORMAT FORMAT is interpreted like 'date'

# Error messages
du-error-invalid-max-depth = invalid maximum depth { $depth }
du-error-summarize-depth-conflict = summarizing conflicts with --max-depth={ $depth }
du-error-invalid-time-style = invalid argument { $style } for 'time style'
  Valid arguments are:
    - 'full-iso'
    - 'long-iso'
    - 'iso'
    - +FORMAT (e.g., +%H:%M) for a 'date'-style format
  Try '{ $help }' for more information.
du-error-invalid-time-arg = 'birth' and 'creation' arguments for --time are not supported on this platform.
du-error-invalid-glob = Invalid exclude syntax: { $error }
du-error-cannot-read-directory = cannot read directory { $path }
du-error-cannot-access = cannot access { $path }
du-error-read-error-is-directory = { $file }: read error: Is a directory
du-error-cannot-open-for-reading = cannot open { $file } for reading: No such file or directory
du-error-invalid-zero-length-file-name = { $file }:{ $line }: invalid zero-length file name
du-error-extra-operand-with-files0-from = extra operand { $file }
  file operands cannot be combined with --files0-from
du-error-invalid-block-size-argument = invalid --{ $option } argument { $value }
du-error-cannot-access-no-such-file = cannot access { $path }: No such file or directory
du-error-printing-thread-panicked = Printing thread panicked.
du-error-invalid-suffix = invalid suffix in --{ $option } argument { $value }
du-error-invalid-argument = invalid --{ $option } argument { $value }
du-error-argument-too-large = --{ $option } argument { $value } too large
du-error-hyphen-file-name-not-allowed = when reading file names from standard input, no file name of '-' allowed

# Verbose/status messages
du-verbose-ignored = { $path } ignored
du-verbose-adding-to-exclude-list = adding { $pattern } to the exclude list
du-total = total
du-warning-apparent-size-ineffective-with-inodes = options --apparent-size and -b are ineffective with --inodes
