mv-about = Move SOURCE to DEST, or multiple SOURCE(s) to DIRECTORY.
mv-usage = mv [OPTION]... [-T] SOURCE DEST
  mv [OPTION]... SOURCE... DIRECTORY
  mv [OPTION]... -t DIRECTORY SOURCE...
mv-after-help = When specifying more than one of -i, -f, -n, only the final one will take effect.

  Do not move a non-directory that has an existing destination with the same or newer modification timestamp;
  instead, silently skip the file without failing. If the move is across file system boundaries, the comparison is
  to the source timestamp truncated to the resolutions of the destination file system and of the system calls used
  to update timestamps; this avoids duplicate work if several mv -u commands are executed with the same source
  and destination. This option is ignored if the -n or --no-clobber option is also specified. which gives more control
  over which existing files in the destination are replaced, and its value can be one of the following:

  - all This is the default operation when an --update option is not specified, and results in all existing files in the destination being replaced.
  - none This is similar to the --no-clobber option, in that no files in the destination are replaced, but also skipping a file does not induce a failure.
  - older This is the default operation when --update is specified, and results in files being replaced if they’re older than the corresponding source file.

# Error messages
mv-error-insufficient-arguments = The argument '<{$arg_files}>...' requires at least 2 values, but only 1 was provided
mv-error-no-such-file = cannot stat {$path}: No such file or directory
mv-error-cannot-stat-not-directory = cannot stat {$path}: Not a directory
mv-error-same-file = {$source} and {$target} are the same file
mv-error-self-target-subdirectory = cannot move {$source} to a subdirectory of itself, {$target}
mv-error-directory-to-non-directory = cannot overwrite directory {$path} with non-directory
mv-error-non-directory-to-directory = cannot overwrite non-directory {$target} with directory {$source}
mv-error-not-directory = target {$path}: Not a directory
mv-error-target-not-directory = target directory {$path}: Not a directory
mv-error-failed-access-not-directory = failed to access {$path}: Not a directory
mv-error-backup-with-no-clobber = cannot combine --backup with -n/--no-clobber or --update=none-fail
mv-error-extra-operand = mv: extra operand {$operand}
mv-error-backup-might-destroy-source = backing up {$target} might destroy source;  {$source} not moved
mv-error-will-not-overwrite-just-created = will not overwrite just-created {$target} with {$source}
mv-error-not-replacing = not replacing {$target}
mv-error-cannot-move = cannot move {$source} to {$target}
mv-error-directory-not-empty = Directory not empty
mv-error-dangling-symlink = can't determine symlink type, since it is dangling
mv-error-no-symlink-support = your operating system does not support symlinks
mv-error-permission-denied = Permission denied
mv-error-inter-device-move-failed = inter-device move failed: {$from} to {$to}; unable to remove target: {$err}

# Help messages
mv-help-force = do not prompt before overwriting
mv-help-interactive = prompt before override
mv-help-no-clobber = do not overwrite an existing file
mv-help-strip-trailing-slashes = remove any trailing slashes from each SOURCE argument
mv-help-target-directory = move all SOURCE arguments into DIRECTORY
mv-help-no-target-directory = treat DEST as a normal file
mv-help-verbose = explain what is being done
mv-help-progress = Display a progress bar.
  Note: this feature is not supported by GNU coreutils.
mv-help-debug = explain how a file is copied. Implies -v
mv-help-selinux = set SELinux security context of destination file to default type
mv-help-context = like -Z, or if CTX is specified then set the SELinux security context to CTX

# Verbose messages
mv-verbose-renamed = renamed {$from} -> {$to}
mv-verbose-renamed-with-backup = renamed {$from} -> {$to} (backup: {$backup})

# Debug messages
mv-debug-skipped = skipped {$target}

# Prompt messages
mv-prompt-overwrite = overwrite {$target}?
mv-prompt-overwrite-mode = replace {$target}, overriding mode {$mode_info}?

# Progress messages
mv-progress-moving = moving
