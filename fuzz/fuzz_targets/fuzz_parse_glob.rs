#![no_main]

use libfuzzer_sys::fuzz_target;
use uucore::parse_glob;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        _ = parse_glob::from_str(s);
    }
});
