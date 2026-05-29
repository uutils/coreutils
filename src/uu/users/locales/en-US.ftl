users-about = Print the user names of users currently logged in to the current host.
users-usage = users [FILE]
users-about-musl-warning =
    Warning: When built with musl libc, the `users` utility may show '0 users',
    due to musl's stub implementation of utmpx functions.
users-long-usage = Output who is currently logged in according to FILE.
    If FILE is not specified, use { $default_path }.  /var/log/wtmp as FILE is common.
