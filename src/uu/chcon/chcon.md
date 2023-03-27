<!-- spell-checker:ignore (vars) RFILE -->
# chcon

```
chcon [OPTION]... CONTEXT FILE...
chcon [OPTION]... [-u USER] [-r ROLE] [-l RANGE] [-t TYPE] FILE...
chcon [OPTION]... --reference=RFILE FILE...
```

Change the SELinux security context of each FILE to CONTEXT.
With --reference, change the security context of each FILE to that of RFILE.
