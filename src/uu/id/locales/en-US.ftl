id-about = Print user and group information for each specified USER,
  or (when USER omitted) for the current user.
id-usage = id [OPTION]... [USER]...
id-after-help = The id utility displays the user and group names and numeric IDs, of the
  calling process, to the standard output. If the real and effective IDs are
  different, both are displayed, otherwise only the real ID is displayed.

  If a user (login name or user ID) is specified, the user and group IDs of
  that user are displayed. In this case, the real and effective IDs are
  assumed to be the same.

# Context help text
id-context-help-disabled = print only the security context of the process (not enabled)
id-context-help-enabled = print only the security context of the process

# Error messages
id-error-names-real-ids-require-flags = printing only names or real IDs requires -u, -g, or -G
id-error-zero-not-permitted-default = option --zero not permitted in default format
id-error-cannot-print-context-with-user = cannot print security context when user specified
id-error-cannot-get-context = can't get process context
id-error-context-selinux-only = --context (-Z) works only on an SELinux-enabled kernel
id-error-no-such-user = { $user }: no such user
id-error-cannot-find-group-name = cannot find name for group ID { $gid }
id-error-cannot-find-user-name = cannot find name for user ID { $uid }
id-error-audit-retrieve = couldn't retrieve information

# Help text for command-line arguments
id-help-ignore = ignore, for compatibility with other versions
id-help-audit = Display the process audit user ID and other process audit properties,
  which requires privilege (not available on Linux).
id-help-user = Display only the effective user ID as a number.
id-help-group = Display only the effective group ID as a number
id-help-groups = Display only the different group IDs as white-space separated numbers,
  in no particular order.
id-help-human-readable = Make the output human-readable. Each display is on a separate line.
id-help-name = Display the name of the user or group ID for the -G, -g and -u options
  instead of the number.
  If any of the ID numbers cannot be mapped into
  names, the number will be displayed as usual.
id-help-password = Display the id as a password file entry.
id-help-real = Display the real ID for the -G, -g and -u options instead of
  the effective ID.
id-help-zero = delimit entries with NUL characters, not whitespace;
  not permitted in default format

# Output labels
id-output-uid = uid
id-output-groups = groups
id-output-login = login
id-output-euid = euid
id-output-rgid = rgid
id-output-context = context
