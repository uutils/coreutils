// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) IOFBF IOLBF IONBF cstdio setvbuf

use cpp::cpp;
use libc::{c_char, c_int, fileno, size_t, FILE, _IOFBF, _IOLBF, _IONBF};
use std::env;
use std::ptr;

cpp! {{
    #include <cstdio>

    extern "C" {
        void __stdbuf(void);

        void __attribute((constructor))
        __stdbuf_init(void) {
            __stdbuf();
        }

        FILE *__stdbuf_get_stdin() { return stdin; }
        FILE *__stdbuf_get_stdout() { return stdout; }
        FILE *__stdbuf_get_stderr() { return stderr; }
    }
}}

extern "C" {
    fn __stdbuf_get_stdin() -> *mut FILE;
    fn __stdbuf_get_stdout() -> *mut FILE;
    fn __stdbuf_get_stderr() -> *mut FILE;
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
        eprintln!(
            "could not set buffering of {} to mode {}",
            unsafe { fileno(stream) },
            mode
        );
    }
}

/// # Safety
/// ToDO ... (safety note)
#[no_mangle]
pub unsafe extern "C" fn __stdbuf() {
    if let Ok(val) = env::var("_STDBUF_E") {
        set_buffer(__stdbuf_get_stderr(), &val);
    }
    if let Ok(val) = env::var("_STDBUF_I") {
        set_buffer(__stdbuf_get_stdin(), &val);
    }
    if let Ok(val) = env::var("_STDBUF_O") {
        set_buffer(__stdbuf_get_stdout(), &val);
    }
}
