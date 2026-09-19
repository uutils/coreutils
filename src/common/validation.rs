// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore execveat fexecve memfd prefixcat rsplit testcat atexit profraw Cprofile

use std::ffi::{OsStr, OsString};
use std::io::{Write, stderr};
use std::path::{Path, PathBuf};
use std::process;

use uucore::Args;
use uucore::display::Quotable;
use uucore::locale;

// The instrumented binary built by `util/build-pgo.sh` (`--cfg pgo_training`)
// has to flush its own profile counters on Windows. Every exit below goes
// through `std::process::exit`, which is `libc::exit` on Unix but
// `ExitProcess` on Windows; the latter skips the `atexit` handler the LLVM
// profiling runtime writes the counters from, so the training runs would
// leave nothing but empty `.profraw` files behind.
#[cfg(all(pgo_training, windows))]
unsafe extern "C" {
    fn __llvm_profile_write_file() -> i32;
}

/// Terminates the process with `code`, flushing the PGO counters when the
/// binary was built for profile training on Windows.
pub fn exit(code: i32) -> ! {
    #[cfg(all(pgo_training, windows))]
    // SAFETY: `__llvm_profile_write_file` is provided by the LLVM profiling
    // runtime, which is linked in whenever this cfg is set (the same build
    // passes `-Cprofile-generate`). It takes no arguments and only writes the
    // counter file named by `LLVM_PROFILE_FILE`.
    unsafe {
        __llvm_profile_write_file();
    }
    process::exit(code)
}

/// Gets all available utilities including "coreutils"
#[allow(clippy::type_complexity)]
pub fn get_all_utilities<T: Args>(
    util_map: &phf::OrderedMap<&'static str, (fn(T) -> i32, fn() -> clap::Command)>,
) -> Vec<&'static str> {
    std::iter::once("coreutils")
        .chain(util_map.keys().copied())
        .collect()
}

/// Prints a "utility not found" error and exits
pub fn not_found(util: &OsStr) -> ! {
    let _ = writeln!(
        stderr(),
        "coreutils: unknown program '{}'",
        util.maybe_quote()
    );
    exit(1);
}

/// Prints an "unrecognized option" error and exits
pub fn unrecognized_option(binary_name: &str, option: &OsStr) -> ! {
    let _ = writeln!(
        stderr(),
        "{binary_name}: unrecognized option '{}'",
        option.to_string_lossy()
    );
    exit(1);
}

/// Sets up localization for a utility with proper error handling
pub fn setup_localization_or_exit(util_name: &str) {
    let util_name = get_canonical_util_name(util_name);
    locale::setup_localization(util_name).unwrap_or_else(|err| {
        match err {
            locale::LocalizationError::ParseResource {
                error: err_msg,
                snippet,
            } => eprintln!("Localization parse error at {snippet}: {err_msg}"),
            other => eprintln!("Could not init the localization system: {other}"),
        }
        exit(99)
    });
}

/// Gets the canonical utility name, resolving aliases
fn get_canonical_util_name(util_name: &str) -> &str {
    match util_name {
        // uu_test aliases - '[' is an alias for test
        "[" => "test",
        "dir" | "vdir" => "ls", // aliases for ls

        // Default case - return the util name as is
        _ => util_name,
    }
}

/// Gets the binary path from command line arguments
/// Panics if the binary path cannot be determined
#[cfg(any(
    not(any(
        target_os = "linux",
        all(target_os = "android", target_pointer_width = "64")
    )),
    target_env = "musl"
))]
pub fn binary_path(args: &mut impl Iterator<Item = OsString>) -> PathBuf {
    PathBuf::from(args.next().unwrap())
    // no fallback for empty args. current_exe() (/proc/self/exe) is valid only for hardlinks
}
/// Get actual binary path from kernel, not argv0, to prevent `env -a` from bypassing
/// AppArmor, SELinux policies on hard-linked binaries
#[cfg(all(
    any(
        target_os = "linux",
        all(target_os = "android", target_pointer_width = "64")
    ),
    not(target_env = "musl")
))]
pub fn binary_path(args: &mut impl Iterator<Item = OsString>) -> PathBuf {
    use std::fs::File;
    use std::io::Read;
    use std::os::unix::ffi::OsStrExt;
    let execfn = rustix::param::linux_execfn();
    let execfn_bytes = execfn.to_bytes();
    let exec_path = Path::new(OsStr::from_bytes(execfn_bytes));
    let argv0 = args.next().unwrap();
    let mut shebang_buf = [0u8; 2];
    // exec_path is wrong when called from a shebang, or via fexecve/execveat:
    // the kernel reports /dev/fd/* and memfd_create/glibc's fallback /proc/self/fd/*
    // argv0 is not full-path when called from PATH
    if execfn_bytes.rsplit(|&b| b == b'/').next() == argv0.as_bytes().rsplit(|&b| b == b'/').next()
        || execfn_bytes.starts_with(b"/proc/")
        || execfn_bytes.starts_with(b"/dev/fd/")
        || (File::open(Path::new(exec_path))
            .and_then(|mut f| f.read_exact(&mut shebang_buf))
            .is_ok()
            && &shebang_buf == b"#!")
    {
        argv0.into()
    } else {
        exec_path.into()
    }
}
/// Extracts the binary name from a path
pub fn name(binary_path: &Path) -> Option<&str> {
    binary_path.file_stem()?.to_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_canonical_util_name() {
        // Test a few key aliases
        assert_eq!(get_canonical_util_name("["), "test");
        assert_eq!(get_canonical_util_name("dir"), "ls");

        // Test passthrough case
        assert_eq!(get_canonical_util_name("cat"), "cat");
    }

    #[test]
    fn test_name() {
        // Test normal executable name
        assert_eq!(name(Path::new("/usr/bin/ls")), Some("ls"));
        assert_eq!(name(Path::new("cat")), Some("cat"));
        assert_eq!(
            name(Path::new("./target/debug/coreutils")),
            Some("coreutils")
        );

        // Test with extensions
        assert_eq!(name(Path::new("program.exe")), Some("program"));
        assert_eq!(name(Path::new("/path/to/utility.bin")), Some("utility"));

        // Test edge cases
        assert_eq!(name(Path::new("")), None);
        assert_eq!(name(Path::new("/")), None);
    }
}
