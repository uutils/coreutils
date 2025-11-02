#![no_main]
use libfuzzer_sys::fuzz_target;

use std::ffi::OsString;
use uu_date::uumain;
use uufuzz::generate_and_run_uumain;

fuzz_target!(|data: &[u8]| {
    let delim: u8 = 0; // Null byte
    let args: Vec<OsString> = data
        .split(|b| *b == delim)
        .filter_map(|e| std::str::from_utf8(e).ok())
        .map(OsString::from)
        .collect();
    
    // Ensure we have at least a program name
    if args.is_empty() {
        return;
    }
    
    let date_main = |args: std::vec::IntoIter<OsString>| -> i32 {
        uumain(args)
    };
    
    let _ = generate_and_run_uumain(&args, date_main, None);
});
