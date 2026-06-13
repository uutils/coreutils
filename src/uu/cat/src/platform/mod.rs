// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(unix)]
pub use self::unix::is_unsafe_overwrite;

#[cfg(windows)]
pub use self::windows::is_unsafe_overwrite;

// WASI: when stdout is inherited from a host file descriptor, wasmtime
// reports its fstat as all-zero (st_dev == st_ino == 0), so the dev/inode
// comparison against any input file descriptor can never match. There is
// no reliable way to detect unsafe overwrite here; assume safe rather than
// risk a spurious error.
#[cfg(target_os = "wasi")]
pub fn is_unsafe_overwrite<I, O>(_input: &I, _output: &O) -> bool {
    false
}

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;
