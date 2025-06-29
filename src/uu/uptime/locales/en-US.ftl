uptime-about = Display the current time, the length of time the system has been up,
  the number of users on the system, and the average number of jobs
  in the run queue over the last 1, 5 and 15 minutes.
uptime-usage = uptime [OPTION]...
uptime-about-musl-warning = Warning: When built with musl libc, the `uptime` utility may show '0 users'
    due to musl's stub implementation of utmpx functions. Boot time and load averages
    are still calculated using alternative mechanisms.

# Help messages
uptime-help-since = system up since
uptime-help-path = file to search boot time from

# Error messages
uptime-error-io = couldn't get boot time: { $error }
uptime-error-target-is-dir = couldn't get boot time: Is a directory
uptime-error-target-is-fifo = couldn't get boot time: Illegal seek
uptime-error-couldnt-get-boot-time = couldn't get boot time

# Output messages
uptime-output-unknown-uptime = up ???? days ??:??,

uptime-user-count = { $count ->
    [one] 1 user
   *[other] { $count } users
}

# Error messages
uptime-lib-error-system-uptime = could not retrieve system uptime
uptime-lib-error-system-loadavg = could not retrieve system load average
uptime-lib-error-windows-loadavg = Windows does not have an equivalent to the load average on Unix-like systems
uptime-lib-error-boot-time = boot time larger than current time

# Uptime formatting
uptime-format = { $days ->
    [0] { $time }
    [one] { $days } day, { $time }
   *[other] { $days } days { $time }
}

# Load average formatting
uptime-lib-format-loadavg = load average: { $avg1 }, { $avg5 }, { $avg15 }
