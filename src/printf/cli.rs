//! stdio convenience fns
#[allow(unused_must_use)]

use std::io::{stderr, stdout, Write};
use std::env;

pub static EXIT_OK: i32 = 0;
pub static EXIT_ERR: i32 = 1;

pub fn err_msg(msg: &str) {
    let exe_path = match env::current_exe() {
        Ok(p) => p.to_string_lossy().into_owned(),
        _ => String::from(""),
    };
    writeln!(&mut stderr(), "{}: {}", exe_path, msg).unwrap();
}

// by default stdout only flushes
// to console when a newline is passed.
#[allow(unused_must_use)]
pub fn flush_char(c: &char) {
    print!("{}", c);
    stdout().flush();
}
#[allow(unused_must_use)]
pub fn flush_str(s: &str) {
    print!("{}", s);
    stdout().flush();
}
#[allow(unused_must_use)]
pub fn flush_bytes(bslice: &[u8]) {
    stdout().write(bslice);
    stdout().flush();
}
