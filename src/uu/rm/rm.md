# rm

```
rm [OPTION]... FILE...
```

Remove (unlink) the FILE(s)

## After Help

By default, rm does not remove directories.  Use the --recursive (-r or -R)
option to remove each listed directory, too, along with all of its contents

To remove a file whose name starts with a '-', for example '-foo',
use one of these commands:
rm -- -foo

rm ./-foo

Note that if you use rm to remove a file, it might be possible to recover
some of its contents, given sufficient expertise and/or time.  For greater
assurance that the contents are truly unrecoverable, consider using shred.
