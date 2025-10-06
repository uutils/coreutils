// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) IOFBF IOLBF IONBF setvbuf stderrp stdinp stdoutp

use ctor::ctor;
use libc::{_IOFBF, _IOLBF, _IONBF, FILE, c_char, c_int, fileno, size_t};
use std::env;
use std::ptr;

// This runs automatically when the library is loaded via LD_PRELOAD
#[ctor]
fn init() {
    unsafe { __stdbuf() };
}

/// # Safety
/// This function is unsafe because it calls a C API
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __stdbuf_get_stdin() -> *mut FILE {
    #[cfg(any(target_os = "macos", target_os = "freebsd"))]
    {
        unsafe extern "C" {
            fn __stdinp() -> *mut FILE;
        }
        unsafe { __stdinp() }
    }

    #[cfg(not(any(target_os = "macos", target_os = "freebsd")))]
    {
        unsafe extern "C" {
            static mut stdin: *mut FILE;
        }
        unsafe { stdin }
    }
}

/// # Safety
/// This function is unsafe because it calls a C API
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __stdbuf_get_stdout() -> *mut FILE {
    #[cfg(any(target_os = "macos", target_os = "freebsd"))]
    {
        unsafe extern "C" {
            fn __stdoutp() -> *mut FILE;
        }
        unsafe { __stdoutp() }
    }

    #[cfg(not(any(target_os = "macos", target_os = "freebsd")))]
    {
        unsafe extern "C" {
            static mut stdout: *mut FILE;
        }
        unsafe { stdout }
    }
}

/// # Safety
/// This function is unsafe because it calls a C API
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __stdbuf_get_stderr() -> *mut FILE {
    #[cfg(any(target_os = "macos", target_os = "freebsd"))]
    {
        unsafe extern "C" {
            fn __stderrp() -> *mut FILE;
        }
        unsafe { __stderrp() }
    }

    #[cfg(not(any(target_os = "macos", target_os = "freebsd")))]
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
            let buff_size: usize = match input.parse() {
                Ok(num) => num,
                Err(_) => {
                    eprintln!("failed to allocate a {value} byte stdio buffer");
                    std::process::exit(1);
                }
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
        eprintln!("could not set buffering of {} to mode {mode}", unsafe {
            fileno(stream)
        },);
    }
}

/// # Safety
/// This function is intended to be called automatically when the library is loaded via LD_PRELOAD.
/// It assumes that the standard streams are valid and that calling setvbuf on them is safe.
/// The caller must ensure this function is only called in a compatible runtime environment.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __stdbuf() {
    if let Ok(val) = env::var("_STDBUF_E") {
        set_buffer(unsafe { __stdbuf_get_stderr() }, &val);
    }
    if let Ok(val) = env::var("_STDBUF_I") {
        set_buffer(unsafe { __stdbuf_get_stdin() }, &val);
    }
    if let Ok(val) = env::var("_STDBUF_O") {
        set_buffer(unsafe { __stdbuf_get_stdout() }, &val);
    }
}
