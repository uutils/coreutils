# Notes / ToDO

## Missing features

The `-F` flag (same as `--follow=name --retry`) has very good support on Linux (inotify backend),
works good enough on macOS/BSD (kqueue backend) with some minor tests not working,
and is fully untested on Windows.

### Flags with features

- [x] fast poll := '-s.1 --max-unchanged-stats=1'
    - [x] sub-second sleep interval e.g. `-s.1`
    - [ ] `--max-unchanged-stats` (only meaningful with `--follow=name` `---disable-inotify`)
- [x] `---disable-inotify` (three hyphens is correct)
- [x] `--follow=name'
- [x] `--retry'
- [x] `-F' (same as `--follow=name` `--retry`)

### Others

- [ ] The current implementation doesn't follow stdin in non-unix platforms
- [ ] Since the current implementation uses a crate for polling, these flags are too complex to implement:
    - [ ] `--max-unchanged-stats`
    - [ ] check whether process p is alive at least every number of seconds (relevant for `--pid`)

## Possible optimizations

- [ ] Don't read the whole file if not using `-f` and input is regular file. Read in chunks from the end going backwards, reading each individual chunk forward.
