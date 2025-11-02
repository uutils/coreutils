#![no_main]
use libfuzzer_sys::fuzz_target;

use std::ffi::OsString;
use uu_date::uumain;
use uufuzz::generate_and_run_uumain;

fuzz_target!(|data: &[u8]| {
    let delim: u8 = 0; // Null byte
    let fuzz_args: Vec<OsString> = data
        .split(|b| *b == delim)
        .filter_map(|e| std::str::from_utf8(e).ok())
        .map(OsString::from)
        .collect();

    // Skip test cases that would cause the program to read from stdin
    // These would hang the fuzzer waiting for input
    for i in 0..fuzz_args.len() {
        if let Some(arg) = fuzz_args.get(i) {
            let arg_str = arg.to_string_lossy();
            // Skip if -f- or --file=- (reads dates from stdin)
            if (arg_str == "-f"
                && fuzz_args
                    .get(i + 1)
                    .map(|a| a.to_string_lossy() == "-")
                    .unwrap_or(false))
                || arg_str == "-f-"
                || arg_str == "--file=-"
            {
                return;
            }
        }
    }

    // Add program name as first argument (required for proper argument parsing)
    let mut args = vec![OsString::from("date")];
    args.extend(fuzz_args);

    let _ = generate_and_run_uumain(&args, uumain, None);
});
