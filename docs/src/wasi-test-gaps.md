# WASI integration test gaps

Tests annotated with `#[cfg_attr(wasi_runner, ignore = "...")]` or `#[cfg_attr(wasip2_runner, ignore = "...")]` are skipped when running integration tests against a WASI binary via wasmtime. This document tracks the reasons so that gaps in WASI support are visible in one place.

To find all annotated tests: `grep -rn 'wasi_runner, ignore\|wasip2_runner, ignore' tests/`

To find the tests for a specific reason: `grep -rn '<reason text>' tests/`

## WASI sandbox: host paths not visible

The WASI guest only sees directories explicitly mapped with `--dir`. Host paths outside those mappings are not accessible, so any test that reads or writes an absolute host path fails. This is the single largest gap and covers many sub-cases:

- Generic unmapped absolute paths ("WASI sandbox: host paths not visible", "WASI sandbox: absolute host paths not visible", "WASI sandbox: cross-scenario absolute host path not visible").
- `/dev` special files: `/dev/null`, `/dev/zero`, `/dev/random`, `/dev/console`, and `/dev` generally.
- `/proc` and `/sys`: `/proc/version`, `/proc/modules`, `/proc/cpuinfo`, `/proc/self/mem`, `/proc/1/cmdline`, `/sys/kernel/profiling`, and `/proc`/`/sys` generally.
- Locale and timezone databases, which live under host paths the guest never sees ("WASI sandbox: locale database not visible", "WASI sandbox: timezone database not visible", "WASI sandbox: timezone/locale database not visible", including the `--time-style=locale` and "host locale check passes but the wasm guest can't use it" variants).
- Path identity/display mismatches that stem from the same root cause — the guest's view of the filesystem is a virtual root, not the host's real one: `pwd`/`getcwd` report the guest's virtual root instead of the host absolute path ("WASI sandbox: pwd reports the guest's virtual root..."), canonicalized paths resolve to the virtual root instead of the host path ("WASI sandbox: canonicalized path resolves to the guest's virtual root..."), `current_directory_resolved` used to build expected hyperlink URIs differs from the host path, `/` inside the guest is its own writable root rather than the real filesystem root, and UNC-style paths resolve differently inside the guest root than on the host.

## WASI: argv/filenames must be valid UTF-8

The WASI specification requires that argv entries, environment values, and filenames are valid UTF-8. Tests that pass non-UTF-8 bytes as arguments, environment values, or create files with non-UTF-8 names cannot run under WASI ("WASI: argv/filenames must be valid UTF-8", "WASI: argv must be valid UTF-8", "WASI: env values must be valid UTF-8", "WASI: preopened directories reject non-UTF-8 filenames", "WASI: non-utf8 arguments cannot be passed through the spawned test harness", "WASI preview2: OsString requires valid UTF-8, unlike unix/wasip1").

## WASI: no FIFO/mkfifo support

WASI does not support creating or opening FIFOs (named pipes). Tests that use `mkfifo`, classify files via `FileTypeExt::is_fifo()`, or read from a FIFO (which surfaces as `EINVAL`/"Invalid seek" instead of blocking) are skipped.

## WASI: no pipe/signal support

WASI does not support Unix signals or pipe creation. Tests that rely on `SIGPIPE`, `SIGINT`, broken pipe detection, or pipe-based I/O are skipped ("WASI: no pipe/signal support", "WASI: no signal support", "WASI: no signal support (SIGINT)").

## WASI: no subprocess spawning

WASI does not support spawning child processes. Tests that shell out to other commands, invoke a second binary, or rely on `--filter`'s process-spawning support are skipped ("WASI: no subprocess spawning", "WASI: --filter has no process-spawning support").

## WASI: no stdout-to-file redirection

Tests that redirect a subprocess's stdout directly to a file outside the test harness's own plumbing are skipped; the wasmtime runner does not support this redirection path.

## WASI: sparse/reflink/ACL copy-on-write features not supported

`cp`'s sparse-file detection, reflink/copy-on-write (`--reflink`), and ACL preservation rely on Linux-specific filesystem features that wasmtime's virtualized filesystem does not implement.

## WASI: follow mode (-f) is not supported on this platform

`tail -f` and related follow-mode behavior depend on OS-level file-change notification that is not available to a WASI guest.

## WASI: st_mode has no real permission bits

