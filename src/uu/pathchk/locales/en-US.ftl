pathchk-about = Check whether file names are valid or portable
pathchk-usage = pathchk [OPTION]... NAME...

# Help messages
pathchk-help-posix = check for most POSIX systems
pathchk-help-posix-special = check for empty names and leading "-"
pathchk-help-portability = check for all POSIX systems (equivalent to -p -P)

# Error messages
pathchk-error-missing-operand = missing operand
pathchk-error-empty-file-name = empty file name
pathchk-error-posix-path-length-exceeded = limit { $limit } exceeded by length { $length } of file name { $path }
pathchk-error-posix-name-length-exceeded = limit { $limit } exceeded by length { $length } of file name component { $component }
pathchk-error-leading-hyphen = leading hyphen in file name component { $component }
pathchk-error-path-length-exceeded = limit { $limit } exceeded by length { $length } of file name { $path }
pathchk-error-name-length-exceeded = limit { $limit } exceeded by length { $length } of file name component { $component }
pathchk-error-empty-path-not-found = pathchk: '': No such file or directory
pathchk-error-nonportable-character = nonportable character '{ $character }' in file name component { $component }
