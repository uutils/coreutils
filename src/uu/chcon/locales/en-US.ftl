chcon-about = Change the SELinux security context of each FILE to CONTEXT.
  With --reference, change the security context of each FILE to that of RFILE.
chcon-usage = chcon [OPTION]... CONTEXT FILE...
  chcon [OPTION]... [-u USER] [-r ROLE] [-l RANGE] [-t TYPE] FILE...
  chcon [OPTION]... --reference=RFILE FILE...

# Help messages
chcon-help-help = Print help information.
chcon-help-dereference = Affect the referent of each symbolic link (this is the default), rather than the symbolic link itself.
chcon-help-no-dereference = Affect symbolic links instead of any referenced file.
chcon-help-preserve-root = Fail to operate recursively on '/'.
chcon-help-no-preserve-root = Do not treat '/' specially (the default).
chcon-help-reference = Use security context of RFILE, rather than specifying a CONTEXT value.
chcon-help-user = Set user USER in the target security context.
chcon-help-role = Set role ROLE in the target security context.
chcon-help-type = Set type TYPE in the target security context.
chcon-help-range = Set range RANGE in the target security context.
chcon-help-recursive = Operate on files and directories recursively.
chcon-help-follow-arg-dir-symlink = If a command line argument is a symbolic link to a directory, traverse it. Only valid when -R is specified.
chcon-help-follow-dir-symlinks = Traverse every symbolic link to a directory encountered. Only valid when -R is specified.
chcon-help-no-follow-symlinks = Do not traverse any symbolic links (default). Only valid when -R is specified.
chcon-help-verbose = Output a diagnostic for every file processed.

# Error messages - basic validation
chcon-error-no-context-specified = No context is specified
chcon-error-no-files-specified = No files are specified
chcon-error-data-out-of-range = Data is out of range
chcon-error-operation-failed = { $operation } failed
chcon-error-operation-failed-on = { $operation } failed on { $operand }

# Error messages - argument validation
chcon-error-invalid-context = Invalid security context '{ $context }'.
chcon-error-recursive-no-dereference-require-p = '--recursive' with '--no-dereference' require '-P'
chcon-error-recursive-dereference-require-h-or-l = '--recursive' with '--dereference' require either '-H' or '-L'

# Operation strings for error context
chcon-op-getting-security-context = Getting security context
chcon-op-file-name-validation = File name validation
chcon-op-getting-meta-data = Getting meta data
chcon-op-modifying-root-path = Modifying root path
chcon-op-accessing = Accessing
chcon-op-reading-directory = Reading directory
chcon-op-reading-cyclic-directory = Reading cyclic directory
chcon-op-applying-partial-context = Applying partial security context to unlabeled file
chcon-op-creating-security-context = Creating security context
chcon-op-setting-security-context-user = Setting security context user
chcon-op-setting-security-context = Setting security context

# Verbose output
chcon-verbose-changing-context = { $util_name }: changing security context of { $file }

# Warning messages
chcon-warning-dangerous-recursive-root = It is dangerous to operate recursively on '/'. Use --{ $option } to override this failsafe.
chcon-warning-dangerous-recursive-dir = It is dangerous to operate recursively on { $dir } (same as '/'). Use --{ $option } to override this failsafe.
chcon-warning-circular-directory = Circular directory structure.
  This almost certainly means that you have a corrupted file system.
  NOTIFY YOUR SYSTEM MANAGER.
  The following directory is part of the cycle { $file }.
