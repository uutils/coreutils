<!-- spell-checker:ignore markdownlint ; (misc) backends kqueue Testsuite ksyms stdlib -->

# Notes / ToDO

## Missing features

* `--max-unchanged-stats`

Note:
There's a stub for `--max-unchanged-stats` so GNU test-suite checks using it
can run, however this flag has no functionality yet.

### Platform support for `--follow` and `--retry`

The `--follow=descriptor`, `--follow=name` and `--retry` flags have very good
support on Linux (inotify backend).
They work good enough on macOS/BSD (kqueue backend) with some tests failing due
to differences of how kqueue works compared to inotify.
Windows support is there in theory due to ReadDirectoryChanges support by the
notify-crate, however these flags are completely untested on Windows.

Note:
The undocumented `---disable-inotify` flag is used to disable the inotify
backend to test polling.
However inotify is a Linux only backend and polling is now supported also
for the other backends.
Because of this, `disable-inotify` is now an alias to the new and more versatile
flag name: `--use-polling`.

## Possible optimizations

* Don't read the whole file if not using `-f` and input is regular file.
  Read in chunks from the end going backwards, reading each individual chunk
  forward.
* Reduce number of system calls to e.g. `fstat`
* Improve resource management by adding more system calls to `inotify_rm_watch`
  when appropriate.

## GNU test-suite results (9.1.8-e08752)

The functionality for the test "gnu/tests/tail-2/follow-stdin.sh" is implemented.
It fails because it is provoking closing a file descriptor with `tail -f <&-`
and as part of a workaround, Rust's stdlib reopens closed FDs as `/dev/null`
which means uu_tail cannot detect this.
See also, e.g. the discussion at:
<https://github.com/uutils/coreutils/issues/2873>

The functionality for the test "gnu/tests/tail-2/inotify-rotate-resources.sh"
is implemented.
It fails with an error because it is using `strace` to look for calls to
`inotify_add_watch` and `inotify_rm_watch`,
however in uu_tail these system calls are invoked from a separate thread.
If the GNU test would follow threads, i.e. use `strace -f`, this issue could be
resolved.

There are 5 tests which are fixed but do not (always) pass the test suite
if it's run inside the CI.
The reason for this is probably related to load/scheduling on the CI test VM.
The tests in question are:

* [x] `tail-2/F-vs-rename.sh`
* [x] `tail-2/follow-name.sh`
* [x] `tail-2/inotify-rotate.sh`
* [x] `tail-2/overlay-headers.sh`
* [x] `tail-2/retry.sh`
