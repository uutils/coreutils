# WASI integration test gaps

Tests annotated with `#[cfg_attr(wasi_runner, ignore = "...")]` are skipped when running integration tests against a WASI binary via wasmtime. This document tracks the reasons so that gaps in WASI support are visible in one place.

To find all annotated tests: `grep -rn 'wasi_runner, ignore' tests/`

## Tools not yet covered by integration tests

arch, b2sum, cat, cksum, cp, csplit, date, dir, dircolors, fmt, join, ls, md5sum, mkdir, mv, nproc, pathchk, pr, printenv, ptx, pwd, readlink, realpath, rm, rmdir, seq, sha1sum, sha224sum, sha256sum, sha384sum, sha512sum, shred, sleep, sort, split, tail, touch, tsort, uname, uniq, vdir, yes

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

## WASI: read_link on absolute paths fails under wasmtime via spawned test harness

`fs::read_link` on an absolute path inside the sandbox (e.g. `/file2`) returns `EPERM` when the WASI binary is launched through `std::process::Command` from the test harness, even though the same call works when wasmtime is invoked directly. This breaks `uucore::fs::canonicalize` for symlink sources, so tests that rely on following a symlink to compute a relative path are skipped.
