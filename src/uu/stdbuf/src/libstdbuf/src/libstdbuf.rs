// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) getreent reent IOFBF IOLBF IONBF setvbuf stderrp stdinp stdoutp fdopen getenv

// Builds that abort on panic, like release ones, leave std out: the library is
// preloaded into every process stdbuf runs. A no_std cdylib cannot unwind on
// stable Rust, so other builds keep std.
#![cfg_attr(panic = "abort", no_std)]

use core::ffi::{CStr, c_char, c_int};
use core::fmt::{self, Write};
use core::ptr;
use ctor::ctor;
use libc::{_IOFBF, _IOLBF, _IONBF, FILE, fileno, size_t};

#[cfg(panic = "abort")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    // SAFETY: abort has no preconditions.
    unsafe { libc::abort() }
}

// The prebuilt core refers to the unwinding personality routine that std
// provides. Panics abort, so it is never called, but linking without LTO can
// need the symbol.
#[cfg(panic = "abort")]
// SAFETY: std, the only other definition of this symbol, is left out.
#[unsafe(no_mangle)]
extern "C" fn rust_eh_personality() {}

// Hidden, so that the preloaded library cannot replace the routine of programs
// that export one, such as rustc. Mach-O and COFF bind imports per library.
#[cfg(all(
    panic = "abort",
    not(any(target_vendor = "apple", target_os = "cygwin"))
))]
core::arch::global_asm!(".hidden rust_eh_personality");

/// Writes to the stderr file descriptor directly, bypassing the C stream and
/// the buffering it may have been given.
struct Stderr;

impl Write for Stderr {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut bytes = s.as_bytes();
        while !bytes.is_empty() {
            // SAFETY: the pointer and length describe the valid buffer `bytes`.
            let n = unsafe { libc::write(libc::STDERR_FILENO, bytes.as_ptr().cast(), bytes.len()) };
            if n <= 0 {
                return Err(fmt::Error);
            }
            bytes = &bytes[n as usize..];
        }
        Ok(())
    }
}

// This runs automatically when the library is loaded via LD_PRELOAD
#[ctor(unsafe)]
fn init() {
    unsafe { __stdbuf() };
}

/// Returns the C library's stdin stream.
#[unsafe(no_mangle)]
pub extern "C" fn __stdbuf_get_stdin() -> *mut FILE {
    #[cfg(any(target_vendor = "apple", target_os = "freebsd"))]
    {
        unsafe extern "C" {
            fn __stdinp() -> *mut FILE;
        }
        unsafe { __stdinp() }
    }

    #[cfg(target_os = "openbsd")]
    {
        unsafe extern "C" {
            static mut __stdin: *mut FILE;
        }
        unsafe { __stdin }
    }

    #[cfg(target_os = "netbsd")]
    {
        unsafe { libc::fdopen(0, c"r".as_ptr()) }
    }

    #[cfg(target_os = "cygwin")]
    {
        // _getreent()->_std{in,out,err}
        // see:
        // echo '#include <stdio.h>\nstd{in,out,err}' | gcc -E -xc - -std=c23 | tail -n1
        // echo '#include <stdio.h>' | grep -E -xc - -std=c23 | grep 'struct _reent' -A91 | grep 580 -A91 | tail -n+2

        #[repr(C)]
        struct _reent {
            _errno: c_int,
            _stdin: *mut FILE,
            _stdout: *mut FILE,
            _stderr: *mut FILE,
            // other stuff
        }

        unsafe extern "C" {
            fn __getreent() -> *mut _reent;
        }

        unsafe { (*__getreent())._stdin }
    }

    #[cfg(not(any(
        target_vendor = "apple",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "cygwin"
    )))]
    {
        unsafe extern "C" {
            static mut stdin: *mut FILE;
        }
        unsafe { stdin }
    }
}

/// Returns the C library's stdout stream.
#[unsafe(no_mangle)]
pub extern "C" fn __stdbuf_get_stdout() -> *mut FILE {
    #[cfg(any(target_vendor = "apple", target_os = "freebsd"))]
    {
        unsafe extern "C" {
            fn __stdoutp() -> *mut FILE;
        }
        unsafe { __stdoutp() }
    }

    #[cfg(target_os = "openbsd")]
    {
        unsafe extern "C" {
            static mut __stdout: *mut FILE;
        }
        unsafe { __stdout }
    }

    #[cfg(target_os = "netbsd")]
    {
        unsafe { libc::fdopen(1, c"w".as_ptr()) }
    }

    #[cfg(target_os = "cygwin")]
    {
        // _getreent()->_std{in,out,err}
        // see:
        // echo '#include <stdio.h>\nstd{in,out,err}' | gcc -E -xc - -std=c23 | tail -n1
        // echo '#include <stdio.h>' | grep -E -xc - -std=c23 | grep 'struct _reent' -A91 | grep 580 -A91 | tail -n+2

        #[repr(C)]
        struct _reent {
            _errno: c_int,
            _stdin: *mut FILE,
            _stdout: *mut FILE,
            _stderr: *mut FILE,
            // other stuff
        }

        unsafe extern "C" {
            fn __getreent() -> *mut _reent;
        }

        unsafe { (*__getreent())._stdout }
    }

    #[cfg(not(any(
        target_vendor = "apple",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "cygwin"
    )))]
    {
        unsafe extern "C" {
            static mut stdout: *mut FILE;
        }
        unsafe { stdout }
    }
}

