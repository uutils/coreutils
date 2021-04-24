//! stdio convenience fns

// spell-checker:ignore (ToDO) bslice

use std::env;
use std::io::{stderr, stdout, Write};

pub const EXIT_OK: i32 = 0;
pub const EXIT_ERR: i32 = 1;

pub fn err_msg(msg: &str) {
    let exe_path = match env::current_exe() {
        Ok(p) => p.to_string_lossy().into_owned(),
        _ => String::from(""),
    };
    writeln!(&mut stderr(), "{}: {}", exe_path, msg).unwrap();
}

// by default stdout only flushes
// to console when a newline is passed.
pub fn flush_char(c: char) {
    print!("{}", c);
    let _ = stdout().flush();
}
pub fn flush_str(s: &str) {
    print!("{}", s);
    let _ = stdout().flush();
}
pub fn flush_bytes(bslice: &[u8]) {
    let _ = stdout().write(bslice);
    let _ = stdout().flush();
}