WASI's `stat` only reports file type, not real Unix permission bits, so `st_mode` is a placeholder. Tests that assert on permission bits, mode-dependent coloring, or mode-dependent output are skipped ("WASI: st_mode has no real permission bits", "...only file-type; ls -l shows placeholder rwx", "...color/mode-dependent output differs").

## WASI: chmod/umask have no real effect in the guest sandbox

`chmod` has no ENOSYS-free syscall in the WASI guest, so tests that expect `chmod` to restore write permission or make a directory read-only are skipped. Similarly `umask()` only affects the wasmtime host process, not the guest sandbox, so tests asserting on umask-influenced output are skipped.

## WASI: sort -m spawns real OS threads for multi-file merge

`sort -m`'s multi-file merge path spawns real OS threads, which is unsupported under wasmtime's default configuration. This also affects `--compress-program`, since `ext_sort` falls back to a single-threaded in-memory path that bypasses the external compress program entirely.

## WASI: File::try_clone() is unsupported

`shuf --random-source` (and similar) rely on `File::try_clone()`, which is unsupported under the WASI runtime.

## WASI: read_link on absolute paths fails under wasmtime via spawned test harness

`fs::read_link` on an absolute path inside the sandbox (e.g. `/file2`) returns `EPERM` when the WASI binary is launched through `std::process::Command` from the test harness, even though the same call works when wasmtime is invoked directly. This breaks `uucore::fs::canonicalize` for symlink sources, so tests that rely on following a symlink to compute a relative path are skipped ("WASI: read_link on absolute paths fails...", "WASI: read_link() not supported for symlinks").

## WASI: stdin file position not preserved through wasmtime

When stdin is a seekable file, wasmtime does not preserve the file position between the host and guest. Tests that validate stdin offset behavior after `head` reads are skipped.

## WASI: inode/ctime/atime metadata gaps

Several `stat`-adjacent metadata fields are unreliable or unavailable under WASI: inode display needs a `rustix::fs::stat`-based path and is currently gated to unix only; `ctime` is unavailable via `std::fs::Metadata` on stable; access/change time tracking granularity does not match the host filesystem's; and `rustix::fs::stat` doesn't return stable inode identity across path lookups under wasmtime ("WASI preview2: rustix::fs::stat doesn't return stable inode identity...").

## WASI: utimensat rejects negative (pre-1970) timestamps

Setting a file's timestamp to before the Unix epoch fails with `EINVAL` under WASI's `utimensat`, unlike native Unix.

## WASI: setting the system clock is not supported at all

Unlike native Unix, where changing the system clock is merely permission-gated, WASI does not support setting the system clock at all.

## WASI: direct file descriptor manipulation not supported

Tests that manipulate file descriptors directly (e.g. reusing a descriptor across `dup`-like operations to alias stdin/stdout to the same file) rely on primitives not supported under WASI.

## WASI: no /dev/fd/0 support

Without `/dev/fd/0`, a redirected-directory stdin hits the generic pipe error path (like macOS) instead of the regular-file path with the GNU-matching error message.

## WASI: killing the wasmtime process discards the unflushed output buffer

When a test kills the wasmtime process to check partial output, any unflushed output buffer is discarded, so streamed bytes never reach stdout the way they would with a natively killed process.

## WASI: resource limits not supported

Tests that use `rlimit` to constrain resources (file descriptors, address space) don't observe the expected behavior under wasmtime, since it doesn't enforce host-style resource limits inside the guest. This includes address-space-limit regression tests, for which "the WASI runner target is not suitable."

## WASI Preview2: exit with code requires an opt-in feature

`std::process::exit` on `wasm32-wasip2` goes through the *stable* `wasi:cli/exit#exit`
function, which only carries a success/failure bit, so every nonzero exit code
collapses to `1`.

The [wasi:cli/exit#exit-with-code](https://github.com/WebAssembly/WASI/blob/a1fc383d01eabaf3fac01de03c0ab1a01bfdd099/proposals/cli/wit/exit.wit#L16)
function propagates the real exit code, but it is marked `@unstable` in the
WIT definition. `uucore`'s `wasip2-exit-with-code` Cargo feature (off by
default) switches `uucore::error::process_exit` to call it via the `wasip2`
crate. Because the function is unstable, any WASI host must explicitly opt
in or the process traps instead of exiting (e.g. wasmtime requires
`-S cli-exit-with-code=y`). CI builds with this feature enabled and passes
that flag, so integration tests see the real exit code.

