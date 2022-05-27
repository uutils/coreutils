<!-- spell-checker:ignore markdownlint ; (libs) kqueue -->

# Notes / ToDO

## Missing features

* `--max-unchanged-stats`
* The current implementation doesn't follow stdin on non-unix platforms
* check whether process p is alive at least every number of seconds (relevant for `--pid`)

Note:
There's a stub for `--max-unchanged-stats` so GNU tests using it can run, however this flag has no functionality yet.

### Platform support for `--follow`
The `--follow=descriptor`, `--follow=name` and `--retry` flags have very good support on Linux (inotify backend).
They work good enough on macOS/BSD (kqueue backend) with some tests failing due to differences how kqueue works compared to inotify.
Windows support is there in theory due to support by the notify-crate, however it's completely untested.

### Flags with features

- [x] fast poll := `-s.1 --max-unchanged-stats=1`
    - [x] sub-second sleep interval e.g. `-s.1`
    - [ ] `--max-unchanged-stats` (only meaningful with `--follow=name` `---disable-inotify`)
- [x] `--follow=name`
- [x] `--retry`
- [x] `-F` (same as `--follow=name` `--retry`)
- [x] `---disable-inotify` (three hyphens is correct)
- [x] `---presume-input-pipe` (three hyphens is correct)

Note:
`---disable-inotify` means to use polling instead of inotify,
however inotify is a Linux only backend and polling is now supported also for the other backends.
Because of this, `disable-inotify` is now an alias to the new and more versatile flag name: `--use-polling`.

## Possible optimizations

* Don't read the whole file if not using `-f` and input is regular file. Read in chunks from the end going backwards, reading each individual chunk forward.
* Reduce number of system calls to e.g. `fstat`

# GNU tests results

The functionality for the test "gnu/tests/tail-2/follow-stdin.sh" is implemented.
It fails because it is provoking closing a file descriptor with `tail -f <&-` and as part of a workaround, Rust's stdlib reopens closed FDs as `/dev/null` which means uu_tail cannot detect this.
See also, e.g. the discussion at: https://github.com/uutils/coreutils/issues/2873

The functionality for the test "gnu/tests/tail-2/inotify-rotate-resourced.sh" is implemented.
It fails with an error because it is using `strace` to look for calls to 'inotify_add_watch' and 'inotify_rm_watch',
however in uu_tail these system calls are invoked from a seperate thread. If the GNU test would use `strace -f` this issue could be resolved.

## Testsuite summary for GNU coreutils 9.1.8-e08752:

### PASS:
tail-2/F-headers.sh
tail-2/F-vs-missing.sh
tail-2/append-only.sh # skipped test: must be run as root
tail-2/assert-2.sh
tail-2/assert.sh
tail-2/big-4gb.sh
tail-2/descriptor-vs-rename.sh
tail-2/end-of-device.sh # skipped test: must be run as root
tail-2/flush-initial.sh
tail-2/follow-name.sh
tail-2/inotify-dir-recreate.sh
tail-2/inotify-hash-abuse.sh
tail-2/inotify-hash-abuse2.sh
tail-2/inotify-only-regular.sh
tail-2/inotify-rotate.sh
tail-2/overlay-headers.sh
tail-2/pid.sh
tail-2/pipe-f2.sh
tail-2/proc-ksyms.sh
tail-2/start-middle.sh
tail-2/tail-c.sh
tail-2/tail-n0f.sh
tail-2/truncate.sh

### SKIP:
tail-2/inotify-race.sh # skipped test: breakpoint not hit
tail-2/inotify-race2.sh # skipped test: breakpoint not hit
tail-2/pipe-f.sh # skipped test: trapping SIGPIPE is not supported

### FAIL:
misc/tail.pl
tail-2/F-vs-rename.sh
tail-2/follow-stdin.sh
tail-2/retry.sh
tail-2/symlink.sh
tail-2/wait.sh

### ERROR:
tail-2/inotify-rotate-resources.sh
