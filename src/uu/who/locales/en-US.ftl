who-about = Print information about users who are currently logged in.
who-usage = who [OPTION]... [ FILE | ARG1 ARG2 ]
who-about-musl-warning = Note: When built with musl libc, the `who` utility will not display any
    information about logged-in users. This is due to musl's stub implementation
    of `utmpx` functions, which prevents access to the necessary data.

# Long usage help text
who-long-usage = If FILE is not specified, use { $default_file }. /var/log/wtmp as FILE is common.
    If ARG1 ARG2 given, -m presumed: 'am i' or 'mom likes' are usual.

# Help text for command-line arguments
who-help-all = same as -b -d --login -p -r -t -T -u
who-help-boot = time of last system boot
who-help-dead = print dead processes
who-help-heading = print line of column headings
who-help-login = print system login processes
who-help-lookup = attempt to canonicalize hostnames via DNS
who-help-only-hostname-user = only hostname and user associated with stdin
who-help-process = print active processes spawned by init
who-help-count = all login names and number of users logged on
who-help-runlevel = print current runlevel
who-help-runlevel-non-linux = print current runlevel (This is meaningless on non Linux)
who-help-short = print only name, line, and time (default)
who-help-time = print last system clock change
who-help-users = list users logged in
who-help-mesg = add user's message status as +, - or ?

# Output messages
who-user-count = # users={ $count }

# Idle time indicators
who-idle-current =   .
who-idle-old =  old
who-idle-unknown =   ?

# System information
who-runlevel = run-level { $level }
who-runlevel-last = last={ $last }
who-clock-change = clock change
who-login = LOGIN
who-login-id = id={ $id }
who-dead-exit-status = term={ $term } exit={ $exit }
who-system-boot = system boot

# Table headings
who-heading-name = NAME
who-heading-line = LINE
who-heading-time = TIME
who-heading-idle = IDLE
who-heading-pid = PID
who-heading-comment = COMMENT
who-heading-exit = EXIT

# Error messages
who-canonicalize-error = failed to canonicalize { $host }

# Platform-specific messages
who-unsupported-openbsd = unsupported command on OpenBSD