/// Returns the C library's stderr stream.
#[unsafe(no_mangle)]
pub extern "C" fn __stdbuf_get_stderr() -> *mut FILE {
    #[cfg(any(target_vendor = "apple", target_os = "freebsd"))]
    {
        unsafe extern "C" {
            fn __stderrp() -> *mut FILE;
        }
        unsafe { __stderrp() }
    }

    #[cfg(target_os = "openbsd")]
    {
        unsafe extern "C" {
            static mut __stderr: *mut FILE;
        }
        unsafe { __stderr }
    }

    #[cfg(target_os = "netbsd")]
    {
        unsafe { libc::fdopen(2, c"w".as_ptr()) }
    }

    #[cfg(target_os = "cygwin")]
    {
        // _getreent()->_std{in,out,err}
        // see:
        // echo '#include <stdio.h>\nstd{in,out,err}' | gcc -E -xc - -std=c23 | tail -n1
        // echo '#include <stdio.h>' | grep -E -xc - -std=c23 | grep 'struct _reent' -A91 | grep 580 -A91 | tail -n+2

        #[repr(C)]
        struct _reent {
            _errno: c_int,
            _stdin: *mut FILE,
            _stdout: *mut FILE,
            _stderr: *mut FILE,
            // other stuff
        }

        unsafe extern "C" {
            fn __getreent() -> *mut _reent;
        }

        unsafe { (*__getreent())._stdin }
    }

    #[cfg(not(any(
        target_vendor = "apple",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "cygwin"
    )))]
    {
        unsafe extern "C" {
            static mut stderr: *mut FILE;
        }
        unsafe { stderr }
    }
}

fn set_buffer(stream: *mut FILE, value: &str) {
    let (mode, size): (c_int, size_t) = match value {
        "0" => (_IONBF, 0_usize),
        "L" => (_IOLBF, 0_usize),
        input => {
            let Ok(buff_size) = input.parse::<usize>() else {
                let _ = writeln!(Stderr, "failed to allocate a {value} byte stdio buffer");
                // SAFETY: this runs from the library constructor, before main,
                // so no other thread calls exit at the same time.
                unsafe { libc::exit(1) };
            };
            (_IOFBF, buff_size as size_t)
        }
    };
    let res: c_int;
    unsafe {
        let buffer: *mut c_char = ptr::null_mut();
        assert!(buffer.is_null());
        res = libc::setvbuf(stream, buffer, mode, size);
    }
    if res != 0 {
        let _ = writeln!(
            Stderr,
            "could not set buffering of {} to mode {mode}",
            unsafe { fileno(stream) }
        );
    }
}

/// Applies the buffering mode in environment variable `name`, if set, to the
/// stream returned by `get_stream`.
fn set_buffer_from_env(name: &CStr, get_stream: extern "C" fn() -> *mut FILE) {
    // SAFETY: name is NUL-terminated, and nothing changes the environment
    // while this runs.
    let ptr = unsafe { libc::getenv(name.as_ptr()) };
    if ptr.is_null() {
        return;
    }
    // SAFETY: getenv returned a NUL-terminated string, which stays valid as
    // nothing changes the environment while this runs.
    let value = unsafe { CStr::from_ptr(ptr) };
    if let Ok(value) = value.to_str() {
        set_buffer(get_stream(), value);
    }
}

/// # Safety
/// This function is intended to be called automatically when the library is loaded via LD_PRELOAD.
/// It assumes that the standard streams are valid and that calling setvbuf on them is safe.
/// The caller must ensure this function is only called in a compatible runtime environment.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __stdbuf() {
    set_buffer_from_env(c"_STDBUF_E", __stdbuf_get_stderr);
    set_buffer_from_env(c"_STDBUF_I", __stdbuf_get_stdin);
    set_buffer_from_env(c"_STDBUF_O", __stdbuf_get_stdout);
}
