# cp

```
cp [OPTION]... [-T] SOURCE DEST
cp [OPTION]... SOURCE... DIRECTORY
cp [OPTION]... -t DIRECTORY SOURCE...
```

Copy SOURCE to DEST, or multiple SOURCE(s) to DIRECTORY.

## After Help

Do not copy a non-directory that has an existing destination with the same or newer modification timestamp;
instead, silently skip the file without failing. If timestamps are being preserved, the comparison is to the
source timestamp truncated to the resolutions of the destination file system and of the system calls used to
update timestamps; this avoids duplicate work if several `cp -pu` commands are executed with the same source
and destination. This option is ignored if the `-n` or `--no-clobber` option is also specified. Also, if
`--preserve=links` is also specified (like with `cp -au` for example), that will take precedence; consequently,
depending on the order that files are processed from the source, newer files in the destination may be replaced,
to mirror hard links in the source. which gives more control over which existing files in the destination are
replaced, and its value can be one of the following:

* `all`    This is the default operation when an `--update` option is not specified, and results in all existing files in the destination being replaced.
* `none`   This is similar to the `--no-clobber` option, in that no files in the destination are replaced, but also skipping a file does not induce a failure.
* `older`  This is the default operation when `--update` is specified, and results in files being replaced if they’re older than the corresponding source file.
