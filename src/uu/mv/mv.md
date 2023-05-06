# mv

```
mv [OPTION]... [-T] SOURCE DEST
mv [OPTION]... SOURCE... DIRECTORY
mv [OPTION]... -t DIRECTORY SOURCE...
```
Move `SOURCE` to `DEST`, or multiple `SOURCE`(s) to `DIRECTORY`.

## After Help

When specifying more than one of -i, -f, -n, only the final one will take effect.

Do not move a non-directory that has an existing destination with the same or newer modification timestamp;
instead, silently skip the file without failing. If the move is across file system boundaries, the comparison is
to the source timestamp truncated to the resolutions of the destination file system and of the system calls used
to update timestamps; this avoids duplicate work if several `mv -u` commands are executed with the same source
and destination. This option is ignored if the `-n` or `--no-clobber` option is also specified. which gives more control
over which existing files in the destination are replaced, and its value can be one of the following:

* `all`    This is the default operation when an `--update` option is not specified, and results in all existing files in the destination being replaced.
* `none`   This is similar to the `--no-clobber` option, in that no files in the destination are replaced, but also skipping a file does not induce a failure.
* `older`  This is the default operation when `--update` is specified, and results in files being replaced if they’re older than the corresponding source file.
