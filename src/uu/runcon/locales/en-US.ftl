runcon-about = Run command with specified security context under SELinux enabled systems.
runcon-usage = runcon CONTEXT COMMAND [ARG...]
  runcon [-c] [-u USER] [-r ROLE] [-t TYPE] [-l RANGE] COMMAND [ARG...]
runcon-after-help = Run COMMAND with completely-specified CONTEXT, or with current or transitioned security context modified by one or more of LEVEL, ROLE, TYPE, and USER.

  If none of --compute, --type, --user, --role or --range is specified, then the first argument is used as the complete context.

  Note that only carefully-chosen contexts are likely to successfully run.

  If neither CONTEXT nor COMMAND is specified, the current security context is printed.

# Help messages
runcon-help-compute = Compute process transition context before modifying.
runcon-help-user = Set user USER in the target security context.
runcon-help-role = Set role ROLE in the target security context.
runcon-help-type = Set type TYPE in the target security context.
runcon-help-range = Set range RANGE in the target security context.

# Error messages
runcon-error-no-command = No command is specified
runcon-error-selinux-not-enabled = runcon may be used only on a SELinux kernel
runcon-error-operation-failed = { $operation } failed
runcon-error-operation-failed-on = { $operation } failed on { $operand }

# Operation names
runcon-operation-getting-current-context = Getting security context of the current process
runcon-operation-creating-context = Creating new context
runcon-operation-checking-context = Checking security context
runcon-operation-setting-context = Setting new security context
runcon-operation-getting-process-class = Getting process security class
runcon-operation-getting-file-context = Getting security context of command file
runcon-operation-computing-transition = Computing result of process transition
runcon-operation-getting-context = Getting security context
runcon-operation-setting-user = Setting security context user
runcon-operation-setting-role = Setting security context role
runcon-operation-setting-type = Setting security context type
runcon-operation-setting-range = Setting security context range
runcon-operation-executing-command = Executing command
