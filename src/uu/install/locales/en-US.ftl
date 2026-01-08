install-about = Copy SOURCE to DEST or multiple SOURCE(s) to the existing
  DIRECTORY, while setting permission modes and owner/group
install-usage = install [OPTION]... [FILE]...

# Help messages
install-help-ignored = ignored
install-help-compare = compare each pair of source and destination files, and in some cases, do not modify the destination at all
install-help-directory = treat all arguments as directory names. create all components of the specified directories
install-help-create-leading = create all leading components of DEST except the last, then copy SOURCE to DEST
install-help-group = set group ownership, instead of process's current group
install-help-mode = set permission mode (as in chmod), instead of rwxr-xr-x
install-help-owner = set ownership (super-user only)
install-help-preserve-timestamps = apply access/modification times of SOURCE files to corresponding destination files
install-help-strip = strip symbol tables (no action Windows)
install-help-strip-program = program used to strip binaries (no action Windows)
install-help-target-directory = move all SOURCE arguments into DIRECTORY
install-help-no-target-directory = treat DEST as a normal file
install-help-verbose = explain what is being done
install-help-preserve-context = preserve security context
install-help-context = set security context of files and directories
install-help-default-context = set SELinux security context of destination file and each created directory to default type
install-help-unprivileged = do not require elevated privileges to change the owner, the group, or the file flags of the destination
install-help-debug = display detailed debug messages about the installation process

# Error messages
install-error-dir-needs-arg = { $util_name } with -d requires at least one argument.
install-error-create-dir-failed = cannot create directory { $path }
install-error-chmod-failed = failed to chmod { $path }
install-error-chmod-failed-detailed = { $path }: chmod failed with error { $error }
install-error-chown-failed = failed to chown { $path }: { $error }
install-error-invalid-target = invalid target { $path }: No such file or directory
install-error-target-not-dir = target { $path } is not a directory
install-error-backup-failed = cannot backup { $from } to { $to }
install-error-install-failed = cannot install { $from } to { $to }
install-error-strip-failed = strip program failed: { $error }
install-error-strip-abnormal = strip process terminated abnormally - exit code: { $code }
install-error-metadata-failed = metadata error
install-error-invalid-user = invalid user: { $user }
install-error-invalid-group = invalid group: { $group }
install-error-omitting-directory = omitting directory { $path }
install-error-not-a-directory = failed to access { $path }: Not a directory
install-error-override-directory-failed = cannot overwrite directory { $dir } with non-directory { $file }
install-error-same-file = { $file1 } and { $file2 } are the same file
install-error-extra-operand = extra operand { $operand }
{ $usage }
install-error-invalid-mode = Invalid mode string: { $error }
install-error-mutually-exclusive-target = Options --target-directory and --no-target-directory are mutually exclusive
install-error-mutually-exclusive-compare-preserve = Options --compare and --preserve-timestamps are mutually exclusive
install-error-mutually-exclusive-compare-strip = Options --compare and --strip are mutually exclusive
install-error-missing-file-operand = missing file operand
install-error-missing-destination-operand = missing destination file operand after { $path }
install-error-failed-to-remove = Failed to remove existing file { $path }. Error: { $error }

# Warning messages
install-warning-compare-ignored = the --compare (-C) option is ignored when you specify a mode with non-permission bits

# Verbose output
install-verbose-creating-directory = creating directory { $path }
install-verbose-creating-directory-step = install: creating directory { $path }
install-verbose-removed = removed { $path }
install-verbose-copy = { $from } -> { $to }
install-verbose-backup = (backup: { $backup })

# Debug messages
install-debug-will-copy = will copy { $from } to { $to }
install-debug-backing-up = backing up { $to } to { $backup }
install-debug-copying = copying { $from } to { $to }
install-debug-stripping = stripping { $to }
install-debug-changing-mode = changing mode of { $to } to { $mode }
install-debug-preserving-timestamps = preserving timestamps of { $from } to { $to }
install-debug-preserving-context = preserving security context of { $to }
install-debug-setting-context = setting security context of { $to }
install-debug-checking-copy = checking if { $from } needs to be copied to { $to }
install-debug-reason-source-missing = source file does not exist
install-debug-reason-dest-missing = destination file does not exist
install-debug-reason-different-content = contents differ
install-debug-reason-different-owner = owner differs
install-debug-reason-different-group = group differs
install-debug-reason-different-mode = mode differs


# Additional debug messages for compatibility with tests
install-debug-cannot-stat-source = cannot stat source file '{ $path }'; will copy
install-debug-cannot-stat-dest = cannot stat destination file '{ $path }'; will copy
install-debug-dest-is-symlink = destination '{ $path }' is a symbolic link; will copy
install-debug-special-mode-bits = special mode bits present; will copy
install-debug-mode-differs = destination mode differs; will copy
install-debug-not-regular-file = source or destination is not a regular file; will copy
install-debug-sizes-differ = sizes differ between { $from } and { $to }; will copy
install-debug-selinux-contexts-differ = SELinux contexts differ between { $from } and { $to }; will copy
install-debug-owner-differs = destination owner differs; will copy
install-debug-group-differs = destination group differs; will copy
install-debug-ownership-needs-update = destination ownership needs update; will copy
install-debug-contents-differ = contents differ between { $from } and { $to }; will copy
install-debug-preserving-selinux-context = preserving security context of { $to }
install-debug-setting-selinux-context = setting security context of { $to }
