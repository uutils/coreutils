#![crate_name = "libstdbuf"]
#![crate_type = "staticlib"]

extern crate libc;

#[macro_use]
extern crate uucore;

use libc::{c_int, size_t, c_char, FILE, _IOFBF, _IONBF, _IOLBF, setvbuf};
use std::env;
use std::io::Write;
use std::ptr;

extern {
    static stdin: *mut FILE;
    static stdout: *mut FILE;
    static stderr: *mut FILE;
}

static NAME: &'static str = "libstdbuf";

fn set_buffer(stream: *mut FILE, value: &str) {
    let (mode, size): (c_int, size_t) = match value {
        "0" => (_IONBF, 0 as size_t),
        "L" => (_IOLBF, 0 as size_t),
        input => {
            let buff_size: usize = match input.parse() {
                Ok(num) => num,
                Err(e) => crash!(1, "incorrect size of buffer!: {}", e)
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
        crash!(res, "error while calling setvbuf!");
    }
}

#[no_mangle]
pub extern fn stdbuf() {
    if let Ok(val) = env::var("_STDBUF_E") {
        set_buffer(stderr, &val);
    }
    if let Ok(val) = env::var("_STDBUF_I") {
        set_buffer(stdin, &val);
    }
    if let Ok(val) = env::var("_STDBUF_O") {
        set_buffer(stdout, &val);
    }
}
