# false

```
false
```

Returns false, an unsuccessful exit status.

Immediately returns with the exit status `1`. When invoked with one of the recognized options it
will try to write the help or version text. Any IO error during this operation is diagnosed, yet
the program will also return `1`.
