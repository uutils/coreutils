stat-about = Display file or file system status.
stat-usage = stat [OPTION]... FILE...
stat-after-help = Valid format sequences for files (without `--file-system`):

  -`%a`: access rights in octal (note '#' and '0' printf flags)
  -`%A`: access rights in human readable form
  -`%b`: number of blocks allocated (see %B)
  -`%B`: the size in bytes of each block reported by %b
  -`%C`: SELinux security context string
  -`%d`: device number in decimal
  -`%D`: device number in hex
  -`%f`: raw mode in hex
  -`%F`: file type
  -`%g`: group ID of owner
  -`%G`: group name of owner
  -`%h`: number of hard links
  -`%i`: inode number
  -`%m`: mount point
  -`%n`: file name
  -`%N`: quoted file name with dereference (follow) if symbolic link
  -`%o`: optimal I/O transfer size hint
  -`%s`: total size, in bytes
  -`%t`: major device type in hex, for character/block device special files
  -`%T`: minor device type in hex, for character/block device special files
  -`%u`: user ID of owner
  -`%U`: user name of owner
  -`%w`: time of file birth, human-readable; - if unknown
  -`%W`: time of file birth, seconds since Epoch; 0 if unknown
  -`%x`: time of last access, human-readable
  -`%X`: time of last access, seconds since Epoch
  -`%y`: time of last data modification, human-readable

  -`%Y`: time of last data modification, seconds since Epoch
  -`%z`: time of last status change, human-readable
  -`%Z`: time of last status change, seconds since Epoch

  Valid format sequences for file systems:

  -`%a`: free blocks available to non-superuser
  -`%b`: total data blocks in file system
  -`%c`: total file nodes in file system
  -`%d`: free file nodes in file system
  -`%f`: free blocks in file system
  -`%i`: file system ID in hex
  -`%l`: maximum length of filenames
  -`%n`: file name
  -`%s`: block size (for faster transfers)
  -`%S`: fundamental block size (for block counts)
  -`%t`: file system type in hex
  -`%T`: file system type in human readable form

  NOTE: your shell may have its own version of stat, which usually supersedes
  the version described here.  Please refer to your shell's documentation
  for details about the options it supports.

## Error messages

stat-error-invalid-quoting-style = Invalid quoting style: {$style}
stat-error-missing-operand = missing operand
  Try 'stat --help' for more information.
stat-error-invalid-directive = {$directive}: invalid directive
stat-error-cannot-read-filesystem = cannot read table of mounted file systems: {$error}
stat-error-stdin-filesystem-mode = using '-' to denote standard input does not work in file system mode
stat-error-cannot-read-filesystem-info = cannot read file system information for {$file}: {$error}
stat-error-cannot-stat = cannot stat {$file}: {$error}

## Warning messages

stat-warning-backslash-end-format = backslash at end of format
stat-warning-unrecognized-escape-x = unrecognized escape '\x'
stat-warning-incomplete-hex-escape = incomplete hex escape '\x'
stat-warning-unrecognized-escape = unrecognized escape '\{$escape}'

## Help messages

stat-help-dereference = follow links
stat-help-file-system = display file system status instead of file status
stat-help-terse = print the information in terse form
stat-help-format = use the specified FORMAT instead of the default;
 output a newline after each use of FORMAT
stat-help-printf = like --format, but interpret backslash escapes,
  and do not output a mandatory trailing newline;
  if you want a newline, include \n in FORMAT

## Word translations

stat-word-file = File
stat-word-id = ID
stat-word-namelen = Namelen
stat-word-type = Type
stat-word-block = Block
stat-word-size = size
stat-word-fundamental = Fundamental
stat-word-block-size = block size
stat-word-blocks = Blocks
stat-word-total = Total
stat-word-free = Free
stat-word-available = Available
stat-word-inodes = Inodes
stat-word-device = Device
stat-word-inode = Inode
stat-word-links = Links
stat-word-io = IO
stat-word-access = Access
stat-word-uid = Uid
stat-word-gid = Gid
stat-word-modify = Modify
stat-word-change = Change
stat-word-birth = Birth

## SELinux context messages

stat-selinux-failed-get-context = failed to get security context
stat-selinux-unsupported-system = unsupported on this system
stat-selinux-unsupported-os = unsupported for this operating system