## WASI Preview2: OS error message text/mapping differs from native Unix

Some I/O error paths produce a different underlying OS error, or a
different-but-equivalent error text, under WASI Preview2 than on native
Unix, even though the exit code is the same. Examples: reading a directory
as a file surfaces as `Bad file descriptor` instead of `Is a directory`;
`stat` on a dangling symlink with a trailing slash returns `ENOTDIR` instead
of `ENOENT`; `stat` on a trailing-slash path over a regular file surfaces a
raw `ENOTDIR` from the runtime instead of going through the
`CannotStatNotADirectory` path. Tests that assert on the exact error text or
error code for these paths are skipped under `wasip2_runner`/`wasi_runner`.

## WASI P2: /dev/full filesystem not available

`/dev/full` (a device that always reports "No space left on device" on
write) is a Linux/FreeBSD/NetBSD-specific device node not present in the
WASI Preview2 sandbox. Tests that pipe output to `/dev/full` to exercise a
write-failure path are skipped.

## Harness/environment mismatches (not WASI capability gaps)

A handful of skipped tests aren't blocked by a WASI capability at all — they're blocked by how the test harness or CI environment invokes the binary:

- Tests that invoke the binary directly via a shell script or `std::process::Command`, bypassing the wasmtime runner entirely, can't run against the WASI binary as-is.
- A test that sets `HOME` to a relative path breaks wasmtime's own cache-dir resolution.
- A test that asserts on macOS-specific error text can't pass when the wasm guest under test isn't macOS, even though the test binary itself runs on macOS.
- `ls`'s open-fd-leak regression test hits wasmtime's own `--dir` sandbox fd/depth limit before reaching the 30-level depth the test is trying to probe, unrelated to whether `ls` itself leaks descriptors.
- `sort`'s buffer-size test with `u64::MAX`-sized arguments overflows differently on WASI because the wasm guest is always a 32-bit target regardless of the host's pointer width.

## WASI: the `same-file` crate has no WASI backend

`cp --link` on a symlinked directory needs to detect whether source and
destination are the same file. That check goes through the `same-file`
crate, which has no `wasm32-wasip1`/`wasm32-wasip2` implementation and always
returns "same-file is not supported on this platform" on that target.

## Needs investigation

A number of `cp`, `ls`, and `mv` tests were originally bulk-tagged with the
placeholder `ignore = "WASI: needs investigation (chmod/interactive/device/mode gaps)"`
pending a closer look. Most have since been triaged: 59 of them actually pass
under wasmtime (their `ignore` attribute has been removed) once the earlier
placeholder was replaced with real testing. Search for `needs investigation`
in `tests/by-util/test_cp.rs`, `tests/by-util/test_ls.rs`, and
`tests/by-util/test_mv.rs` to find the ones still pending.

The tests still carrying the placeholder fall into a few buckets, none fixable
with a small coreutils-side change:

- **chmod/permission-mode preservation**: `cp --preserve=mode`, `-p`, `-a`,
  and friends hit `ENOSYS` ("Function not implemented") when they try to
  `chmod` under WASI, since there is no working `fchmod`/`chmod` syscall in
  the guest sandbox.
- **File type gaps**: char devices, sockets, and other special files can't be
  `stat`ed or copied (`cp: cannot stat`, `cp: ... Not supported`) because the
  WASI guest doesn't expose real device/socket types.
- **`filetime` has no WASI backend**: preserving timestamps through symlinks
  (`set_symlink_file_times`) hits the crate's `wasm.rs` stub, which
  unconditionally returns "Wasm not implemented".
- **Debug/reflink text mismatches**: `cp --debug`'s `copy offload`/`reflink`/
  `sparse detection` fields, and the `--reflink` error message, are written
  assuming a `target_os = "linux"`/`"macos"` host and don't have a WASI-specific
  branch, so the text differs from what the test expects.
- **Virtual-root-relative absolute paths**: a few tests build an absolute
  path via `root_dir_resolved()` or copy `.` into a sibling directory
  reached through `..`; both hit the same "WASI sandbox: pwd reports the
  guest's virtual root, not the host's absolute path" limitation described
  above, just via a different code path (`cp: cannot stat` on the
  synthesized absolute path, or `cp: cannot copy a directory ... into
  itself` on the `..`-relative destination).
