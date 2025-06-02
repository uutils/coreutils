who-about = Print information about users who are currently logged in.
who-usage = who [OPTION]... [ FILE | ARG1 ARG2 ]
who-about-musl-warning = Note: When built with musl libc, the `who` utility will not display any
    information about logged-in users. This is due to musl's stub implementation
    of `utmpx` functions, which prevents access to the necessary data.
