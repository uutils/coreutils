<!-- spell-checker:ignore RFILE ugoa -->

# chmod

```
chmod [OPTION]... MODE[,MODE]... FILE...
chmod [OPTION]... OCTAL-MODE FILE...
chmod [OPTION]... --reference=RFILE FILE...
```

Change the mode of each FILE to MODE.
With --reference, change the mode of each FILE to that of RFILE.

## After Help

Each MODE is of the form '[ugoa]*([-+=]([rwxXst]*|[ugo]))+|[-+=]?[0-7]+'.
