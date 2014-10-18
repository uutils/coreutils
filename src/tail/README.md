Rudimentary tail implementation.

##Missing features:

### Flags with features
* `--bytes` : does not handle size suffixes
* `--lines` : does not handle size suffixes
* `--max-unchanged-stats` : with `--follow=name`, reopen a FILE which has not changed size after N (default 5) iterations  to see if it has been unlinked or renamed (this is the usual case of rotated log files).  With inotify, this option is rarely useful.
* `--pid` : with `-f`, terminate after process ID, PID dies
* `--quiet` : never output headers giving file names
* `--retry` : keep trying to open a file even when it is or becomes inaccessible; useful when follow‐ing by name, i.e., with `--follow=name`
* `--verbose` : always output headers giving file names

### Others
The current implementation does not handle `-` as an alias for stdin.

##Possible optimizations:
* Don't read the whole file if not using `-f` and input is regular file. Read in chunks from the end going backwards, reading each individual chunk forward.
