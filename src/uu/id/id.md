# id

```
id [OPTION]... [USER]...
```

Print user and group information for each specified `USER`,
or (when `USER` omitted) for the current user.

## After help

The id utility displays the user and group names and numeric IDs, of the
calling process, to the standard output. If the real and effective IDs are
different, both are displayed, otherwise only the real ID is displayed.

If a user (login name or user ID) is specified, the user and group IDs of
that user are displayed. In this case, the real and effective IDs are
assumed to be the same.
