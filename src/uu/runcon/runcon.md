# runcon

```
runcon CONTEXT COMMAND [ARG...]
runcon [-c] [-u USER] [-r ROLE] [-t TYPE] [-l RANGE] COMMAND [ARG...]
```

Run command with specified security context under SELinux enabled systems.

## After Help

Run COMMAND with completely-specified CONTEXT, or with current or transitioned security context modified by one or more of LEVEL, ROLE, TYPE, and USER.

If none of --compute, --type, --user, --role or --range is specified, then the first argument is used as the complete context.

Note that only carefully-chosen contexts are likely to successfully run.

If neither CONTEXT nor COMMAND is specified, the current security context is printed.
