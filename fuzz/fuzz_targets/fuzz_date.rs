#![no_main]
use libfuzzer_sys::fuzz_target;

use std::ffi::OsString;
use uu_date::uumain;

fuzz_target!(|data: &[u8]| {
    let delim: u8 = 0; // Null byte
    let args = data
        .split(|b| *b == delim)
        .filter_map(|e| std::str::from_utf8(e).ok())
        .map(OsString::from);
    uumain(args);
});
