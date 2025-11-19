chmod-about = Change the mode of each FILE to MODE.
  With --reference, change the mode of each FILE to that of RFILE.
chmod-usage = chmod [OPTION]... MODE[,MODE]... FILE...
  chmod [OPTION]... OCTAL-MODE FILE...
  chmod [OPTION]... --reference=RFILE FILE...
chmod-after-help = Each MODE is of the form [ugoa]*([-+=]([rwxXst]*|[ugo]))+|[-+=]?[0-7]+.
chmod-error-cannot-stat = cannot stat attributes of {$file}
chmod-error-dangling-symlink = cannot operate on dangling symlink {$file}
chmod-error-no-such-file = cannot access {$file}: No such file or directory
chmod-error-preserve-root = it is dangerous to operate recursively on {$file}
  chmod: use --no-preserve-root to override this failsafe
chmod-error-permission-denied = cannot access {$file}: Permission denied
chmod-error-new-permissions = {$file}: new permissions are {$actual}, not {$expected}
chmod-error-missing-operand = missing operand

# Help messages
chmod-help-print-help = Print help information.
chmod-help-changes = like verbose but report only when a change is made
chmod-help-quiet = suppress most error messages
chmod-help-verbose = output a diagnostic for every file processed
chmod-help-no-preserve-root = do not treat '/' specially (the default)
chmod-help-preserve-root = fail to operate recursively on '/'
chmod-help-recursive = change files and directories recursively
chmod-help-reference = use RFILE's mode instead of MODE values

# Verbose messages
chmod-verbose-failed-dangling = failed to change mode of {$file} from 0000 (---------) to 1500 (r-x-----T)
chmod-verbose-neither-changed = neither symbolic link {$file} nor referent has been changed
chmod-verbose-mode-retained = mode of {$file} retained as {$mode_octal} ({$mode_display})
chmod-verbose-failed-change = failed to change mode of file {$file} from {$old_mode} ({$old_mode_display}) to {$new_mode} ({$new_mode_display})
chmod-verbose-mode-changed = mode of {$file} changed from {$old_mode} ({$old_mode_display}) to {$new_mode} ({$new_mode_display})
