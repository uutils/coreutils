#![crate_type = "dylib"]

extern crate libc;
use libc::{c_int, size_t, c_char, FILE, _IOFBF, _IONBF, _IOLBF, setvbuf};
use std::ptr;
use std::os;

extern {
    static stdin: *mut FILE;
    static stdout: *mut FILE;
    static stderr: *mut FILE;
}

fn set_buffer(stream: *mut FILE, value: &str) {
    let (mode, size): (c_int, size_t) = match value {
        "0" => (_IONBF, 0 as size_t),
        "L" => (_IOLBF, 0 as size_t),
        input => {
            let buff_size: uint = match from_str(input) {
                Some(num) => num,
                None => panic!("incorrect size of buffer!")
            };
            (_IOFBF, buff_size as size_t)
        }
    };
    let mut res: c_int;
    unsafe {
        let buffer: *mut c_char = ptr::null_mut();
        assert!(buffer.is_null());
        res = libc::setvbuf(stream, buffer, mode, size);
    }
    if res != 0 {
        panic!("error while calling setvbuf!");
    }
}

#[no_mangle]
pub extern fn stdbuf() {
    if let Some(val) = os::getenv("_STDBUF_E") {
        set_buffer(stderr, val.as_slice());
    }
    if let Some(val) = os::getenv("_STDBUF_I") {
        set_buffer(stdin, val.as_slice());
    }
    if let Some(val) = os::getenv("_STDBUF_O") {
        set_buffer(stdout, val.as_slice()); 
    }
}
