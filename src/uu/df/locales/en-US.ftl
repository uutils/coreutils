df-about = Show information about the file system on which each FILE resides,
  or all file systems by default.
df-usage = df [OPTION]... [FILE]...
df-after-help = Display values are in units of the first available SIZE from --block-size,
  and the DF_BLOCK_SIZE, BLOCK_SIZE and BLOCKSIZE environment variables.
  Otherwise, units default to 1024 bytes (or 512 if POSIXLY_CORRECT is set).

  SIZE is an integer and optional unit (example: 10M is 10*1024*1024).
  Units are K, M, G, T, P, E, Z, Y (powers of 1024) or KB, MB,... (powers
  of 1000). Units can be decimal, hexadecimal, octal, binary.

# Help messages
df-help-print-help = Print help information.
df-help-all = include dummy file systems
df-help-block-size = scale sizes by SIZE before printing them; e.g. '-BM' prints sizes in units of 1,048,576 bytes
df-help-total = produce a grand total
df-help-human-readable = print sizes in human readable format (e.g., 1K 234M 2G)
df-help-si = likewise, but use powers of 1000 not 1024
df-help-inodes = list inode information instead of block usage
df-help-kilo = like --block-size=1K
df-help-local = limit listing to local file systems
df-help-no-sync = do not invoke sync before getting usage info (default)
df-help-output = use output format defined by FIELD_LIST, or print all fields if FIELD_LIST is omitted.
df-help-portability = use the POSIX output format
df-help-sync = invoke sync before getting usage info (non-windows only)
df-help-type = limit listing to file systems of type TYPE
df-help-print-type = print file system type
df-help-exclude-type = limit listing to file systems not of type TYPE

# Error messages
df-error-block-size-too-large = --block-size argument '{ $size }' too large
df-error-invalid-block-size = invalid --block-size argument { $size }
df-error-invalid-suffix = invalid suffix in --block-size argument { $size }
df-error-field-used-more-than-once = option --output: field { $field } used more than once
df-error-filesystem-type-both-selected-and-excluded = file system type { $type } both selected and excluded
df-error-no-such-file-or-directory = { $path }: No such file or directory
df-error-no-file-systems-processed = no file systems processed
df-error-cannot-access-over-mounted = cannot access { $path }: over-mounted by another device
df-error-cannot-read-table-of-mounted-filesystems = cannot read table of mounted file systems
df-error-inodes-not-supported-windows = { $program }: doesn't support -i option

# Headers
df-header-filesystem = Filesystem
df-header-size = Size
df-header-used = Used
df-header-avail = Avail
df-header-available = Available
df-header-use-percent = Use%
df-header-capacity = Capacity
df-header-mounted-on = Mounted on
df-header-inodes = Inodes
df-header-iused = IUsed
df-header-iavail = IFree
df-header-iuse-percent = IUse%
df-header-file = File
df-header-type = Type

# Other
df-total = total
df-blocks-suffix = -blocks
