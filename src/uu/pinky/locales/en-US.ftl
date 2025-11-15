pinky-about = Displays brief user information on Unix-based systems
pinky-usage = pinky [OPTION]... [USER]...
pinky-about-musl-warning = Warning: When built with musl libc, the `pinky` utility may show incomplete
    or missing user information due to musl's stub implementation of `utmpx`
    functions. This limitation affects the ability to retrieve accurate details
    about logged-in users.

# Long usage description
pinky-long-usage-description = A lightweight 'finger' program;  print user information.
  The utmp file will be

# Help messages
pinky-help-long-format = produce long format output for the specified USERs
pinky-help-omit-home-dir = omit the user's home directory and shell in long format
pinky-help-omit-project-file = omit the user's project file in long format
pinky-help-omit-plan-file = omit the user's plan file in long format
pinky-help-short-format = do short format output, this is the default
pinky-help-omit-headings = omit the line of column headings in short format
pinky-help-omit-name = omit the user's full name in short format
pinky-help-omit-name-host = omit the user's full name and remote host in short format
pinky-help-omit-name-host-time = omit the user's full name, remote host and idle time in short format
pinky-help-lookup = attempt to canonicalize hostnames via DNS
pinky-help-help = Print help information

# Column headers for short format
pinky-column-login = Login
pinky-column-name = Name
pinky-column-tty =  TTY
pinky-column-idle = Idle
pinky-column-when = When
pinky-column-where = Where

# Labels for long format
pinky-login-name-label = Login name:
pinky-real-life-label = In real life:
pinky-directory-label = Directory:
pinky-shell-label = Shell:
pinky-project-label = Project:
pinky-plan-label = Plan

# Status messages
pinky-unsupported-openbsd = unsupported command on OpenBSD
