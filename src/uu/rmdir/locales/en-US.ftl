rmdir-about = Remove the DIRECTORY(ies), if they are empty.
rmdir-usage = rmdir [OPTION]... DIRECTORY...

# Help messages
rmdir-help-ignore-fail-non-empty = ignore each failure that is solely because a directory is non-empty
rmdir-help-parents = remove DIRECTORY and its ancestors; e.g., 'rmdir -p a/b/c' is similar to rmdir a/b/c a/b a
rmdir-help-verbose = output a diagnostic for every directory processed

# Error messages
rmdir-error-symbolic-link-not-followed = failed to remove { $path }: Symbolic link not followed
rmdir-error-failed-to-remove = failed to remove { $path }: { $err }

# Verbose output
rmdir-verbose-removing-directory = { $util_name }: removing directory, { $path }
