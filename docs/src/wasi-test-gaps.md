<!-- spell-checker:ignore tzdata preopen setrlimit NOFILE filestat kqueue sysinfo EISDIR ELOOP -->

# WASI integration test gaps

Tests annotated with `#[cfg_attr(wasi_runner, ignore = "...")]` are skipped when running integration tests against a WASI binary via wasmtime. This document tracks the reasons so that gaps in WASI support are visible in one place.

To find all annotated tests: `grep -rn 'wasi_runner, ignore' tests/`

## Tools not yet covered by integration tests

arch, b2sum, cksum, cp, csplit, date, dir, dircolors, fmt, join, ln, ls, md5sum, mkdir, mv, nproc, pathchk, pr, printenv, ptx, pwd, readlink, realpath, rm, rmdir, seq, sha1sum, sha224sum, sha256sum, sha384sum, sha512sum, shred, sleep, split, tsort, uname, uniq, vdir, yes

## WASI sandbox: host paths not visible

The WASI guest only sees directories explicitly mapped with `--dir`. Host paths like `/proc`, `/sys`, and `/dev` are not accessible. Affected tests include those that read `/proc/version`, `/proc/modules`, `/proc/cpuinfo`, `/proc/self/mem`, `/sys/kernel/profiling`, `/dev/null`, `/dev/zero`, `/dev/full`, and tests that rely on anonymous pipes or Linux-specific I/O error paths.

## WASI: argv/filenames must be valid UTF-8

The WASI specification requires that argv entries and filenames are valid UTF-8. Tests that pass non-UTF-8 bytes as arguments or create files with non-UTF-8 names cannot run under WASI.

## WASI: no FIFO/mkfifo support

WASI does not support creating or opening FIFOs (named pipes). Tests that use `mkfifo` are skipped.

## WASI: no pipe/signal support

WASI does not support Unix signals or pipe creation. Tests that rely on `SIGPIPE`, broken pipe detection, or pipe-based I/O are skipped.

## WASI: no subprocess spawning

WASI does not support spawning child processes. Tests that shell out to other commands or invoke a second binary are skipped.

## WASI: stdin file position not preserved through wasmtime

When stdin is a seekable file, wasmtime does not preserve the file position between the host and guest. Tests that validate stdin offset behavior after `head` reads are skipped.

## WASI: absolute symlink targets fail under wasmtime

Under wasmtime, symlinks whose stored target is an absolute guest path (for example `bar -> /foo`) fail in cases that work on POSIX: `readlink bar` exits 1 and `readlink -v bar` reports `Permission denied`, while the equivalent relative symlink (`bar -> foo`) succeeds. This reproduces even when `wasmtime` is invoked directly, so it is not specific to the cargo-test harness. `realpath` and symlink-heavy `cp` paths inherit the same limitation, and individual symptom tests under this umbrella are annotated with narrower reasons describing the observed errno mismatch.

## WASI: no Unix domain socket support

WASI does not support Unix domain sockets. Tests that create or read from `AF_UNIX` sockets are skipped.

## WASI: no locale data

The WASI sandbox does not ship locale data, so `setlocale`/`LC_ALL` have no effect and sorting falls back to byte comparison. Tests that depend on locale-aware collation or month-name translation are skipped.

## WASI: tail follow mode disabled

`tail -f` / `tail -F` (follow mode) requires change-notification mechanisms (`inotify`, `kqueue`) and signal handling that WASI does not provide, so follow is disabled on WASI and a warning is emitted. Tests that exercise follow behavior are skipped.

## WASI: cannot detect unsafe overwrite

`is_unsafe_overwrite` (used by `cat` to detect input-is-output situations) is stubbed to return `false` on WASI because the required `stat` / device-and-inode comparison is not available. Tests that assert this error path are skipped.

## WASI: pre-epoch timestamps not representable

WASI Preview 1 `Timestamp` is a `u64` nanosecond count since the Unix epoch, so `path_filestat_set_times` (and therefore `touch -t` with a two-digit year ≥ 69) cannot express dates before 1970. Tests that set pre-epoch timestamps are skipped.

## WASI: no timezone database

wasi-libc does not ship tzdata, so `TZ` is not honoured and timezone-dependent validation (e.g. `touch -t` rejecting a nonexistent local time during a DST transition) does not happen. Tests that rely on this are skipped.

## WASI: guest root is a writable preopen

The test harness maps the per-test working directory as the guest's `/`. That makes `/` writable inside the guest, so GNU-style protections against operating on the system root (e.g. `touch /` failing) cannot be exercised. It also means guest-visible absolute paths are rooted at `/`, not at the host tempdir. Tests that compare against host `canonicalize()` results or pass host absolute paths into the guest (for example some `cp --parents`, `readlink`, `realpath`, `pwd`, and `ls` cases) need guest-aware expectations or separate coverage. Tests that assert the root-protection behavior are skipped.

## WASI: `touch -` (stdout) unsupported

On WASI, `touch -` returns `UnsupportedPlatformFeature` because the guest cannot reliably locate the host file backing stdout. Tests that exercise `touch -` are skipped.

## WASI: rlimit/setrlimit not supported

WASI has no concept of per-process resource limits, so `setrlimit` (and the `rlimit` crate that wraps it) has no effect. Tests that set `RLIMIT_NOFILE` to verify behavior under restricted file-descriptor budgets are skipped.

## WASI: sysinfo/meminfo not available

WASI has no `sysinfo`/`/proc/meminfo` equivalent, so features that size buffers as a percentage of system memory (e.g. `sort -S 10%`) cannot resolve the limit and fail. Tests that exercise percentage-based sizing are skipped.

## WASI: errno/error-message mismatches

Several error paths surface different errno values (and therefore different error messages) through wasmtime than on POSIX. Observed cases:

- Opening a directory as a file returns `EBADF` rather than `EISDIR`.
- Redirecting a directory into stdin returns `ENOENT` rather than `EISDIR`.
- Filesystem permission errors surface as `ENOENT` rather than `EACCES`.
- Symlink-loop traversal does not reliably surface `ELOOP` ("Too many levels of symbolic links").
- Opening a symlink-to-directory does not reliably surface `EISDIR`.

Tests that assert specific error text for these paths are skipped.
