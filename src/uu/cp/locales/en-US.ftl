cp-about = Copy SOURCE to DEST, or multiple SOURCE(s) to DIRECTORY.
cp-usage = cp [OPTION]... [-T] SOURCE DEST
  cp [OPTION]... SOURCE... DIRECTORY
  cp [OPTION]... -t DIRECTORY SOURCE...
cp-after-help = Do not copy a non-directory that has an existing destination with the same or newer modification timestamp;
  instead, silently skip the file without failing. If timestamps are being preserved, the comparison is to the
  source timestamp truncated to the resolutions of the destination file system and of the system calls used to
  update timestamps; this avoids duplicate work if several cp -pu commands are executed with the same source
  and destination. This option is ignored if the -n or --no-clobber option is also specified. Also, if
  --preserve=links is also specified (like with cp -au for example), that will take precedence; consequently,
  depending on the order that files are processed from the source, newer files in the destination may be replaced,
  to mirror hard links in the source. which gives more control over which existing files in the destination are
  replaced, and its value can be one of the following:

  - all This is the default operation when an --update option is not specified, and results in all existing files in the destination being replaced.
  - none This is similar to the --no-clobber option, in that no files in the destination are replaced, but also skipping a file does not induce a failure.
  - older This is the default operation when --update is specified, and results in files being replaced if they're older than the corresponding source file.

# Help messages
cp-help-target-directory = copy all SOURCE arguments into target-directory
cp-help-no-target-directory = Treat DEST as a regular file and not a directory
cp-help-interactive = ask before overwriting files
cp-help-link = hard-link files instead of copying
cp-help-no-clobber = don't overwrite a file that already exists
cp-help-recursive = copy directories recursively
cp-help-strip-trailing-slashes = remove any trailing slashes from each SOURCE argument
cp-help-debug = explain how a file is copied. Implies -v
cp-help-verbose = explicitly state what is being done
cp-help-symbolic-link = make symbolic links instead of copying
cp-help-force = if an existing destination file cannot be opened, remove it and try again (this option is ignored when the -n option is also used). Currently not implemented for Windows.
cp-help-remove-destination = remove each existing destination file before attempting to open it (contrast with --force). On Windows, currently only works for writeable files.
cp-help-reflink = control clone/CoW copies. See below
cp-help-attributes-only = Don't copy the file data, just the attributes
cp-help-preserve = Preserve the specified attributes (default: mode, ownership (unix only), timestamps), if possible additional attributes: context, links, xattr, all
cp-help-preserve-default = same as --preserve=mode,ownership(unix only),timestamps
cp-help-no-preserve = don't preserve the specified attributes
cp-help-parents = use full source file name under DIRECTORY
cp-help-no-dereference = never follow symbolic links in SOURCE
cp-help-dereference = always follow symbolic links in SOURCE
cp-help-cli-symbolic-links = follow command-line symbolic links in SOURCE
cp-help-archive = Same as -dR --preserve=all
cp-help-no-dereference-preserve-links = same as --no-dereference --preserve=links
cp-help-one-file-system = stay on this file system
cp-help-sparse = control creation of sparse files. See below
cp-help-selinux = set SELinux security context of destination file to default type
cp-help-context = like -Z, or if CTX is specified then set the SELinux or SMACK security context to CTX
cp-help-progress = Display a progress bar. Note: this feature is not supported by GNU coreutils.
cp-help-copy-contents = NotImplemented: copy contents of special files when recursive

# Error messages
cp-error-missing-file-operand = missing file operand
cp-error-missing-destination-operand = missing destination file operand after { $source }
cp-error-extra-operand = extra operand { $operand }
cp-error-same-file = { $source } and { $dest } are the same file
cp-error-backing-up-destroy-source = backing up { $dest } might destroy source;  { $source } not copied
cp-error-cannot-open-for-reading = cannot open { $source } for reading
cp-error-not-writing-dangling-symlink = not writing through dangling symlink { $dest }
cp-error-failed-to-clone = failed to clone { $source } from { $dest }: { $error }
cp-error-cannot-change-attribute = cannot change attribute { $dest }: Source file is a non regular file
cp-error-cannot-stat = cannot stat { $source }: No such file or directory
cp-error-cannot-create-symlink = cannot create symlink { $dest } to { $source }
cp-error-cannot-create-hard-link = cannot create hard link { $dest } to { $source }
cp-error-omitting-directory = -r not specified; omitting directory { $dir }
cp-error-cannot-copy-directory-into-itself = cannot copy a directory, { $source }, into itself, { $dest }
cp-error-will-not-copy-through-symlink = will not copy { $source } through just-created symlink { $dest }
cp-error-will-not-overwrite-just-created = will not overwrite just-created { $dest } with { $source }
cp-error-target-not-directory = target: { $target } is not a directory
cp-error-cannot-overwrite-directory-with-non-directory = cannot overwrite directory { $dir } with non-directory
cp-error-cannot-overwrite-non-directory-with-directory = cannot overwrite non-directory with directory
cp-error-with-parents-dest-must-be-dir = with --parents, the destination must be a directory
cp-error-not-replacing = not replacing { $file }
cp-error-failed-get-current-dir = failed to get current directory { $error }
cp-error-failed-set-permissions = cannot set permissions { $path }
cp-error-backup-mutually-exclusive = options --backup and --no-clobber are mutually exclusive
cp-error-invalid-argument = invalid argument { $arg } for '{ $option }'
cp-error-option-not-implemented = Option '{ $option }' not yet implemented.
cp-error-not-all-files-copied = Not all files were copied
cp-error-reflink-always-sparse-auto = `--reflink=always` can be used only with --sparse=auto
cp-error-file-exists = { $path }: File exists
cp-error-invalid-backup-argument = --backup is mutually exclusive with -n or --update=none-fail
cp-error-reflink-not-supported = --reflink is only supported on linux and macOS
cp-error-sparse-not-supported = --sparse is only supported on linux
cp-error-not-a-directory = { $path } is not a directory
cp-error-selinux-not-enabled = SELinux was not enabled during the compile time!
cp-error-selinux-set-context = failed to set the security context of { $path }: { $error }
cp-error-selinux-get-context = failed to get security context of { $path }
cp-error-selinux-error = SELinux error: { $error }
cp-error-cannot-create-fifo = cannot create fifo { $path }: File exists
cp-error-invalid-attribute = invalid attribute { $value }
cp-error-failed-to-create-whole-tree = failed to create whole tree
cp-error-failed-to-create-directory = Failed to create directory: { $error }
cp-error-backup-format = cp: { $error }
  Try '{ $exec } --help' for more information.

# Debug enum strings
cp-debug-enum-no = no
cp-debug-enum-yes = yes
cp-debug-enum-avoided = avoided
cp-debug-enum-unsupported = unsupported
cp-debug-enum-unknown = unknown
cp-debug-enum-zeros = zeros
cp-debug-enum-seek-hole = SEEK_HOLE
cp-debug-enum-seek-hole-zeros = SEEK_HOLE + zeros

# Warning messages
cp-warning-source-specified-more-than-once = source { $file_type } { $source } specified more than once

# Verbose and debug messages
cp-verbose-copied = { $source } -> { $dest }
cp-debug-skipped = skipped { $path }
cp-verbose-removed = removed { $path }
cp-verbose-created-directory = { $source } -> { $dest }
cp-debug-copy-offload = copy offload: { $offload }, reflink: { $reflink }, sparse detection: { $sparse }

# Prompts
cp-prompt-overwrite = overwrite { $path }?
cp-prompt-overwrite-with-mode = replace { $path }, overriding mode
