// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! A collection of procedural macros for the uutests integration test harness.

#![deny(missing_docs)]

use proc_macro::TokenStream;
use quote::quote;

/// Marks an integration test as `#[ignore]` under the WASI test runner, for one of a fixed
/// set of named, de-duplicated reasons.
///
/// This expands to `#[cfg_attr(wasi_runner, ignore = "...")]`, where `wasi_runner` is the
/// `--cfg` set only by the WASI CI job (see `.github/workflows/wasi.yml`) when host-compiled
/// integration tests drive a `wasm32-wasip1`/`wasm32-wasip2` binary through `wasmtime`. Every
/// test ignored under WASI names its reason here, even reasons currently used only once, so
/// the set of known WASI limitations stays in one place instead of scattered as ad hoc string
/// literals across the test suite.
#[proc_macro_attribute]
pub fn wasi_ignore(args: TokenStream, item: TokenStream) -> TokenStream {
    let key = args.to_string();
    let reason = match key.as_str() {
        "AbsoluteHostTmpdirOutsideGuestRoot" => {
            "WASI sandbox: absolute host tmpdir path is outside the mapped guest root, so cp cannot stat it"
        }
        "AbsoluteSymlinkTargetsUnfollowable" => {
            "WASI sandbox: absolute symlink targets cannot be followed"
        }
        "AclXattrArchivePreservationFails" => {
            "WASI: ACL/xattr syscalls are not implemented, so --archive's attribute preservation fails"
        }
        "AclXattrModePreservationFails" => {
            "WASI: ACL/xattr syscalls are not implemented, so -p's attribute preservation fails"
        }
        "AddressSpaceLimitUnsuitable" => {
            "WASI runner target is not suitable for this address-space-limit regression test"
        }
        "ArgvFilenamesUtf8" => "WASI: argv/filenames must be valid UTF-8",
        "ArgvUtf8" => "WASI: argv must be valid UTF-8",
        "CannotDetectUnsafeOverwrite" => "WASI: cannot detect unsafe overwrite",
        "CannotExecShellFromDeletedCwd" => {
            "WASI: wasmtime cannot exec the binary via a raw shell from a deleted cwd"
        }
        "ClosedStdoutGenericIoError" => {
            "WASI: closed stdout reports a generic I/O error rather than BrokenPipe"
        }
        "DebugReportsUnsupported" => {
            "WASI: --debug reports 'unsupported' instead of 'unknown' for this platform"
        }
        "DevNullNotSeekable" => "WASI sandbox: /dev/null is not a seekable device",
        "DirectoryModesUmaskNotFaithful" => {
            "WASI: directory modes/umask are not faithfully reproduced"
        }
        "DotDotCanonicalizationDiffers" => {
            "WASI sandbox: relative '..' path canonicalization differs, causing a false self-copy detection"
        }
        "EmptyPathResolvesToDirectory" => {
            "WASI: an empty path argument resolves to a directory instead of ENOENT"
        }
        "ErrnoMessageMismatch" => "WASI: errno/error-message mismatches",
        "GuestRootWritablePreopen" => {
            "WASI: guest root is a writable preopen, not the protected system root"
        }
        "HostPathsDevNotVisible" => "WASI sandbox: host paths (/dev) not visible",
        "HostPathsDevPrivateNotVisible" => "WASI sandbox: host paths (/dev, /private) not visible",
        "HostPathsNotVisible" => "WASI sandbox: host paths not visible",
        "HostPathsProcNotVisible" => "WASI sandbox: host paths (/proc) not visible",
        "HostPathsSysNotVisible" => "WASI sandbox: host paths (/sys) not visible",
        "KillDiscardsUnflushedOutput" => {
            "WASI: killing the wasmtime process discards the unflushed output buffer, so the streamed bytes never reach stdout"
        }
        "LcAllNotInherited" => "WASI: the guest does not inherit LC_ALL",
        "LocaleEnvNotPropagated" => "WASI: locale env vars not propagated",
        "NoBlockSizeSupport" => "WASI: no block size support",
        "NoChmod" => "WASI: no chmod syscall, so required mode/ownership preservation always fails",
        "NoFifo" => "WASI: no FIFO/mkfifo support",
        "NoFifoSupport" => "WASI: no FIFO support",
        "NoLocaleData" => "WASI: no locale data, every locale is C",
        "NoPermissionBits" => "WASI: no permission bits",
        "NoPipeSignalSupport" => "WASI: no pipe/signal support",
        "NoRlimitSetrlimitSupport" => "WASI: rlimit/setrlimit not supported",
        "NoSameFileHardlinkDetection" => "WASI: same-file hard link detection not supported",
        "NoStdoutFileRedirection" => "WASI: no stdout-to-file redirection",
        "NoSubprocessSpawning" => "WASI: no subprocess spawning",
        "NoTzdbDstValidationSkipped" => {
            "WASI: no tzdb; TZ env var is not honoured so DST validation is skipped"
        }
        "NoTzdbTimestampsDiffer" => {
            "WASI: no tzdb; TZ env var is not honoured so timezone-dependent timestamps differ"
        }
        "NoUidGid" => "WASI: no uid/gid",
        "NoUnixDomainSockets" => "WASI: no Unix domain socket support",
        "NonUtf8ArgsCantPassThroughHarness" => {
            "WASI: non-utf8 arguments cannot be passed through the spawned test harness"
        }
        "NonUtf8ArgsCantPassThroughWasmtime" => {
            "WASI sandbox: non-UTF-8 arguments can't be passed through wasmtime"
        }
        "PathCreateDirectoryNoModeParam" => {
            "WASI: path_create_directory has no mode parameter and chmod returns ENOSYS, so directories can't be created with restricted permissions"
        }
        "PermissionErrorsSurfaceAsEnoent" => {
            "WASI: filesystem permission errors surface as ENOENT rather than EACCES"
        }
        "PreEpochTimestampsUnrepresentable" => {
            "WASI: pre-epoch timestamps not representable by path_filestat_set_times"
        }
        "ReadLinkAbsoluteFailsViaHarness" => {
            "WASI: read_link on absolute paths fails under wasmtime via spawned test harness"
        }
        "ReflinkLinuxMacosOnly" => "WASI: --reflink is only supported on linux and macOS",
        "RequiresUtf8Paths" => {
            "WASI: requires valid UTF-8 paths, so the non-UTF-8 error is expected"
        }
        "SparseLinuxOnly" => "WASI: --sparse is only supported on linux",
        "StdinPositionNotPreserved" => "WASI: stdin file position not preserved through wasmtime",
        "StdoutDashUnsupported" => "WASI: touch - (stdout) is unsupported",
        "SymlinkHardlinkCaps" => {
            "WASI sandbox: symlink/hardlink capability restrictions cause dangling-symlink/same-file detection to differ"
        }
        "SymlinkLoopNoElooop" => {
            "WASI: symlink loop traversal does not surface ELOOP ('Too many levels of symbolic links')"
        }
        "SysProcNotExposed" => "WASI: /sys and /proc virtual files are not exposed to the sandbox",
        "TailFollowDisabled" => "WASI: tail follow mode disabled",
        "Usize32BitOverflow" => "WASI: usize is 32-bit, the host usize::MAX does not parse",
        "WasmtimeRejectsNonUtf8Args" => "WASI: wasmtime rejects non-UTF-8 arguments",
        other => panic!(
            "wasi_ignore: unknown reason key `{other}` (add it to tests/uutests_procs/src/lib.rs)"
        ),
    };

    let item = proc_macro2::TokenStream::from(item);
    let tokens = quote! {
        #[cfg_attr(wasi_runner, ignore = #reason)]
        #item
    };
    TokenStream::from(tokens)
}
