# Notes / ToDO

- Rudimentary tail implementation.

## Missing features

### Flags with features

- [ ] `--max-unchanged-stats` : with `--follow=name`, reopen a FILE which has not changed size after N (default 5) iterations  to see if it has been unlinked or renamed (this is the usual case of rotated log files).  With `inotify`, this option is rarely useful.
- [ ] `--retry` : keep trying to open a file even when it is or becomes inaccessible; useful when follow‐ing by name, i.e., with `--follow=name`

### Others

- [ ] The current implementation doesn't follow stdin in non-unix platforms

## Possible optimizations

- [ ] Don't read the whole file if not using `-f` and input is regular file. Read in chunks from the end going backwards, reading each individual chunk forward.
