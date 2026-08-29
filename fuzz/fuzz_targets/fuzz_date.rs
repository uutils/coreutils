// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

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
            let arg_bytes = arg.as_encoded_bytes();
            // Skip if -f- or --file=- or combined options like -Rf- (reads dates from stdin)
            if (arg_bytes.first() == Some(&b'-')
                && !arg_bytes.starts_with(b"--")
                && arg_bytes.ends_with(b"f-"))
                || (arg_bytes == b"-f"
                    && fuzz_args
                        .get(i + 1)
                        .map(|a| a.as_encoded_bytes() == b"-")
                        .unwrap_or(false))
                || matches!(arg_bytes, b"-f-" | b"--file=-")
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
