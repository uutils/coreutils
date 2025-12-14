touch-about = Update the access and modification times of each FILE to the current time.
touch-usage = touch [OPTION]... [FILE]...

# Help messages
touch-help-help = Print help information.
touch-help-access = change only the access time
touch-help-timestamp = use [[CC]YY]MMDDhhmm[.ss] instead of the current time
touch-help-date = parse argument and use it instead of current time
touch-help-modification = change only the modification time
touch-help-no-create = do not create any files
touch-help-no-deref = affect each symbolic link instead of any referenced file (only for systems that can change the timestamps of a symlink)
touch-help-reference = use this file's times instead of the current time
touch-help-time = change only the specified time: "access", "atime", or "use" are equivalent to -a; "modify" or "mtime" are equivalent to -m

# Error messages
touch-error-missing-file-operand = missing file operand
  Try '{ $help_command } --help' for more information.
touch-error-setting-times-of = setting times of { $filename }
touch-error-setting-times-no-such-file = setting times of { $filename }: No such file or directory
touch-error-cannot-touch = cannot touch { $filename }
touch-error-no-such-file-or-directory = No such file or directory
touch-error-failed-to-get-attributes = failed to get attributes of { $path }
touch-error-setting-times-of-path = setting times of { $path }
touch-error-invalid-date-ts-format = invalid date ts format { $date }
touch-error-invalid-date-format = invalid date format { $date }
touch-error-unable-to-parse-date = Unable to parse date: { $date }
touch-error-windows-stdout-path-failed = GetFinalPathNameByHandleW failed with code { $code }
touch-error-invalid-filetime = Source has invalid access or modification time: { $time }
touch-error-reference-file-inaccessible = failed to get attributes of { $path }: { $error }
